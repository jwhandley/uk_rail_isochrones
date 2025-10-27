use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use geojson::{Feature, FeatureCollection, ser::serialize_geometry};
use kiddo::{KdTree, SquaredEuclidean};
use serde::{Deserialize, Serialize};

use crate::adapters::CsaAdapter;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct StopId(u64);

impl StopId {
    pub fn new(idx: u64) -> Self {
        Self(idx)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TripId(u64);

impl TripId {
    pub fn new(idx: u64) -> Self {
        Self(idx)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrivalTime {
    pub stop_name: String,
    pub arrival_time: NaiveDateTime,
    #[serde(serialize_with = "serialize_geometry")]
    pub geometry: geo_types::Point<f64>,
}

const WALKING_SPEED_M_S: f64 = 1.4;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stop {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl Stop {
    pub fn new(name: String, lat: f64, lon: f64) -> Self {
        Self { name, lat, lon }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Connection {
    pub trip_id: TripId,
    pub from_stop_id: StopId,
    pub to_stop_id: StopId,
    pub departure_time: NaiveTime,
    pub arrival_time: NaiveTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transfer {
    pub from_stop_id: StopId,
    pub to_stop_id: StopId,
    pub transfer_time: TimeDelta,
}

impl Connection {
    fn departure_date_time(&self, date: NaiveDate) -> NaiveDateTime {
        NaiveDateTime::new(date, self.departure_time)
    }

    fn arrival_date_time(&self, date: NaiveDate) -> NaiveDateTime {
        if self.arrival_time < self.departure_time {
            NaiveDateTime::new(date + TimeDelta::days(1), self.arrival_time)
        } else {
            NaiveDateTime::new(date, self.arrival_time)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Calendar {
    services: HashMap<TripId, Vec<Service>>,
    cancellations: HashMap<TripId, Vec<Service>>,
}

impl Calendar {
    pub fn new(
        services: HashMap<TripId, Vec<Service>>,
        cancellations: HashMap<TripId, Vec<Service>>,
    ) -> Self {
        Self {
            services,
            cancellations,
        }
    }

    fn runs_on(&self, trip_id: TripId, date: NaiveDate) -> bool {
        let service_runs = self
            .services
            .get(&trip_id)
            .map(|services| services.iter().any(|s| s.runs_on(date)))
            .unwrap_or(false);

        let cancelled = self
            .cancellations
            .get(&trip_id)
            .map(|c| c.iter().any(|s| s.runs_on(date)))
            .unwrap_or(false);

        service_runs && !cancelled
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    start_date: NaiveDate,
    end_date: NaiveDate,
    runs_on: [bool; 7],
}

impl Service {
    pub fn new(start_date: NaiveDate, end_date: NaiveDate, runs_on: [bool; 7]) -> Self {
        Self {
            start_date,
            end_date,
            runs_on,
        }
    }

    fn runs_on(&self, date: NaiveDate) -> bool {
        let in_range = self.start_date <= date && self.end_date >= date;
        let valid_weekday = self.runs_on[date.weekday().days_since(chrono::Weekday::Mon) as usize];

        in_range && valid_weekday
    }
}

#[derive(Serialize, Deserialize)]
pub struct TransportNetwork {
    tree: kiddo::KdTree<f64, 3>,
    stops: HashMap<StopId, Stop>,
    connections: Vec<Connection>,
    transfers: HashMap<StopId, Vec<Transfer>>,
    calendar: Calendar,
}

impl TransportNetwork {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(postcard::from_bytes(&bytes)?)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let bytes = postcard::to_stdvec(self)?;
        std::fs::write(path, &bytes)?;
        Ok(())
    }

    pub fn from_adapter<A: CsaAdapter>(adapter: &A) -> Result<Self, A::Error> {
        let stops = adapter.stops()?;
        let mut connections = adapter.connections()?;
        connections.sort_unstable_by_key(|c| c.departure_time);

        let mut tree = KdTree::new();
        stops.iter().for_each(|(&id, s)| {
            tree.add(&to_unit(s.lat, s.lon), id.0);
        });

        let transfers = adapter.transfers()?;
        let calendar = adapter.calendar()?;

        Ok(Self {
            tree,
            stops,
            connections,
            transfers,
            calendar,
        })
    }

    pub fn query_lat_lon(
        &self,
        lat: f64,
        lon: f64,
        date: NaiveDate,
        departure_time: NaiveTime,
    ) -> Vec<ArrivalTime> {
        let departure_date_time = NaiveDateTime::new(date, departure_time);
        let mut csa = CsaState::new();

        for (stop_id, distance) in self.stops_within_radius(lat, lon, 500.0) {
            let time =
                departure_date_time + TimeDelta::seconds((distance / WALKING_SPEED_M_S) as i64);
            csa.update_arrival(stop_id, time);

            for transfer in self.get_transfers(stop_id) {
                if csa.should_update_arrival(transfer.to_stop_id, time + transfer.transfer_time) {
                    csa.update_arrival(transfer.to_stop_id, time + transfer.transfer_time);
                }
            }
        }

        for c in self.connections_after(departure_time) {
            if !self.calendar.runs_on(c.trip_id, date) {
                continue;
            }

            let already_boarded = csa.has_boarded(c.trip_id);
            let can_board = csa.can_board(c.from_stop_id, c.departure_date_time(date));

            if !already_boarded && !can_board {
                continue;
            }

            csa.board_trip(c.trip_id.clone());

            if csa.should_update_arrival(c.to_stop_id, c.arrival_date_time(date)) {
                csa.update_arrival(c.to_stop_id.clone(), c.arrival_date_time(date));

                for transfer in self.get_transfers(c.to_stop_id) {
                    let new_arrival = c.arrival_date_time(date) + transfer.transfer_time;
                    let earlier_arrival =
                        csa.should_update_arrival(transfer.to_stop_id, new_arrival);

                    if earlier_arrival {
                        csa.update_arrival(transfer.to_stop_id.clone(), new_arrival);
                    }
                }
            }
        }

        csa.arrival_times
            .iter()
            .map(|(&k, &v)| {
                let stop = self.stop(k);
                let arrival = v;

                let location = geo_types::Point::new(stop.lon, stop.lat);
                ArrivalTime {
                    stop_name: stop.name.clone(),
                    arrival_time: arrival,
                    geometry: location,
                }
            })
            .collect()
    }

    fn get_transfers(&self, stop: StopId) -> impl Iterator<Item = &Transfer> {
        match self.transfers.get(&stop) {
            Some(transfers) => transfers.iter(),
            None => [].iter(),
        }
    }

    fn connections_after(&self, departure_time: NaiveTime) -> impl Iterator<Item = &Connection> {
        let first_connection = self
            .connections
            .binary_search_by_key(&departure_time, |c| c.departure_time)
            .unwrap_or_else(|n| n);

        self.connections[first_connection..].iter()
    }

    fn stops_within_radius(
        &self,
        lat: f64,
        lon: f64,
        distance: f64,
    ) -> impl Iterator<Item = (StopId, f64)> {
        self.tree
            .within::<SquaredEuclidean>(&to_unit(lat, lon), meters_to_chord2(distance))
            .into_iter()
            .map(|x| (StopId(x.item), chord2_to_meters(x.distance)))
    }

    fn stop(&self, id: StopId) -> &Stop {
        &self.stops[&id]
    }
}

const R_EARTH_M: f64 = 6_371_008.8;

fn to_unit(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    let (clat, clon, slat, slon) = (lat.cos(), lon.cos(), lat.sin(), lon.sin());
    [clat * clon, clat * slon, slat]
}

#[inline]
fn chord2_to_meters(chord2: f64) -> f64 {
    let c = chord2.sqrt();
    let theta = 2.0 * (c / 2.0).asin();
    R_EARTH_M * theta
}

#[inline]
fn meters_to_chord2(d_m: f64) -> f64 {
    // numerically stable for small d
    let half = d_m / (2.0 * R_EARTH_M);
    4.0 * half.sin().powi(2)
}

pub fn to_feature_collection(arrival_times: &[ArrivalTime]) -> anyhow::Result<FeatureCollection> {
    let features = arrival_times
        .into_iter()
        .map(|t| geojson::ser::to_feature(t))
        .collect::<Result<Vec<Feature>, geojson::Error>>()?;

    Ok(FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    })
}

#[derive(Debug, Default)]
struct CsaState {
    arrival_times: HashMap<StopId, NaiveDateTime>,
    boarded_trips: HashSet<TripId>,
}

impl CsaState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update_arrival(&mut self, stop_id: StopId, time: NaiveDateTime) {
        self.arrival_times.insert(stop_id, time);
    }

    pub fn board_trip(&mut self, trip_id: TripId) {
        self.boarded_trips.insert(trip_id);
    }

    pub fn has_boarded(&self, trip_id: TripId) -> bool {
        self.boarded_trips.contains(&trip_id)
    }

    pub fn can_board(&self, stop_id: StopId, departure_time: NaiveDateTime) -> bool {
        self.arrival_times
            .get(&stop_id)
            .map(|&time| time <= departure_time)
            .unwrap_or(false)
    }

    pub fn should_update_arrival(&self, stop_id: StopId, new_arrival: NaiveDateTime) -> bool {
        self.arrival_times
            .get(&stop_id)
            .map(|&time| time > new_arrival)
            .unwrap_or(true)
    }
}
