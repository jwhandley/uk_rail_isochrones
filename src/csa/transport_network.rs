use crate::csa::{StopId, TripId, adapters::CsaAdapter};
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use kiddo::{SquaredEuclidean, float::kdtree};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Stop {
    #[allow(unused)]
    pub id: StopId,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl Stop {
    pub fn new(id: StopId, name: String, lat: f64, lon: f64) -> Self {
        Self { id, name, lat, lon }
    }
}

#[derive(Debug)]
pub struct Connection {
    pub trip_id: TripId,
    pub from_stop_id: StopId,
    pub to_stop_id: StopId,
    pub departure_time: NaiveDateTime,
    pub arrival_time: NaiveDateTime,
}

pub struct Transfer {
    pub from_stop_id: StopId,
    pub to_stop_id: StopId,
    pub transfer_time: TimeDelta,
}

pub struct TransportNetwork {
    tree: kdtree::KdTree<f64, usize, 3, 32, u32>,
    stops: HashMap<StopId, Stop>,
    connections: Vec<Connection>,
    transfers: HashMap<StopId, Vec<Transfer>>,
    pub date: NaiveDate,
}

impl TransportNetwork {
    pub fn from_adapter<A: CsaAdapter>(adapter: &A) -> Result<Self, A::Error> {
        let stops = adapter.stops()?;
        let mut connections = adapter.connections()?;
        connections.sort_unstable_by_key(|c| c.departure_time);

        let mut tree: kdtree::KdTree<f64, usize, 3, 32, u32> = kdtree::KdTree::new();
        stops.iter().for_each(|(&id, s)| {
            tree.add(&to_unit(s.lat, s.lon), id.0);
        });
        let transfers = adapter.transfers()?;

        Ok(Self {
            tree,
            stops,
            connections,
            transfers,
            date: adapter.date(),
        })
    }

    pub fn get_transfers(&self, stop: StopId) -> impl Iterator<Item = &Transfer> {
        match self.transfers.get(&stop) {
            Some(transfers) => transfers.iter(),
            None => [].iter(),
        }
    }

    pub fn connections_after(
        &self,
        departure_time: NaiveDateTime,
    ) -> impl Iterator<Item = &Connection> {
        let first_connection = self
            .connections
            .binary_search_by_key(&departure_time, |c| c.departure_time)
            .unwrap_or_else(|n| n);

        self.connections[first_connection..].iter()
    }

    pub fn stops_within_radius(
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

    pub fn stop(&self, id: StopId) -> &Stop {
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
