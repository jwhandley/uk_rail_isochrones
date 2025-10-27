pub mod cif;

use std::collections::HashMap;

use crate::csa::{
    StopId,
    transport_network::{Connection, Stop, Transfer},
};

pub trait CsaAdapter {
    type Error;

    /// Returns a stable, deduplicated list of stops (in CSA order).
    fn stops(&self) -> Result<HashMap<StopId, Stop>, Self::Error>;

    /// Returns all connections (any order); the builder will sort by departure.
    fn connections(&self) -> Result<Vec<Connection>, Self::Error>;

    /// Returns footpath/transfer graph.
    fn transfers(&self) -> Result<HashMap<StopId, Vec<Transfer>>, Self::Error>;
}
