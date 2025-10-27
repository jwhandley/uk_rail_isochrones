mod adapters;
mod cif;
mod csa;
use std::path::{Path, PathBuf};

use crate::{
    cif::CifTimetable,
    csa::{TransportNetwork, to_feature_collection},
};
use chrono::{NaiveDate, NaiveTime};
use clap::{Parser, Subcommand};
use geojson::FeatureCollection;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Import {
        timetable_path: PathBuf,
        #[arg(default_value = "./network.pc")]
        network_path: PathBuf,
    },
    Query {
        network_path: PathBuf,
        #[arg(allow_hyphen_values = true)]
        lat: f64,
        #[arg(allow_hyphen_values = true)]
        lon: f64,
        date: NaiveDate,
        time: NaiveTime,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Import {
            timetable_path,
            network_path,
        } => import_timetable(timetable_path, network_path),
        Commands::Query {
            network_path,
            lat,
            lon,
            date,
            time,
        } => {
            let geojson = run_query(network_path, lat, lon, date, time)?;
            println!("{}", geojson.to_string());
            Ok(())
        }
    }
}

fn import_timetable(
    timetable_path: impl AsRef<Path>,
    network_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    eprintln!("Reading timetable");
    let timetable = CifTimetable::read(timetable_path)?;
    eprintln!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    eprintln!("Adapting to transport network");
    let network = TransportNetwork::try_from(&timetable)?;
    eprintln!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    eprintln!("Saving network");
    network.save(network_path)?;
    eprintln!("Done in {:?}", now.elapsed());

    Ok(())
}

fn run_query(
    network_path: impl AsRef<Path>,
    lat: f64,
    lon: f64,
    date: NaiveDate,
    time: NaiveTime,
) -> anyhow::Result<FeatureCollection> {
    let now = std::time::Instant::now();
    eprintln!("Loading network");
    let network = TransportNetwork::load(network_path)?;
    eprintln!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    eprintln!(
        "Querying network for arrival times starting from ({lat}, {lon}) on {date} at {time}"
    );
    let arrival_times = network.query_lat_lon(lat, lon, date, time);
    eprintln!("Done in {:?}", now.elapsed());
    to_feature_collection(&arrival_times)
}
