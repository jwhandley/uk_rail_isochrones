use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime, TimeDelta};

#[allow(unused)]
pub struct Msn {
    pub header: Header,
    pub stations: Vec<Station>,
    pub aliases: Vec<Alias>,
}

impl Msn {
    /// Parse from any reader (file, in-memory, decompressed stream, zip entry, etc.)
    pub fn from_reader<R: Read>(r: R) -> Result<Self> {
        let reader = BufReader::new(r);
        parse_msn(reader)
    }

    /// Convenience wrapper for plain files.
    #[allow(unused)]
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path).with_context(|| format!("opening {:?}", path.as_ref()))?;
        Self::from_reader(file)
    }
}

fn parse_msn<R: BufRead>(reader: R) -> Result<Msn> {
    let mut header = None;
    let mut stations = Vec::new();
    let aliases = Vec::new();

    let mut parsed_header = false;
    for line in reader.lines() {
        let line = line?;
        if line.starts_with('/') {
            continue;
        }

        if !parsed_header {
            header = Some(Header::from_str(&line)?);
            parsed_header = true;
        } else if line.starts_with('A') {
            stations.push(Station::from_str(&line)?);
        } else if line.starts_with('L') {
            // aliases.push(Alias::from_str(&line)?);
        }
    }

    Ok(Msn {
        header: header.context("no header record found")?,
        stations,
        aliases,
    })
}

#[allow(unused)]
#[derive(Debug)]
pub struct Header {
    pub version: f64,
    pub creation_date: NaiveDate,
    pub creation_time: NaiveTime,
    pub sequence_number: u32,
}

impl FromStr for Header {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version: f64 = s[43..47]
            .parse()
            .with_context(|| format!("Invalid version number in {s}"))?;
        let creation_date = NaiveDate::parse_from_str(&s[48..56], "%d/%m/%y")
            .with_context(|| format!("Failed to parse date in {s}"))?;
        let creation_time = NaiveTime::parse_from_str(s[57..66].trim(), "%H.%M.%S")
            .with_context(|| format!("Failed to parse time in {}", &s[57..66]))?;
        let sequence_number: u32 = s[66..71]
            .trim()
            .parse()
            .with_context(|| format!("Invalid sequence number in {s}"))?;

        Ok(Header {
            version,
            creation_date,
            creation_time,
            sequence_number,
        })
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct Station {
    pub station_name: String,
    pub interchange_status: u8,
    pub tiploc: String,
    pub minor_crs: String,
    pub crs: String,
    pub easting: u32,
    pub northing: u32,
    pub min_change_time: TimeDelta,
}

impl FromStr for Station {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let station_name = s[5..31].trim().to_string();
        let interchange_status: u8 = s[35..36]
            .trim()
            .parse()
            .with_context(|| format!("Invalid interchange status in {s}"))?;

        let tiploc = s[36..43].trim().to_string();
        let minor_crs = s[43..46].to_string();
        let crs = s[49..52].to_string();
        let easting: u32 = s[55..57]
            .parse()
            .with_context(|| format!("Invalid easting in {s}"))?;

        let northing: u32 = s[58..63]
            .parse()
            .with_context(|| format!("Invalid northing in {s}"))?;

        let min_change_time: i64 = s[63..65]
            .trim()
            .parse()
            .with_context(|| format!("Invalid change time in {s}"))?;

        Ok(Station {
            station_name,
            interchange_status,
            tiploc,
            minor_crs,
            crs,
            easting,
            northing,
            min_change_time: TimeDelta::minutes(min_change_time),
        })
    }
}

#[allow(unused)]
pub struct Alias {
    pub station_name: String,
    pub station_alias: String,
}

impl FromStr for Alias {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let station_name = s[5..31].to_string();
        let station_alias = s[36..61].to_string();

        Ok(Alias {
            station_name,
            station_alias,
        })
    }
}
