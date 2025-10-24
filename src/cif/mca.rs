use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};

use crate::cif::{parse_date_ddmmyy, parse_hhmm};

pub struct MCA {
    pub schedules: Vec<Schedule>,
}

impl MCA {
    pub fn from_reader<R: Read>(r: R) -> Result<Self> {
        let reader = BufReader::new(r);
        parse_mca(reader)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        parse_mca(reader)
    }
}

fn parse_mca<R: BufRead>(reader: R) -> Result<MCA> {
    let mut schedules = vec![];

    let mut parsing_trip = false;
    for line in reader.lines() {
        let line: String = line?;

        match &line[0..2] {
            "BS" => {
                let trip_id = line[3..9].to_owned();
                let start_date = parse_date_ddmmyy(&line[9..15])?;
                let end_date = parse_date_ddmmyy(&line[15..21])?;
                let trip_type = match line.chars().last() {
                    Some('P') => ScheduleType::Permanent,
                    Some('O') => ScheduleType::Overlay,
                    Some('N') => ScheduleType::New,
                    Some('C') => ScheduleType::Cancellation,
                    _ => anyhow::bail!("Unexpected character at end of line: {line}"),
                };

                let mut days_run = [false; 7];
                line[21..28].char_indices().for_each(|(i, d)| {
                    days_run[i] = d == '1';
                });

                parsing_trip = true;
                schedules.push(Schedule::new(
                    trip_id, start_date, end_date, trip_type, days_run,
                ));
            }
            "LI" if !valid_activities(&line[42..54]) => continue,
            "LO" | "LI" | "LD" if parsing_trip => {
                let loc = Location::from_str(&line)?;

                if loc.is_dest() {
                    parsing_trip = false;
                }

                if let Some(trip) = schedules.last_mut() {
                    trip.add_location(loc);
                }
            }
            _ => continue,
        }
    }

    Ok(MCA { schedules })
}

fn valid_activities(s: &str) -> bool {
    s.contains("T ") || s.contains("D ") || s.contains("U ")
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum ScheduleType {
    Permanent,
    New,
    Overlay,
    Cancellation,
}

#[derive(Debug)]
pub struct Schedule {
    pub id: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub trip_type: ScheduleType,
    pub days_run: [bool; 7],
    pub locations: Vec<Location>,
}

impl Schedule {
    pub fn new(
        id: String,
        start_date: NaiveDate,
        end_date: NaiveDate,
        trip_type: ScheduleType,
        days_run: [bool; 7],
    ) -> Self {
        Self {
            id,
            start_date,
            end_date,
            trip_type,
            days_run,
            locations: vec![],
        }
    }

    pub fn add_location(&mut self, loc: Location) {
        self.locations.push(loc);
    }
}

#[derive(Debug)]
pub enum Location {
    Origin {
        tiploc: String,
        departure_time: NaiveTime,
    },
    Intermediate {
        tiploc: String,
        arrival_time: NaiveTime,
        departure_time: NaiveTime,
    },
    Destination {
        tiploc: String,
        arrival_time: NaiveTime,
    },
}

impl FromStr for Location {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            s if s.starts_with("LO") => {
                let tiploc = s[2..9].trim().to_string();
                let departure_time = parse_hhmm(&s[15..19])?;

                Ok(Location::Origin {
                    tiploc,
                    departure_time,
                })
            }
            s if s.starts_with("LI") => {
                let activities = &s[42..54];
                if !(activities.contains("T ")
                    || activities.contains("D ")
                    || activities.contains("U "))
                {
                    anyhow::bail!("Location does not pick up passengers")
                }

                let tiploc = s[2..9].trim().to_string();

                let mut arrival_time = parse_hhmm(&s[25..29])?;
                let mut departure_time = parse_hhmm(&s[29..33])?;

                // If no public time, use scheduled one
                if &s[25..29] == "0000" {
                    arrival_time = parse_hhmm(&s[10..14])?;
                }

                // If no public time, use scheduled one
                if &s[29..33] == "0000" {
                    departure_time = parse_hhmm(&s[15..19])?;
                }

                Ok(Location::Intermediate {
                    tiploc,
                    arrival_time,
                    departure_time,
                })
            }
            s if s.starts_with("LT") => {
                let tiploc = s[2..9].trim().to_string();
                let arrival_time = parse_hhmm(&s[15..19])?;

                Ok(Location::Destination {
                    tiploc,
                    arrival_time,
                })
            }
            _ => anyhow::bail!("Invalid location record"),
        }
    }
}

impl Location {
    pub fn id(&self) -> String {
        match self {
            Location::Origin { tiploc, .. } => tiploc.clone(),
            Location::Intermediate { tiploc, .. } => tiploc.clone(),
            Location::Destination { tiploc, .. } => tiploc.clone(),
        }
    }

    pub fn departure_time(&self) -> Option<NaiveTime> {
        match self {
            Location::Origin { departure_time, .. } => Some(*departure_time),
            Location::Intermediate { departure_time, .. } => Some(*departure_time),
            Location::Destination { .. } => None,
        }
    }

    pub fn arrival_time(&self) -> Option<NaiveTime> {
        match self {
            Location::Origin { .. } => None,
            Location::Intermediate { arrival_time, .. } => Some(*arrival_time),
            Location::Destination { arrival_time, .. } => Some(*arrival_time),
        }
    }

    pub fn is_dest(&self) -> bool {
        match self {
            Location::Origin { .. } => false,
            Location::Intermediate { .. } => false,
            Location::Destination { .. } => true,
        }
    }

    pub fn is_orign(&self) -> bool {
        match self {
            Location::Origin { .. } => true,
            Location::Intermediate { .. } => false,
            Location::Destination { .. } => false,
        }
    }
}
