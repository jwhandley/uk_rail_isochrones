mod adapters;
mod cif;
mod csa;
use crate::{
    cif::{CifTimetable, adapter::CifAdapter},
    csa::{TransportNetwork, to_feature_collection},
};
use chrono::{NaiveDate, NaiveTime};
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

    let now = std::time::Instant::now();
    eprintln!("Adapting to transport network");
    let adapter = CifAdapter::new(&timetable);
    let date = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap();
    let network = TransportNetwork::from_adapter(&adapter, date)?;
    eprintln!("Done in {:?}", now.elapsed());

    let Commands::Query { lat, lon, time } = args.command;

    let arrival_times = network.query_lat_lon(lat, lon, time);
    let geojson = to_feature_collection(&arrival_times)?;
    println!("{}", geojson.to_string());

    Ok(())
}
