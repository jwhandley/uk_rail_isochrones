mod adapters;
mod cif;
mod csa;
use crate::{
    adapters::cif::{CifAdapter, StationInfo},
    cif::CifTimetable,
    csa::{query_lat_lon, to_feature_collection, transport_network::TransportNetwork},
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Query {
        lat: f64,
        #[arg(allow_hyphen_values = true)]
        lon: f64,
        time: NaiveTime,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

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

    let Commands::Query { lat, lon, time } = args.command;
    let departure_time = NaiveDateTime::new(date, time);
    let arrival_times = query_lat_lon(&network, lat, lon, departure_time);
    let geojson = to_feature_collection(&arrival_times)?;
    println!("{}", geojson.to_string());

    Ok(())
}
