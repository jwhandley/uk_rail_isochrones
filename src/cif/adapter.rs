use anyhow::Result;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashMap;

use crate::{
    adapters::CsaAdapter,
    cif::CifTimetable,
    csa::{Calendar, Connection, Service, Stop, StopId, Transfer, TripId},
};

#[derive(Deserialize, Clone)]
pub struct StationInfo {
    #[serde(rename = "stationName")]
    name: String,
    #[serde(rename = "crsCode")]
    pub crs: String,
    lat: f64,
    #[serde(rename = "long")]
    lon: f64,
}

pub struct CifAdapter<'a> {
    timetable: &'a CifTimetable,
    schedule_to_trip_id: HashMap<String, TripId>,
    crs_to_stop_id: HashMap<String, StopId>,
    tiploc_to_stop_id: HashMap<String, StopId>,
    stops: HashMap<StopId, Stop>,
}

impl<'a> CifAdapter<'a> {
    pub fn new(timetable: &'a CifTimetable) -> Result<Self> {
        let station_str = include_str!("../../uk-railway-stations/stations.json");
        let station_info: Vec<StationInfo> = serde_json::from_str(station_str)?;
        let station_info: HashMap<String, StationInfo> = station_info
            .into_iter()
            .map(|s| (s.crs.clone(), s))
            .collect();

        let mut crs_to_stop_id = HashMap::new();
        let mut tiploc_to_stop_id = HashMap::new();
        let mut stops = HashMap::new();
        let mut schedule_to_trip_id = HashMap::new();

        for (i, s) in timetable
            .stations
            .iter()
            .filter(|s| station_info.contains_key(&s.crs))
            .enumerate()
        {
            let id = StopId::new(i as u64);
            let crs = s.crs.clone();
            let tiploc = s.tiploc.clone();

            let info = &station_info[&crs];
            let name = info.name.clone();
            let lat = info.lat;
            let lon = info.lon;

            let stop = Stop::new(id, name, lat, lon);

            stops.insert(id, stop);
            tiploc_to_stop_id.insert(tiploc, id.clone());
            crs_to_stop_id.insert(crs, id);
        }

        for (i, schedule) in timetable.schedules.iter().enumerate() {
            let trip_id = TripId::new(i as u64);
            schedule_to_trip_id.insert(schedule.id.clone(), trip_id);
        }

        Ok(Self {
            timetable,
            schedule_to_trip_id,
            crs_to_stop_id,
            tiploc_to_stop_id,
            stops,
        })
    }
}

impl<'a> CsaAdapter for CifAdapter<'a> {
    type Error = anyhow::Error;

    fn stops(&self) -> Result<HashMap<StopId, Stop>> {
        Ok(self.stops.clone())
    }

    fn calendar(&self) -> Result<Calendar> {
        let mut services: HashMap<TripId, Vec<Service>> = HashMap::new();
        let mut cancellations: HashMap<TripId, Vec<Service>> = HashMap::new();

        for schedule in self.timetable.schedules.iter() {
            let trip_id = self.schedule_to_trip_id[&schedule.id];
            let service = Service::new(schedule.start_date, schedule.end_date, schedule.days_run);
            match schedule.trip_type {
                crate::cif::mca::ScheduleType::Cancellation => {
                    cancellations.entry(trip_id).or_default().push(service)
                }
                _ => {
                    services.entry(trip_id).or_default().push(service);
                }
            }
        }

        Ok(Calendar::new(services, cancellations))
    }

    fn connections(&self) -> Result<Vec<Connection>> {
        // trip ID can be created from schedule ID
        // stop ID must be converted from tiplocs
        // Will need a map from tiploc to stop ID,
        // which can be made by combining the stops step (crs to StopID)
        // with the tiploc_to_crs map in the timetable
        let mut connections = vec![];

        for schedule in self.timetable.schedules.iter() {
            let trip_id = self.schedule_to_trip_id[&schedule.id];

            let locations: Vec<_> = schedule
                .locations
                .iter()
                .filter(|loc| self.tiploc_to_stop_id.contains_key(&loc.id()))
                .collect();

            for locs in locations.windows(2) {
                let from = &locs[0];
                let to = &locs[1];

                let from_id = self.tiploc_to_stop_id[&from.id()];
                let to_id = self.tiploc_to_stop_id[&to.id()];

                let departure_time = from
                    .departure_time()
                    .expect("Should only be origin or intermediate");
                let arrival_time = to
                    .arrival_time()
                    .expect("Should only be intermediate or destination");

                let connection = Connection {
                    trip_id,
                    from_stop_id: from_id,
                    to_stop_id: to_id,
                    departure_time,
                    arrival_time,
                };
                connections.push(connection);
            }
        }

        Ok(connections)
    }

    fn transfers(&self) -> Result<HashMap<StopId, Vec<Transfer>>, Self::Error> {
        // links contain origin and destination CRS, which can use the map from CRS to Stop ID
        // They also contain a transfer time in minutes which can just be reused
        let transfers = self
            .timetable
            .links
            .iter()
            .filter(|link| {
                self.crs_to_stop_id.contains_key(&link.origin_crs)
                    && self.crs_to_stop_id.contains_key(&link.dest_crs)
            })
            .map(|link| {
                let from_stop_id = self.crs_to_stop_id[&link.origin_crs];
                let to_stop_id = self.crs_to_stop_id[&link.dest_crs];
                let time = link.time;
                Transfer {
                    from_stop_id,
                    to_stop_id,
                    transfer_time: time,
                }
            })
            .into_group_map_by(|t| t.from_stop_id);

        Ok(transfers)
    }
}
