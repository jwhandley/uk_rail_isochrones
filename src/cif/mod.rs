use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime};
use std::{fs::File, path::Path};
use zip::ZipArchive;

pub mod adapter;
mod alf;
mod mca;
mod msn;

use alf::{Link, parse_alf};
use mca::{Schedule, parse_mca};
use msn::{Msn, Station};

use crate::cif::adapter::CifAdapter;

pub fn parse_hhmm(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H%M").with_context(|| format!("bad time (HHMM): {s}"))
}

pub fn parse_date_ddmmyy(s: &str) -> Result<NaiveDate> {
    let day: u32 = s[0..2].parse().context("Invalid day")?;
    let mon: u32 = s[2..4].parse().context("Invalid month")?;
    let yy: i32 = s[4..6].parse().context("Invalid year")?;
    let year = 2000 + yy;
    NaiveDate::from_ymd_opt(year, mon, day).with_context(|| format!("invalid ddmmyy {s}"))
}

pub struct CifTimetable {
    pub schedules: Vec<Schedule>,
    pub stations: Vec<Station>,
    pub links: Vec<Link>,
}

impl CifTimetable {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut msn: Option<Result<Msn>> = None;
        let mut schedules: Option<Result<Vec<Schedule>>> = None;
        let mut links: Option<Result<Vec<Link>>> = None;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_ascii_lowercase();

            if name.ends_with(".msn") {
                msn = Some(Msn::from_reader(file));
            } else if name.ends_with(".mca") {
                schedules = Some(parse_mca(file));
            } else if name.ends_with(".alf") {
                links = Some(parse_alf(file));
            }
        }

        let msn = msn.transpose()?.context("Missing MSN file")?;
        let schedules = schedules.transpose()?.context("Missing MCA file")?;

        let alf = links.transpose()?.context("Missing ALF file")?;

        Ok(Self {
            schedules,
            stations: msn.stations,
            links: alf,
        })
    }
}

impl<'a> From<&'a CifTimetable> for CifAdapter<'a> {
    fn from(value: &'a CifTimetable) -> Self {
        CifAdapter::new(value)
    }
}
