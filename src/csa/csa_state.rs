use crate::csa::{StopId, TripId};
use chrono::NaiveDateTime;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct CsaState {
    pub arrival_times: HashMap<StopId, NaiveDateTime>,
    pub boarded_trips: HashSet<TripId>,
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
