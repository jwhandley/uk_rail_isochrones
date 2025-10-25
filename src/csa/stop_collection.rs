use kiddo::{SquaredEuclidean, float::kdtree};
use std::{collections::HashMap, hash::Hash, ops::Index};

use crate::csa::StopId;

#[derive(Clone, Debug)]
pub struct Stop {
    pub id: StopId,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl Stop {
    pub fn new(id: StopId, name: String, lat: f64, lon: f64) -> Self {
        Self { id, name, lat, lon }
    }
}

impl PartialEq for Stop {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Stop {}

impl Hash for Stop {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct StopCollection {
    tree: kdtree::KdTree<f64, usize, 3, 32, u32>,
    stops: HashMap<StopId, Stop>,
}

impl From<HashMap<StopId, Stop>> for StopCollection {
    fn from(stops: HashMap<StopId, Stop>) -> Self {
        let mut tree: kdtree::KdTree<f64, usize, 3, 32, u32> = kdtree::KdTree::new();
        stops.iter().for_each(|(&id, s)| {
            tree.add(&to_unit(s.lat, s.lon), id.0);
        });

        Self { tree, stops }
    }
}

impl Index<StopId> for StopCollection {
    type Output = Stop;

    fn index(&self, index: StopId) -> &Self::Output {
        &self.stops[&index]
    }
}

impl StopCollection {
    pub fn stops_within_radius(
        &self,
        lat: f64,
        lon: f64,
        distance: f64,
    ) -> impl Iterator<Item = (StopId, f64)> {
        self.tree
            .within::<SquaredEuclidean>(&to_unit(lat, lon), meters_to_chord2(distance))
            .into_iter()
            .map(|x| (StopId(x.item), chord2_to_meters(x.distance)))
    }
}

const R_EARTH_M: f64 = 6_371_008.8;

fn to_unit(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    let (clat, clon, slat, slon) = (lat.cos(), lon.cos(), lat.sin(), lon.sin());
    [clat * clon, clat * slon, slat]
}

#[inline]
fn chord2_to_meters(chord2: f64) -> f64 {
    let c = chord2.sqrt();
    let theta = 2.0 * (c / 2.0).asin();
    R_EARTH_M * theta
}

#[inline]
fn meters_to_chord2(d_m: f64) -> f64 {
    // numerically stable for small d
    let half = d_m / (2.0 * R_EARTH_M);
    4.0 * half.sin().powi(2)
}
