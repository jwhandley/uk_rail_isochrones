use chrono::{NaiveDateTime, NaiveTime, TimeDelta};
use geojson::ser::serialize_geometry;
use serde::Serialize;

use crate::csa::transport_network::TransportNetwork;
pub mod adapters;
mod csa_state;
mod stop_collection;
pub mod transport_network;

const WALKING_SPEED_M_S: f64 = 1.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct StopId(usize);

impl StopId {
    pub fn new(idx: usize) -> Self {
        Self(idx)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TripId(usize);

impl TripId {
    pub fn new(idx: usize) -> Self {
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

pub fn query_lat_lon(
    network: &TransportNetwork,
    lat: f64,
    lon: f64,
    departure_time: NaiveTime,
) -> Vec<ArrivalTime> {
    let mut csa = csa_state::CsaState::new();
    let departure_time = NaiveDateTime::new(network.date, departure_time);
    for (stop_id, distance) in network.stops_within_radius(lat, lon, 500.0) {
        let time = departure_time + TimeDelta::seconds((distance / WALKING_SPEED_M_S) as i64);
        csa.update_arrival(stop_id, time);

        for transfer in network.get_transfers(stop_id) {
            if csa.should_update_arrival(transfer.to_stop_id, time + transfer.transfer_time) {
                csa.update_arrival(transfer.to_stop_id, time + transfer.transfer_time);
            }
        }
    }

    for c in network.connections_after(departure_time) {
        let already_boarded = csa.has_boarded(c.trip_id);
        let can_board = csa.can_board(c.from_stop_id, c.departure_time);

        if !already_boarded && !can_board {
            continue;
        }

        csa.board_trip(c.trip_id.clone());

        if csa.should_update_arrival(c.to_stop_id, c.arrival_time) {
            csa.update_arrival(c.to_stop_id.clone(), c.arrival_time);

            for transfer in network.get_transfers(c.to_stop_id) {
                let new_arrival = c.arrival_time + transfer.transfer_time;
                let earlier_arrival = csa.should_update_arrival(transfer.to_stop_id, new_arrival);

                if earlier_arrival {
                    csa.update_arrival(transfer.to_stop_id.clone(), new_arrival);
                }
            }
        }
    }

    csa.arrival_times
        .iter()
        .map(|(&k, &v)| {
            let stop = &network.stop(k);
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
