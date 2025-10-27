use crate::csa::{Connection, Stop, StopId, Transfer};
use chrono::NaiveDate;
use std::collections::HashMap;

pub trait CsaAdapter {
    type Error;

    /// Returns a stable, deduplicated list of stops (in CSA order).
    fn stops(&self) -> Result<HashMap<StopId, Stop>, Self::Error>;

    /// Returns all connections (any order); the builder will sort by departure.
    fn connections(&self, date: NaiveDate) -> Result<Vec<Connection>, Self::Error>;

    /// Returns footpath/transfer graph.
    fn transfers(&self) -> Result<HashMap<StopId, Vec<Transfer>>, Self::Error>;
}
