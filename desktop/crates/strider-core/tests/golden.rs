use serde_json::{Value, json};
use strider_core::{energy::*, food::*, fuel::*, route::*, *};
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.)
}
fn optional(v: &Value) -> Option<f64> {
    (!v.is_null()).then(|| f(v))
}
fn grams(v: &Value) -> Option<Grams> {
    optional(v).map(|n| Grams(n as i32))
}
fn stats(v: &Value, trail: bool) -> Option<RouteStats> {
    if v.is_null() {
        return None;
    }
    let dist = &v[if trail {
        "typicalDistanceMi"
    } else {
        "distanceMi"
    }];
    Some(RouteStats {
        distance_mi: dist.as_str().and_then(|s| s.parse().ok()),
        elevation_ft: v[if trail {
            "typicalElevationFt"
        } else {
            "elevationGainFt"
        }]
        .as_i64(),
    })
}
fn run(name: &str, a: &[Value]) -> Value {
    match name {
        "fmtWeight" => json!(fmt_weight(grams(&a[0]))),
        "fmtOz" => json!(fmt_oz(grams(&a[0]))),
        "fmtLbs" => json!(fmt_lbs(grams(&a[0]))),
        "displayOz" => json!(display_oz(Grams(f(&a[0]) as i32))),
        "gToOz" => json!(g_to_oz(Grams(f(&a[0]) as i32))),
        "ozToG" => json!(oz_to_g(f(&a[0])).0),
        "fmtUsd" => json!(fmt_usd(optional(&a[0]).map(|n| Cents(n as i32)))),
        "fmtDateRange" => json!(fmt_date_range(
            a[0].as_str(),
            a[1].as_i64().map(|n| n as i32)
        )),
        "kcalPerOz" => json!(kcal_per_oz(optional(&a[0]))),
        "kcalForGrams" => json!(kcal_for_grams(optional(&a[0]), f(&a[1]))),
        "gramsFromVolume" => json!(grams_from_volume(
            f(&a[0]),
            a[1].as_str().unwrap(),
            optional(&a[2])
        )),
        "volumeFromGrams" => json!(volume_from_grams(
            f(&a[0]),
            a[1].as_str().unwrap(),
            optional(&a[2])
        )),
        "parseVolumeUnit" => json!(parse_volume_unit(a[0].as_str().unwrap())),
        "densityFromServing" => json!(density_from_serving(a[0].as_str(), optional(&a[1]))),
        "num" => json!(num(a[0].as_str())),
        "isAmountUnit" => json!(is_amount_unit(a[0].as_str().unwrap())),
        "fuelPlan" => {
            let p = fuel_plan(f(&a[0]));
            json!({"cookWaterMl":p.cook_water_ml,"liters":p.liters,"neededGasG":p.needed_gas_g,"gasProvidedG":p.gas_provided_g,"packedWeightG":p.packed_weight_g,"label":p.label})
        }
        "mifflinBmr" => json!(mifflin_bmr(
            a[0].as_str(),
            optional(&a[1]),
            optional(&a[2]),
            optional(&a[3])
        )),
        "dayTargetKcal" => json!(day_target_kcal(
            &EnergyParams {
                bmr: f(&a[0]["bmr"]),
                cal_per_mile: f(&a[0]["calPerMile"]),
                ft_per_mile: f(&a[0]["ftPerMile"])
            },
            optional(&a[1]),
            optional(&a[2])
        )),
        "effectiveRouteStats" => {
            let p = effective_route_stats(
                stats(&a[0], false).unwrap(),
                stats(&a[1], false),
                stats(&a[2], true),
            );
            json!({"distanceMi":p.distance_mi,"elevationFt":p.elevation_ft})
        }
        "computeDaySegments" => {
            let pts: Vec<_> = a[0]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| RoutePt {
                    d: f(&p["d"]),
                    lat: f(&p["lat"]),
                    lon: f(&p["lon"]),
                    e: f(&p["e"]),
                })
                .collect();
            let camps: Vec<_> = a[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Campsite {
                    distance_mi: f(&p["distanceMi"]),
                    lat: optional(&p["lat"]),
                    lon: optional(&p["lon"]),
                })
                .collect();
            json!(compute_day_segments(&pts,&camps).iter().map(|p| json!({"day":p.day,"distanceMi":p.distance_mi,"elevationGainFt":p.elevation_gain_ft,"lat":p.lat,"lon":p.lon})).collect::<Vec<_>>())
        }
        _ => panic!("unknown fixture function {name}"),
    }
}
fn compare(actual: &Value, expected: &Value, context: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!(
                (a - e).abs() <= 1e-9 * e.abs().max(1.),
                "{context}: {actual} != {expected}"
            );
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(a.len(), e.len(), "{context}");
            for (a, e) in a.iter().zip(e) {
                compare(a, e, context);
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            assert_eq!(a.len(), e.len(), "{context}");
            for (key, e) in e {
                compare(&a[key], e, context);
            }
        }
        _ => assert_eq!(actual, expected, "{context}"),
    }
}
#[test]
fn typescript_golden_fixtures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/domain");
    let mut count = 0;
    for module in ["util", "food", "fuel", "energy", "trip"] {
        let cases: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(format!("{module}.json"))).unwrap(),
        )
        .unwrap();
        for (index, case) in cases.as_array().unwrap().iter().enumerate() {
            let name = case["fn"].as_str().unwrap();
            compare(
                &run(name, case["args"].as_array().unwrap()),
                &case["out"],
                &format!("{module}[{index}] {name}({})", case["args"]),
            );
            count += 1;
        }
    }
    assert_eq!(count, 3808);
}
