use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};

use crate::csa::{
    StopId, TripId,
    adapters::CsaAdapter,
    csa_state::CsaState,
    stop_collection::{Stop, StopCollection},
};

const WALKING_SPEED_M_S: f64 = 1.4;

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
    date: NaiveDate,
}

impl TransportNetwork {
    pub fn from_adapter<A: CsaAdapter>(adapter: &A) -> Result<Self, A::Error> {
        let stops = adapter.stops()?;
        let mut connections = adapter.connections()?;
        connections.sort_unstable_by_key(|c| c.departure_time); // single canonical sort

        // build a StopCollection (assign StopId by index)
        let stops = StopCollection::from(stops); // your existing type

        let transfers = adapter.transfers()?;

        Ok(Self {
            stops,
            connections,
            transfers,
            date: adapter.date(),
        })
    }

    pub fn query_lat_lon(
        &self,
        lat: f64,
        lon: f64,
        departure_time: NaiveTime,
    ) -> HashMap<Stop, NaiveDateTime> {
        let mut csa = CsaState::new();
        let departure_time = NaiveDateTime::new(self.date, departure_time);
        for (stop_id, distance) in self.stops.stops_within_radius(lat, lon, 500.0) {
            let time = departure_time + TimeDelta::seconds((distance / WALKING_SPEED_M_S) as i64);
            csa.update_arrival(stop_id, time);

            for transfer in self.get_transfers(stop_id) {
                if csa.should_update_arrival(transfer.to_stop_id, time + transfer.transfer_time) {
                    csa.update_arrival(transfer.to_stop_id, time + transfer.transfer_time);
                }
            }
        }

        let first_connection = self
            .connections
            .binary_search_by_key(&departure_time, |c| c.departure_time)
            .unwrap_or_else(|n| n);

        for c in self.connections[first_connection..].iter() {
            let already_boarded = csa.has_boarded(c.trip_id);
            let can_board = csa.can_board(c.from_stop_id, c.departure_time);

            if !already_boarded && !can_board {
                continue;
            }

            csa.board_trip(c.trip_id.clone());

            if csa.should_update_arrival(c.to_stop_id, c.arrival_time) {
                csa.update_arrival(c.to_stop_id.clone(), c.arrival_time);

                for transfer in self.get_transfers(c.to_stop_id) {
                    let new_arrival = c.arrival_time + transfer.transfer_time;
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
            .map(|(k, &v)| (self.stops[*k].clone(), v))
            .collect()
    }

    fn get_transfers(&self, stop: StopId) -> impl Iterator<Item = &Transfer> {
        match self.transfers.get(&stop) {
            Some(transfers) => transfers.iter(),
            None => [].iter(),
        }
    }
}
