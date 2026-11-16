//! Port of `lib/overpass.ts`: campsite POIs from OpenStreetMap via the Overpass API, cached per
//! Trip bounding box in the store (`overpass_cache`).
use crate::{Cached, Clock, HttpClient, Result, ServiceError, read_through, ttl};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strider_store::{CacheKind, Store};

/// `[south, west, north, east]`.
pub type Bbox = [f64; 4];
pub const MAX_POIS: usize = 60;
/// Mirrors in the order they are tried; the first 2xx JSON reply wins.
pub const MIRRORS: [&str; 3] = [
    "https://overpass-api.de/api/interpreter",
    "https://overpass.kumi.systems/api/interpreter",
    "https://overpass.private.coffee/api/interpreter",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CampPoi {
    pub lat: f64,
    pub lon: f64,
    pub name: Option<String>,
    pub kind: String,
}

/// The route's bounding box padded by `pad_deg` (the web uses 0.01).
pub fn track_bbox(track: &[[f64; 2]], pad_deg: f64) -> Option<Bbox> {
    let mut pts = track
        .iter()
        .filter(|p| p[0].is_finite() && p[1].is_finite());
    let first = pts.next()?;
    let (mut s, mut w, mut n, mut e) = (first[0], first[1], first[0], first[1]);
    for p in pts {
        s = s.min(p[0]);
        n = n.max(p[0]);
        w = w.min(p[1]);
        e = e.max(p[1]);
    }
    Some([s - pad_deg, w - pad_deg, n + pad_deg, e + pad_deg])
}

pub fn cache_key(b: &Bbox) -> String {
    b.iter()
        .map(|n| format!("{n:.2}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The Overpass QL for campsites, wilderness huts and camp pitches inside the box.
pub fn query(b: &Bbox) -> String {
    let [s, w, n, e] = b;
    let bx = format!("({s},{w},{n},{e})");
    format!(
        "[out:json][timeout:25];(node[\"tourism\"=\"camp_site\"]{bx};node[\"tourism\"=\"wilderness_hut\"]{bx};node[\"tourism\"=\"camp_pitch\"]{bx};);out body {MAX_POIS};"
    )
}
fn form_body(b: &Bbox) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("data", &query(b))
        .finish()
}

/// Parse an Overpass JSON reply: nodes with coordinates, at most [`MAX_POIS`].
pub fn parse(body: &str) -> Result<Vec<CampPoi>> {
    let j: Value = serde_json::from_str(body)
        .map_err(|_| ServiceError::Protocol("Invalid Overpass response".into()))?;
    Ok(j.get("elements")
        .and_then(Value::as_array)
        .map(|els| {
            els.iter()
                .filter_map(|el| {
                    let tags = el.get("tags");
                    Some(CampPoi {
                        lat: el.get("lat")?.as_f64()?,
                        lon: el.get("lon")?.as_f64()?,
                        name: tags
                            .and_then(|t| t.get("name"))
                            .and_then(Value::as_str)
                            .map(String::from),
                        kind: tags
                            .and_then(|t| t.get("tourism"))
                            .and_then(Value::as_str)
                            .unwrap_or("camp_site")
                            .to_string(),
                    })
                })
                .take(MAX_POIS)
                .collect()
        })
        .unwrap_or_default())
}

fn fetch_remote(http: &dyn HttpClient, bbox: &Bbox) -> Result<Vec<CampPoi>> {
    let body = form_body(bbox);
    let mut last = ServiceError::Offline;
    for mirror in MIRRORS {
        match http.post_form(mirror, &body) {
            Ok(r) if r.ok() => match parse(&r.body) {
                Ok(pois) => return Ok(pois),
                Err(e) => last = e,
            },
            Ok(r) => last = r.error(),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Campsite POIs for a box: cached for a week, refetched from the first working mirror, and the
/// last stored copy when every mirror fails.
pub fn fetch_campsite_pois(
    store: &Store,
    http: &dyn HttpClient,
    clock: &dyn Clock,
    bbox: &Bbox,
) -> Result<Cached<Vec<CampPoi>>> {
    read_through(
        store,
        CacheKind::Overpass,
        &cache_key(bbox),
        ttl::OVERPASS,
        clock.now(),
        || fetch_remote(http, bbox).map(Some),
    )
    .map(|c| {
        c.unwrap_or(Cached {
            value: vec![],
            fetched_at: clock.now(),
            stale: false,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::{FakeHttp, FixedClock};

    const REPLY: &str = include_str!("../../../fixtures/http/overpass-camp-sites.json");
    const BOX: Bbox = [39.0, -105.3, 39.2, -105.1];

    #[test]
    fn bbox_pads_the_track() {
        let b = track_bbox(&[[40., -120.], [41., -119.], [f64::NAN, 0.]], 0.01).unwrap();
        assert_eq!(b, [39.99, -120.01, 41.01, -118.99]);
        assert!(track_bbox(&[], 0.01).is_none());
    }
    #[test]
    fn query_matches_the_web() {
        assert_eq!(
            query(&BOX),
            "[out:json][timeout:25];(node[\"tourism\"=\"camp_site\"](39,-105.3,39.2,-105.1);node[\"tourism\"=\"wilderness_hut\"](39,-105.3,39.2,-105.1);node[\"tourism\"=\"camp_pitch\"](39,-105.3,39.2,-105.1););out body 60;"
        );
        assert!(form_body(&BOX).starts_with("data=%5Bout%3Ajson%5D"));
        assert_eq!(cache_key(&BOX), "39.00,-105.30,39.20,-105.10");
    }
    #[test]
    fn parses_nodes_with_coordinates_and_default_kind() {
        let pois = parse(REPLY).unwrap();
        assert_eq!(pois.len(), 3, "the way without coordinates is dropped");
        assert_eq!(pois[0].name.as_deref(), Some("Elk Meadow"));
        assert_eq!(pois[0].kind, "camp_site");
        assert_eq!(pois[1].kind, "wilderness_hut");
        assert_eq!(
            (pois[2].name.clone(), pois[2].kind.as_str()),
            (None, "camp_site")
        );
        assert!(parse("<html>").is_err());
        assert!(parse(r#"{"remark":"timeout"}"#).unwrap().is_empty());
    }
    #[test]
    fn caps_at_sixty_pois() {
        let els: Vec<String> = (0..80)
            .map(|i| format!(r#"{{"type":"node","lat":39.{i:02},"lon":-105.1}}"#))
            .collect();
        let body = format!(r#"{{"elements":[{}]}}"#, els.join(","));
        assert_eq!(parse(&body).unwrap().len(), MAX_POIS);
    }
    #[test]
    fn mirrors_are_tried_in_order_until_one_answers() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("overpass-api.de", 504, "busy")
            .route("kumi.systems", 200, REPLY);
        let c = FixedClock::at(1_000);
        let got = fetch_campsite_pois(&store, &http, &c, &BOX).unwrap();
        assert_eq!(got.value.len(), 3);
        let log = http.log.lock().unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].contains("overpass-api.de") && log[1].contains("kumi.systems"));
        assert!(log[1].contains("data=%5Bout"));
    }
    #[test]
    fn cached_per_bbox_and_stale_when_every_mirror_fails() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("overpass-api.de", 200, REPLY);
        let c = FixedClock::at(1_000);
        fetch_campsite_pois(&store, &http, &c, &BOX).unwrap();
        c.advance(3_600);
        let again = fetch_campsite_pois(&store, &http, &c, &BOX).unwrap();
        assert!(!again.stale);
        assert_eq!(http.calls(), 1);
        // Another box is a different cache entry; offline with no copy is an error.
        http.clear();
        assert_eq!(
            fetch_campsite_pois(&store, &http, &c, &[40., -106., 40.2, -105.8]),
            Err(ServiceError::Offline)
        );
        c.advance(8 * 86_400);
        let stale = fetch_campsite_pois(&store, &http, &c, &BOX).unwrap();
        assert!(stale.stale && stale.fetched_at == 1_000 && stale.value.len() == 3);
    }
}
