use axum::{
    Json, Router,
    extract::{Query, State},
    http::{Method, StatusCode},
    routing::get,
};
use chrono::{NaiveDate, NaiveTime};
use clap::{Parser, Subcommand};
use geojson::FeatureCollection;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::cors::CorsLayer;
use tracing::info;

mod adapters;
mod cif;
mod csa;
use crate::{
    cif::CifTimetable,
    csa::{TransportNetwork, to_feature_collection},
};

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
    Serve {
        network_path: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let args = Cli::parse();

    match args.command {
        Commands::Import {
            timetable_path,
            network_path,
        } => {
            import_timetable(timetable_path, network_path).expect("Unable to import CIF timetable");
        }
        Commands::Query {
            network_path,
            lat,
            lon,
            date,
            time,
        } => {
            let network = TransportNetwork::load(network_path).expect("Failed to load network");
            let geojson =
                run_query(&network, lat, lon, date, time).expect("Failed to execute query");
            println!("{}", geojson.to_string());
        }
        Commands::Serve { network_path } => {
            info!("Loading network from file");
            let network = TransportNetwork::load(network_path).expect("Failed to load network");
            let network = Arc::from(network);

            let app = Router::new()
                .route("/isochrone", get(isochrone))
                .layer(
                    CorsLayer::new()
                        .allow_origin([
                            "http://localhost:5173".parse().unwrap(),
                            "https://jwhandley.github.io/rail-isochrone-viewer"
                                .parse()
                                .unwrap(),
                        ])
                        .allow_methods([Method::GET]),
                )
                .with_state(network);

            let listener = tokio::net::TcpListener::bind("0.0.0.0:10000")
                .await
                .unwrap();

            info!("listening on {}", listener.local_addr().unwrap());

            axum::serve(listener, app).await.unwrap()
        }
    }
}

fn import_timetable(
    timetable_path: impl AsRef<Path>,
    network_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    info!("Reading timetable");
    let timetable = CifTimetable::read(timetable_path)?;
    info!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    info!("Adapting to transport network");
    let network = TransportNetwork::try_from(&timetable)?;
    info!("Done in {:?}", now.elapsed());

    let now = std::time::Instant::now();
    info!("Saving network");
    network.save(network_path)?;
    info!("Done in {:?}", now.elapsed());

    Ok(())
}

#[derive(Deserialize)]
struct IsochroneParams {
    lat: f64,
    lon: f64,
    date: NaiveDate,
    time: NaiveTime,
}

async fn isochrone(
    Query(params): Query<IsochroneParams>,
    State(network): State<Arc<TransportNetwork>>,
) -> Result<Json<FeatureCollection>, StatusCode> {
    let IsochroneParams {
        lat,
        lon,
        date,
        time,
    } = params;

    let now = std::time::Instant::now();
    info!("Querying network for arrival times starting from ({lat}, {lon}) on {date} at {time}");
    let arrival_times = network.query_lat_lon(lat, lon, date, time);
    let features = to_feature_collection(&arrival_times);
    info!("Done in {:?}", now.elapsed());

    features
        .map(|f| Json(f))
        .map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)
}

fn run_query(
    network: &TransportNetwork,
    lat: f64,
    lon: f64,
    date: NaiveDate,
    time: NaiveTime,
) -> anyhow::Result<FeatureCollection> {
    let now = std::time::Instant::now();
    info!("Querying network for arrival times starting from ({lat}, {lon}) on {date} at {time}");
    let arrival_times = network.query_lat_lon(lat, lon, date, time);
    info!("Done in {:?}", now.elapsed());
    to_feature_collection(&arrival_times)
}
