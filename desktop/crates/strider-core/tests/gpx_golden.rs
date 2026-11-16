use serde_json::{Value, json};
use strider_core::gpx::*;
struct Item(Value);
impl Location for Item {
    fn location(&self) -> (Option<f64>, Option<f64>) {
        (self.0["lat"].as_f64(), self.0["lon"].as_f64())
    }
}
fn exact(a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => assert_eq!(a.as_f64(), b.as_f64()),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                exact(a, b)
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (k, b) in b {
                exact(&a[k], b)
            }
        }
        _ => assert_eq!(a, b),
    }
}
#[test]
fn all_69_gpx_goldens() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/domain/gpx.json")).unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 69);
    for (i, c) in cases.as_array().unwrap().iter().enumerate() {
        let a = c["args"].as_array().unwrap();
        let out = match c["fn"].as_str().unwrap() {
            "parseGpx" => json!(parse_gpx(a[0].as_str().unwrap())),
            "gpxFilename" => json!(gpx_filename(a[0].as_str().unwrap())),
            "isLoopTrack" => json!(is_loop_track(
                &serde_json::from_value::<Vec<[f64; 2]>>(a[0].clone()).unwrap()
            )),
            "groupByLocation" => {
                let items: Vec<_> = a[0].as_array().unwrap().iter().cloned().map(Item).collect();
                json!(
                    group_by_location(&items)
                        .iter()
                        .map(|g| g.iter().map(|v| v.0.clone()).collect::<Vec<_>>())
                        .collect::<Vec<_>>()
                )
            }
            "buildGpx" => {
                let pts = a[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| ExportPoint {
                        lat: v["lat"].as_f64().unwrap(),
                        lon: v["lon"].as_f64().unwrap(),
                        e: v["e"].as_f64().unwrap(),
                    })
                    .collect::<Vec<_>>();
                json!(build_gpx(
                    a[0].as_str().unwrap(),
                    &pts,
                    &serde_json::from_value::<Vec<ExportCampsite>>(a[2].clone()).unwrap()
                ))
            }
            _ => panic!("case {i}"),
        };
        eprintln!("case {i} {}", c["fn"]);
        exact(&out, &c["out"]);
    }
}
