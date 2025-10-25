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
