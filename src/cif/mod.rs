use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime};
use zip::ZipArchive;

use crate::cif::{alf::ALF, mca::MCA, msn::MSN};
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
    pub detail: MCA,
    pub stations: MSN,
    pub links: ALF,
}

impl CifTimetable {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut msn_opt = None::<MSN>;
        let mut mca_opt = None::<MCA>;
        let mut alf_opt = None::<ALF>;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.name().to_lowercase().ends_with(".msn") {
                msn_opt = Some(MSN::from_reader(file)?);
            } else if file.name().to_lowercase().ends_with(".mca") {
                mca_opt = Some(MCA::from_reader(file)?);
            } else if file.name().to_lowercase().ends_with(".alf") {
                alf_opt = Some(ALF::from_reader(file)?);
            }
        }

        let msn = msn_opt.context("Failed to load MSN file")?;
        let mca = mca_opt.context("Failed to load MCA file")?;
        let alf = alf_opt.context("Failed to load ALF file")?;

        Ok(Self {
            detail: mca,
            stations: msn,
            links: alf,
        })
    }
}
