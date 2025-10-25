use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, NaiveTime, TimeDelta};
use std::{
    io::{BufRead, BufReader, Read},
    str::FromStr,
};

pub fn parse_alf<R: Read>(reader: R) -> Result<Vec<Link>> {
    let reader = BufReader::new(reader);
    let mut links = vec![];
    for line in reader.lines() {
        let input = line?;
        let link = parse_link(&input)?;
        links.push(link);
    }

    Ok(links)
}

#[allow(unused)]
#[derive(Debug, Default)]
pub struct Link {
    mode: Mode,
    pub origin_crs: String,
    pub dest_crs: String,
    pub time: TimeDelta,
    start_time: NaiveTime,
    end_time: NaiveTime,
    priority: u8,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    days_of_week: Option<[bool; 7]>,
}

#[derive(Debug, Default)]
pub enum Mode {
    Bus,
    Tube,
    #[default]
    Walk,
    Ferry,
    Metro,
    Tram,
    Taxi,
    Transfer,
}

impl FromStr for Mode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "BUS" => Mode::Bus,
            "TUBE" => Mode::Tube,
            "WALK" => Mode::Walk,
            "FERRY" => Mode::Ferry,
            "METRO" => Mode::Metro,
            "TRAM" => Mode::Tram,
            "TAXI" => Mode::Taxi,
            "TRANSFER" => Mode::Transfer,
            _ => bail!("unexpected value for mode: {s}"),
        })
    }
}

#[derive(Default, Debug)]
struct Acc {
    mode: Option<Mode>,
    origin: Option<String>,
    destination: Option<String>,
    time: Option<TimeDelta>,
    start_time: Option<NaiveTime>,
    end_time: Option<NaiveTime>,
    priority: Option<u8>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    days_of_week: Option<[bool; 7]>,
}

impl Acc {
    fn finish(self) -> Option<Link> {
        Some(Link {
            mode: self.mode?,
            origin_crs: self.origin?,
            dest_crs: self.destination?,
            time: self.time?,
            start_time: self.start_time?,
            end_time: self.end_time?,
            priority: self.priority?,
            start_date: self.start_date,
            end_date: self.end_date,
            days_of_week: self.days_of_week,
        })
    }
}

fn parse_hhmm(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H%M").with_context(|| format!("invalid time (HHMM): {s}"))
}

fn parse_ddmmyyyy(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%d/%m/%Y")
        .with_context(|| format!("invalid date (DD/MM/YYYY): {s}"))
}

fn parse_days(s: &str) -> [bool; 7] {
    let mut days = [false; 7];
    for (i, b) in s.as_bytes().iter().take(7).enumerate() {
        days[i] = *b == b'1';
    }
    days
}

pub fn parse_link(input: &str) -> Result<Link> {
    let mut acc = Acc::default();

    for (k, v) in input.split(',').filter_map(|s| s.split_once('=')) {
        match k {
            "M" => acc.mode = Some(v.parse()?),
            "O" => acc.origin = Some(v.trim().to_owned()),
            "D" => acc.destination = Some(v.trim().to_owned()),
            "T" => {
                acc.time = Some(TimeDelta::minutes(
                    v.parse::<i64>()
                        .with_context(|| format!("invalid time minutes: {v}"))?,
                ))
            }
            "S" => acc.start_time = Some(parse_hhmm(v)?),
            "E" => acc.end_time = Some(parse_hhmm(v)?),
            "P" => {
                acc.priority = Some(
                    v.parse::<u8>()
                        .with_context(|| format!("invalid priority: {v}"))?,
                )
            }
            "F" => acc.start_date = Some(parse_ddmmyyyy(v)?),
            "U" => acc.end_date = Some(parse_ddmmyyyy(v)?),
            "R" => acc.days_of_week = Some(parse_days(v)),
            _ => bail!("unexpected key in link: {k}"),
        }
    }

    acc.finish().context("failed to create link")
}
