mod cif;
use crate::cif::CifTimetable;

fn main() -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    println!("Reading timetable");
    let timetable = CifTimetable::read("../timetable-2025-10-24.zip")?;
    println!("Done in {:?}", now.elapsed());

    let msn = timetable.stations;
    println!("Header: {:?}", msn.header);
    println!(
        "Read {} stations and {} aliases",
        msn.stations.len(),
        msn.aliases.len()
    );

    let alf = timetable.links;
    println!("Read {} links", alf.links.len());

    let mca = timetable.detail;
    println!("Read {} schedules from timetable", mca.schedules.len());

    Ok(())
}
