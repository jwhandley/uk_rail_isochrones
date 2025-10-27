mod adapters;
mod cif;
mod csa;
use crate::{
    cif::CifTimetable,
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
        date: NaiveDate,
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
    let network = TransportNetwork::try_from(&timetable)?;
    eprintln!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    eprintln!("Saving network");
    network.save("./network.json")?;
    eprintln!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    eprintln!("Loading network");
    _ = TransportNetwork::load("./network.json")?;
    eprintln!("Done in {:?}", now.elapsed());

    let Commands::Query {
        lat,
        lon,
        date,
        time,
    } = args.command;

    let arrival_times = network.query_lat_lon(lat, lon, date, time);
    let geojson = to_feature_collection(&arrival_times)?;
    println!("{}", geojson.to_string());

    Ok(())
}
