mod adapters;
mod cif;
mod csa;
use std::path::PathBuf;

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
    Import {
        timetable_path: PathBuf,
        #[arg(default_value = "./network.pc")]
        network_path: PathBuf,
    },
    Query {
        network_path: PathBuf,
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
        } => {
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
        }
        Commands::Query {
            network_path,
            lat,
            lon,
            date,
            time,
        } => {
            let now = std::time::Instant::now();
            eprintln!("Loading network");
            let network = TransportNetwork::load(network_path)?;
            eprintln!("Done in {:?}", now.elapsed());

            let now = std::time::Instant::now();
            eprintln!(
                "Querying network for arrival times starting from ({lat}, {lon}) on {date} at {time}"
            );
            let arrival_times = network.query_lat_lon(lat, lon, date, time);
            let geojson = to_feature_collection(&arrival_times)?;
            println!("{}", geojson.to_string());
            eprintln!("Done in {:?}", now.elapsed());
        }
    }

    Ok(())
}
