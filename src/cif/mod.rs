use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, NaiveTime};
use std::{fs::File, path::Path};
use zip::ZipArchive;

use crate::cif::{alf::Alf, mca::Mca, msn::Msn};

pub mod alf;
pub mod mca;
pub mod msn;

pub fn parse_hhmm(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H%M").with_context(|| format!("bad time (HHMM): {s}"))
}

pub fn parse_date_ddmmyy(s: &str) -> Result<NaiveDate> {
    let day: u32 = s[0..2].parse().expect("Invalid day");
    let mon: u32 = s[2..4].parse().expect("Invalid month");
    let yy: i32 = s[4..6].parse().expect("Invalid year");
    let year = 2000 + yy;
    NaiveDate::from_ymd_opt(year, mon, day).ok_or_else(|| anyhow::anyhow!("invalid ddmmyy {s}"))
}

pub struct CifTimetable {
    pub mca: Mca,
    pub msn: Msn,
    pub alf: Alf,
}

impl CifTimetable {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut msn: Option<Result<Msn>> = None;
        let mut mca: Option<Result<Mca>> = None;
        let mut alf: Option<Result<Alf>> = None;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_ascii_lowercase();

            if name.ends_with(".msn") {
                msn = Some(Msn::from_reader(file));
            } else if name.ends_with(".mca") {
                mca = Some(Mca::from_reader(file));
            } else if name.ends_with(".alf") {
                alf = Some(Alf::from_reader(file));
            }
        }

        let msn = msn.transpose()?.context("Missing MSN file")?;
        let mca = mca.transpose()?.context("Missing MCA file")?;
        let alf = alf.transpose()?.context("Missing ALF file")?;

        Ok(Self { mca, msn, alf })
    }
}
