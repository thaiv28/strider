//! FIT activity and course files as route points.
//!
//! The positions feed the same stats and downsampling as GPX (`strider_core::gpx::route_stats`),
//! so a FIT route and the equivalent GPX route report identical numbers.
use fitparser::{FitDataRecord, Value, profile::MesgNum};
use std::fmt;
use strider_core::gpx::{ParsedGpx, route_stats};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FitError {
    /// Not a readable FIT file (damaged, truncated or not FIT at all).
    Corrupt(String),
    /// A valid FIT file with fewer than two positions.
    NoPositions,
}
impl fmt::Display for FitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt(why) => write!(f, "Couldn't read that FIT file ({why})."),
            Self::NoPositions => f.write_str("That FIT file has no route positions."),
        }
    }
}
impl std::error::Error for FitError {}

/// A position in degrees and an optional elevation in metres.
pub type RoutePoint = (f64, f64, Option<f64>);

const SEMICIRCLES_TO_DEGREES: f64 = 180.0 / 2_147_483_648.0;

fn number(v: &Value) -> Option<f64> {
    Some(match v {
        Value::SInt8(n) => f64::from(*n),
        Value::UInt8(n) | Value::UInt8z(n) | Value::Byte(n) | Value::Enum(n) => f64::from(*n),
        Value::SInt16(n) => f64::from(*n),
        Value::UInt16(n) | Value::UInt16z(n) => f64::from(*n),
        Value::SInt32(n) => f64::from(*n),
        Value::UInt32(n) | Value::UInt32z(n) => f64::from(*n),
        Value::Float32(n) => f64::from(*n),
        Value::Float64(n) => *n,
        _ => return None,
    })
    .filter(|n| n.is_finite())
}
fn field(rec: &FitDataRecord, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| {
        rec.fields()
            .iter()
            .find(|f| f.name() == *name)
            .and_then(|f| number(f.value()))
    })
}
/// FIT marks an absent position with the largest signed 32-bit value.
fn degrees(semicircles: f64) -> Option<f64> {
    (semicircles != f64::from(i32::MAX)).then_some(semicircles * SEMICIRCLES_TO_DEGREES)
}
fn position(rec: &FitDataRecord, lat: &str, lon: &str, alt: &[&str]) -> Option<RoutePoint> {
    let (la, lo) = (degrees(field(rec, &[lat])?)?, degrees(field(rec, &[lon])?)?);
    if !(-90.0..=90.0).contains(&la) || !(-180.0..=180.0).contains(&lo) {
        return None;
    }
    Some((la, lo, field(rec, alt)))
}

/// The route points of a FIT file, in order.
///
/// `record` messages are used (latitude and longitude as semicircles converted to degrees,
/// `enhanced_altitude` or `altitude` in metres). Only a file without any positioned record falls
/// back to `course_point` positions, then to `lap` start and end positions.
pub fn route_points(bytes: &[u8]) -> Result<Vec<RoutePoint>, FitError> {
    let records = fitparser::from_bytes(bytes).map_err(|e| FitError::Corrupt(e.to_string()))?;
    if records.is_empty() {
        return Err(FitError::Corrupt("no data messages".into()));
    }
    let alt = ["enhanced_altitude", "altitude"];
    let from = |kind: MesgNum, f: &dyn Fn(&FitDataRecord) -> Vec<RoutePoint>| -> Vec<RoutePoint> {
        records
            .iter()
            .filter(|r| r.kind() == kind)
            .flat_map(f)
            .collect()
    };
    let mut points = from(MesgNum::Record, &|r| {
        position(r, "position_lat", "position_long", &alt)
            .into_iter()
            .collect()
    });
    if points.is_empty() {
        points = from(MesgNum::CoursePoint, &|r| {
            position(r, "position_lat", "position_long", &alt)
                .into_iter()
                .collect()
        });
    }
    if points.is_empty() {
        points = from(MesgNum::Lap, &|r| {
            [
                position(r, "start_position_lat", "start_position_long", &alt),
                position(r, "end_position_lat", "end_position_long", &alt),
            ]
            .into_iter()
            .flatten()
            .collect()
        });
    }
    if points.len() < 2 {
        return Err(FitError::NoPositions);
    }
    Ok(points)
}

/// Stats for a FIT file, computed exactly as for GPX.
pub fn parse_fit(bytes: &[u8]) -> Result<ParsedGpx, FitError> {
    route_stats(&route_points(bytes)?).ok_or(FitError::NoPositions)
}

/// The route as a GPX track, ready for the existing GPX attach path.
///
/// Positions keep 7 decimals (about a centimetre, finer than FIT's own resolution) and
/// elevations 2, so reading this back with `parse_gpx` gives the same stats as [`parse_fit`].
pub fn fit_to_gpx(bytes: &[u8]) -> Result<String, FitError> {
    let points = route_points(bytes)?;
    let trkpts: String = points
        .iter()
        .map(|&(lat, lon, ele)| {
            let ele = ele.map_or(String::new(), |e| format!("<ele>{e:.2}</ele>"));
            format!("<trkpt lat=\"{lat:.7}\" lon=\"{lon:.7}\">{ele}</trkpt>\n")
        })
        .collect();
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"Strider\" xmlns=\"http://www.topografix.com/GPX/1/1\"><trk><trkseg>\n{trkpts}</trkseg></trk></gpx>\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Field, encode_fit, fit_crc, fit_file, message};
    use strider_core::gpx::parse_gpx;

    fn route() -> Vec<(f64, f64, f64)> {
        (0..40)
            .map(|i| {
                let t = f64::from(i);
                (39.95 + t * 0.001, -105.3 + t * 0.001, 1800.0 + t * 12.0)
            })
            .collect()
    }

    #[test]
    fn records_become_degrees_and_metres() {
        let points = route_points(&encode_fit(&route())).expect("points");
        assert_eq!(points.len(), 40);
        let (lat, lon, ele) = points[3];
        assert!((lat - 39.953).abs() < 1e-6, "{lat}");
        assert!((lon + 105.297).abs() < 1e-6, "{lon}");
        assert!((ele.expect("elevation") - 1836.0).abs() < 0.25);
    }

    #[test]
    fn stats_match_the_gpx_algorithm_exactly() {
        let bytes = encode_fit(&route());
        let from_fit = parse_fit(&bytes).expect("fit");
        let gpx = fit_to_gpx(&bytes).expect("gpx");
        let from_gpx = parse_gpx(&gpx).expect("gpx parse");
        assert_eq!(from_fit.distance_mi, from_gpx.distance_mi);
        assert_eq!(from_fit.elevation_gain_ft, from_gpx.elevation_gain_ft);
        assert_eq!(from_fit.min_ele_ft, from_gpx.min_ele_ft);
        assert_eq!(from_fit.max_ele_ft, from_gpx.max_ele_ft);
        assert_eq!(from_fit.points.len(), from_gpx.points.len());
        assert!(from_fit.distance_mi > 3.0 && from_fit.elevation_gain_ft > 1500.0);
    }

    #[test]
    fn long_files_are_downsampled_like_gpx() {
        let long: Vec<_> = (0..2000)
            .map(|i| (39.0 + f64::from(i) * 1e-4, -105.0, 1500.0))
            .collect();
        let parsed = parse_fit(&encode_fit(&long)).expect("fit");
        assert!(parsed.points.len() <= 801);
        assert_eq!(parsed.points.first().map(|p| p.d), Some(0.0));
    }

    #[test]
    fn a_file_without_altitude_has_no_elevation() {
        let mut body = vec![];
        for lat in [39.0, 39.01] {
            let sc = (lat * 2_147_483_648.0_f64 / 180.0).round() as i32;
            let fields: Vec<Field> = vec![
                (0, 0x85, sc.to_le_bytes().to_vec()),
                (1, 0x85, sc.to_le_bytes().to_vec()),
            ];
            body.extend(message(20, 0, &fields));
        }
        let parsed = parse_fit(&fit_file(&body)).expect("fit");
        assert_eq!(parsed.min_ele_ft, None);
        assert!(parsed.distance_mi > 0.);
    }

    #[test]
    fn lap_positions_are_used_only_without_records() {
        let sc = |d: f64| ((d * 2_147_483_648.0_f64 / 180.0).round() as i32).to_le_bytes();
        let lap: Vec<Field> = vec![
            (3, 0x85, sc(39.0).to_vec()),
            (4, 0x85, sc(-105.0).to_vec()),
            (5, 0x85, sc(39.02).to_vec()),
            (6, 0x85, sc(-105.0).to_vec()),
        ];
        let parsed = parse_fit(&fit_file(&message(19, 0, &lap))).expect("lap fallback");
        assert!(parsed.distance_mi > 1.0);
        // With records present, laps are ignored.
        let mut body = message(19, 1, &lap);
        body.extend(encode_fit(&route()).drain(14..).rev().skip(2).rev());
        let n = route_points(&fit_file(&body)).expect("records").len();
        assert_eq!(n, 40);
    }

    #[test]
    fn corrupt_and_position_less_files_are_clear_errors() {
        let good = encode_fit(&route());
        for bad in [
            vec![],
            b"not a fit file at all".to_vec(),
            good[..good.len() / 2].to_vec(),
            {
                let mut b = good.clone();
                let mid = b.len() / 2;
                b[mid] ^= 0xFF;
                b
            },
            {
                let mut b = good.clone();
                let last = b.len() - 1;
                b[last] ^= 0xFF;
                b
            },
        ] {
            let err = route_points(&bad).expect_err("corrupt");
            assert!(matches!(err, FitError::Corrupt(_)), "{err:?}");
            assert!(err.to_string().starts_with("Couldn't read that FIT file"));
        }
        // Valid FIT, records without positions (timestamp only).
        let ts: Vec<Field> = vec![(253, 0x86, 1_000_000_000u32.to_le_bytes().to_vec())];
        let nothing = fit_file(&message(20, 0, &ts));
        assert_eq!(route_points(&nothing), Err(FitError::NoPositions));
        // A single position is not a route.
        let one = encode_fit(&route()[..1]);
        assert_eq!(parse_fit(&one), Err(FitError::NoPositions));
        assert_eq!(fit_crc(&[]), 0);
    }
}
