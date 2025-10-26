mod cif;
mod csa;
use chrono::{NaiveDate, NaiveTime};

use crate::{
    cif::CifTimetable,
    csa::{
        adapters::cif::{CifAdapter, StationInfo},
        transport_network::TransportNetwork,
    },
};

fn main() -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    eprintln!("Reading timetable");
    let timetable = CifTimetable::read("../timetable-2025-10-24.zip")?;
    eprintln!("Done in {:?}", now.elapsed());

    eprintln!("Loading station info");
    let station_str = include_str!("../uk-railway-stations/stations.json");
    let station_info: Vec<StationInfo> = serde_json::from_str(station_str)?;
    let station_info = station_info
        .into_iter()
        .map(|s| (s.crs.clone(), s))
        .collect();
    eprintln!("Done");

    let now = std::time::Instant::now();
    eprintln!("Adapting to transport network");
    let date = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap();
    let adapter = CifAdapter::new(&timetable, date, station_info);

    let network = TransportNetwork::from_adapter(&adapter)?;
    eprintln!("Done in {:?}", now.elapsed());

    let departure_time = NaiveTime::from_hms_opt(8, 30, 0).unwrap();
    let lat = 51.237;
    let lon = -0.58;

    eprintln!(
        "Querying accessible stops from Guildford Station ({lat}, {lon}) leaving at {departure_time}"
    );
    let now = std::time::Instant::now();
    let arrival_times = network.query_lat_lon(lat, lon, departure_time);
    eprintln!("Done in {:?}", now.elapsed());
    let geojson = geojson::ser::to_feature_collection_string(&arrival_times)?;
    println!("{geojson}");

    Ok(())
}
