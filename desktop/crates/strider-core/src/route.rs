#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoutePt {
    pub d: f64,
    pub lat: f64,
    pub lon: f64,
    pub e: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Campsite {
    pub distance_mi: f64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DaySegment {
    pub day: usize,
    pub distance_mi: f64,
    pub elevation_gain_ft: i64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RouteStats {
    pub distance_mi: Option<f64>,
    pub elevation_ft: Option<i64>,
}
pub fn effective_route_stats(
    trip: RouteStats,
    gpx: Option<RouteStats>,
    trail: Option<RouteStats>,
) -> RouteStats {
    let (gpx, trail) = (gpx.unwrap_or_default(), trail.unwrap_or_default());
    RouteStats {
        distance_mi: trip.distance_mi.or(gpx.distance_mi).or(trail.distance_mi),
        elevation_ft: trip
            .elevation_ft
            .or(gpx.elevation_ft)
            .or(trail.elevation_ft),
    }
}
pub fn compute_day_segments(pts: &[RoutePt], campsites: &[Campsite]) -> Vec<DaySegment> {
    let Some(last) = pts.last() else {
        return vec![];
    };
    if campsites.is_empty() {
        return vec![];
    }
    let bounds: Vec<f64> = std::iter::once(0.)
        .chain(campsites.iter().map(|c| c.distance_mi))
        .chain(std::iter::once(last.d))
        .collect();
    (0..=campsites.len())
        .map(|k| {
            let mut gain = 0.;
            let mut reference: Option<f64> = None;
            for p in pts
                .iter()
                .filter(|p| p.d >= bounds[k] && p.d <= bounds[k + 1])
            {
                if let Some(r) = reference {
                    if (p.e - r).abs() >= 13. {
                        if p.e > r {
                            gain += p.e - r;
                        }
                        reference = Some(p.e);
                    }
                } else {
                    reference = Some(p.e);
                }
            }
            let (lat, lon) = campsites
                .get(k)
                .map_or((Some(last.lat), Some(last.lon)), |c| (c.lat, c.lon));
            // Number(toFixed(2)); formatting preserves the binary-float decimal ties.
            let distance = super::format::fixed(bounds[k + 1] - bounds[k], 2)
                .parse()
                .unwrap_or(0.);
            DaySegment {
                day: k + 1,
                distance_mi: distance,
                elevation_gain_ft: crate::format::js_round(gain) as i64,
                lat,
                lon,
            }
        })
        .collect()
}
