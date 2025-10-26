use crate::csa::{
    StopId, TripId,
    adapters::CsaAdapter,
    stop_collection::{Stop, StopCollection},
};
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use std::collections::HashMap;

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
    stops: StopCollection,
    connections: Vec<Connection>,
    transfers: HashMap<StopId, Vec<Transfer>>,
    pub date: NaiveDate,
}

impl TransportNetwork {
    pub fn from_adapter<A: CsaAdapter>(adapter: &A) -> Result<Self, A::Error> {
        let stops = adapter.stops()?;
        let mut connections = adapter.connections()?;
        connections.sort_unstable_by_key(|c| c.departure_time);

        let stops = StopCollection::from(stops);

        let transfers = adapter.transfers()?;

        Ok(Self {
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
        self.stops.stops_within_radius(lat, lon, distance)
    }

    pub fn stop(&self, id: StopId) -> &Stop {
        &self.stops[&id]
    }
}
