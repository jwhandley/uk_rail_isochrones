mod cif;
use std::collections::{HashMap, HashSet};

use crate::cif::CifTimetable;

fn main() -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    println!("Reading timetable");
    let timetable = CifTimetable::read("../timetable-2025-10-24.zip")?;
    println!("Done in {:?}", now.elapsed());

    let stations = timetable.stations;
    println!("Read {} stations", stations.len());

    let links = timetable.links;
    println!("Read {} links", links.len());

    let schedules = &timetable.schedules;
    println!("Read {} schedules", schedules.len());

    let mut stop_to_schedules: HashMap<String, HashSet<String>> = HashMap::new();

    for schedule in timetable.schedules.iter() {
        for loc in schedule.locations.iter() {
            stop_to_schedules
                .entry(loc.id())
                .or_default()
                .insert(schedule.id.clone());
        }
    }

    Ok(())
}
