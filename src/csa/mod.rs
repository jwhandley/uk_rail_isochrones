use chrono::NaiveDateTime;
use geojson::ser::serialize_geometry;
use serde::Serialize;
pub mod adapters;
mod csa_state;
mod stop_collection;
pub mod transport_network;

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
