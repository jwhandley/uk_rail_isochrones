mod cif;
mod csa;
use chrono::{NaiveDate, NaiveTime};
use itertools::Itertools;

use crate::{
    cif::CifTimetable,
    csa::{
        adapters::cif::{CifAdapter, StationInfo},
        transport_network::TransportNetwork,
    },
};

fn main() -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    println!("Reading timetable");
    let timetable = CifTimetable::read("../timetable-2025-10-24.zip")?;
    println!("Done in {:?}", now.elapsed());

    println!("Loading station info");
    let station_str = include_str!("../uk-railway-stations/stations.json");
    let station_info: Vec<StationInfo> = serde_json::from_str(station_str)?;
    let station_info = station_info
        .into_iter()
        .map(|s| (s.crs.clone(), s))
        .collect();
    println!("Done");

    let now = std::time::Instant::now();
    println!("Adapting to transport network");
    let date = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap();
    let adapter = CifAdapter::new(&timetable, date, station_info);

    let network = TransportNetwork::from_adapter(&adapter)?;
    println!("Done in {:?}", now.elapsed());

    let departure_time = NaiveTime::from_hms_opt(8, 30, 0).unwrap();
    let lat = 51.237;
    let lon = -0.58;

    println!(
        "Querying accessible stops from Guildford Station ({lat}, {lon}) leaving at {departure_time}"
    );
    let now = std::time::Instant::now();
    let arrival_times = network.query_lat_lon(lat, lon, departure_time);
    println!("Done in {:?}", now.elapsed());

    arrival_times
        .iter()
        .sorted_by_key(|(_, time)| *time)
        .for_each(|(stop, time)| {
            println!("Arrive at {stop:?} by {time}");
        });

    Ok(())
}
