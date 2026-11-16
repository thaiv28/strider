//! GPX handoff rules, including the web parser's deliberately permissive regexes.
use crate::format::{fixed, js_round};
use regex::Regex;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub d: f64,
    pub lat: f64,
    pub lon: f64,
    pub e: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfilePoint {
    pub d: f64,
    pub e: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedGpx {
    pub start_lat: f64,
    pub start_lon: f64,
    pub distance_mi: f64,
    pub elevation_gain_ft: f64,
    pub min_ele_ft: Option<f64>,
    pub max_ele_ft: Option<f64>,
    pub track: Vec<[f64; 2]>,
    pub profile: Vec<ProfilePoint>,
    pub points: Vec<Point>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportPoint {
    pub lat: f64,
    pub lon: f64,
    pub e: f64,
}
impl From<&Point> for ExportPoint {
    fn from(p: &Point) -> Self {
        Self {
            lat: p.lat,
            lon: p.lon,
            e: p.e,
        }
    }
}
pub trait Location {
    fn location(&self) -> (Option<f64>, Option<f64>);
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCampsite {
    pub night: i64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub ele_ft: Option<f64>,
}
impl Location for ExportCampsite {
    fn location(&self) -> (Option<f64>, Option<f64>) {
        (self.lat, self.lon)
    }
}
pub fn group_by_location<T: Location>(items: &[T]) -> Vec<Vec<&T>> {
    let mut groups: Vec<Vec<&T>> = vec![];
    for it in items {
        let (lat, lon) = it.location();
        let found = groups.iter().position(|g| { let (a,b)=g[0].location();
   matches!((a,b,lat,lon),(Some(a),Some(b),Some(c),Some(d)) if (a-c).abs()<3e-4 && (b-d).abs()<3e-4)
  });
        if let Some(i) = found {
            groups[i].push(it)
        } else {
            groups.push(vec![it])
        }
    }
    groups
}
pub fn is_loop_track(track: &[[f64; 2]]) -> bool {
    track.len() >= 2
        && (track[0][0] - track[track.len() - 1][0]).abs() < 3e-4
        && (track[0][1] - track[track.len() - 1][1]).abs() < 3e-4
}
fn rounded(x: f64, d: u32) -> f64 {
    fixed(x, d).parse().unwrap_or(f64::NAN)
}
pub fn parse_gpx(xml: &str) -> Option<ParsedGpx> {
    let tag = if xml.contains("<trkpt") {
        "trkpt"
    } else {
        "rtept"
    };
    let re = Regex::new(&format!(
        r"<{tag}(?-u:\b)[^>]*>([\s\S]*?)</{tag}>|<{tag}(?-u:\b)[^>]*/>"
    ))
    .ok()?;
    let lat_re = Regex::new(r#"(?-u:\b)lat="([-0-9.]+)""#).ok()?;
    let lon_re = Regex::new(r#"(?-u:\b)lon="([-0-9.]+)""#).ok()?;
    let ele_re = Regex::new(r"<ele>([-0-9.]+)</ele>").ok()?;
    let mut pts = vec![];
    for m in re.captures_iter(xml) {
        let whole = m.get(0)?.as_str();
        let open = &whole[..=whole.find('>')?];
        let lat: f64 = lat_re
            .captures(open)
            .and_then(|c| c[1].parse().ok())
            .unwrap_or(f64::NAN);
        let lon: f64 = lon_re
            .captures(open)
            .and_then(|c| c[1].parse().ok())
            .unwrap_or(f64::NAN);
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        let ele = m
            .get(1)
            .and_then(|s| ele_re.captures(s.as_str()))
            .map(|c| c[1].parse::<f64>().unwrap_or(f64::NAN));
        pts.push((lat, lon, ele));
    }
    if pts.len() < 2 {
        return None;
    }
    let (mut dist, mut gain, mut reference, mut last_ele) = (0., 0., None::<f64>, None::<f64>);
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut full = vec![];
    for (i, &(lat, lon, ele)) in pts.iter().enumerate() {
        if i > 0 {
            let (a, b, _) = pts[i - 1];
            let dl = (lat - a) * std::f64::consts::PI / 180.;
            let dn = (lon - b) * std::f64::consts::PI / 180.;
            let la1 = a * std::f64::consts::PI / 180.;
            let la2 = lat * std::f64::consts::PI / 180.;
            let h = (dl / 2.).sin().powi(2) + la1.cos() * la2.cos() * (dn / 2.).sin().powi(2);
            dist += 2. * 6371000. * h.sqrt().asin();
        }
        let ele = if ele.is_none() {
            last_ele
        } else {
            last_ele = ele;
            ele
        };
        if let Some(e) = ele {
            min = if min.is_nan() || e.is_nan() {
                f64::NAN
            } else {
                min.min(e)
            };
            max = if max.is_nan() || e.is_nan() {
                f64::NAN
            } else {
                max.max(e)
            };
            if let Some(r) = reference {
                if (e - r).abs() >= 4. {
                    if e > r {
                        gain += e - r
                    }
                    reference = Some(e)
                }
            } else {
                reference = Some(e)
            }
        }
        full.push(Point {
            d: rounded(dist * (1. / 1609.344), 3),
            lat,
            lon,
            e: ele.map_or(0., |e| js_round(e * 3.28084)),
        });
    }
    let step = full.len().div_ceil(800);
    let mut points: Vec<_> = full.iter().step_by(step).cloned().collect();
    if !(full.len() - 1).is_multiple_of(step) {
        points.push(full.last()?.clone())
    }
    Some(ParsedGpx {
        start_lat: pts[0].0,
        start_lon: pts[0].1,
        distance_mi: rounded(dist * (1. / 1609.344), 2),
        elevation_gain_ft: js_round(gain * 3.28084),
        min_ele_ft: if min == f64::INFINITY {
            None
        } else {
            Some(js_round(min * 3.28084))
        },
        max_ele_ft: if max == f64::NEG_INFINITY {
            None
        } else {
            Some(js_round(max * 3.28084))
        },
        track: points.iter().map(|p| [p.lat, p.lon]).collect(),
        profile: points
            .iter()
            .map(|p| ProfilePoint { d: p.d, e: p.e })
            .collect(),
        points,
    })
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn gpx_filename(name: &str) -> String {
    let mut base = String::new();
    let mut dash = false;
    for c in name
        .trim_matches(|c: char| c == '\u{feff}' || (c.is_whitespace() && c != '\u{85}'))
        .chars()
    {
        if c.is_ascii_alphanumeric() || "_.-".contains(c) {
            base.push(c);
            dash = false
        } else if !dash {
            base.push('-');
            dash = true
        }
    }
    let base = base.trim_matches('-');
    format!("{}.gpx", if base.is_empty() { "route" } else { base })
}
fn wpt(lat: f64, lon: f64, name: &str, sym: &str, ele: Option<f64>) -> String {
    let ele = ele
        .map(|e| format!("<ele>{}</ele>", fixed(e * (1. / 3.28084), 1)))
        .unwrap_or_default();
    format!(
        "  <wpt lat=\"{}\" lon=\"{}\">{ele}<name>{}</name><sym>{sym}</sym></wpt>",
        fixed(lat, 6),
        fixed(lon, 6),
        esc(name)
    )
}
pub fn build_gpx(name: &str, points: &[ExportPoint], campsites: &[ExportCampsite]) -> String {
    let mut wpts = vec![];
    if let Some(first) = points.first() {
        let track: Vec<_> = points.iter().map(|p| [p.lat, p.lon]).collect();
        if is_loop_track(&track) {
            wpts.push(wpt(
                first.lat,
                first.lon,
                "Start / End",
                "Trail Head",
                Some(first.e),
            ))
        } else {
            wpts.push(wpt(
                first.lat,
                first.lon,
                "Start",
                "Trail Head",
                Some(first.e),
            ));
            if let Some(last) = points.last() {
                wpts.push(wpt(last.lat, last.lon, "End", "Flag, Red", Some(last.e)))
            }
        }
    }
    let camps: Vec<_> = campsites
        .iter()
        .filter(|s| s.lat.is_some() && s.lon.is_some())
        .cloned()
        .collect();
    for g in group_by_location(&camps) {
        let s = g[0];
        let label = format!(
            "Night{} {}",
            if g.len() > 1 { "s" } else { "" },
            g.iter()
                .map(|s| s.night.to_string())
                .collect::<Vec<_>>()
                .join("/")
        );
        if let (Some(lat), Some(lon)) = (s.lat, s.lon) {
            wpts.push(wpt(lat, lon, &label, "Campground", s.ele_ft))
        }
    }
    let trkpts = points
        .iter()
        .map(|p| {
            format!(
                "      <trkpt lat=\"{}\" lon=\"{}\"><ele>{}</ele></trkpt>",
                fixed(p.lat, 6),
                fixed(p.lon, 6),
                fixed(p.e * (1. / 3.28084), 1)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"Backpack\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n  <metadata><name>{}</name></metadata>\n{}\n  <trk><name>{}</name><trkseg>\n{trkpts}\n  </trkseg></trk>\n</gpx>\n",
        esc(name),
        wpts.join("\n"),
        esc(name)
    )
}
