//! Database-free trip weights, food planning and shopping-list calculations.
use crate::format::js_round;
use crate::{
    energy::{EnergyParams, day_target_kcal},
    gear::WeightClass,
};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TripGear {
    pub weight_g: i64,
    pub quantity: i64,
    pub weight_class: WeightClass,
    pub packed: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TripWeights {
    pub days: i64,
    pub base: i64,
    pub worn: i64,
    pub food: i64,
    pub water: i64,
    pub fuel: i64,
    pub consumable: i64,
    pub pack: i64,
    pub skin_out: i64,
}
/// `packed` is checklist state only and never changes carried weight. Water and
/// fuel are flat totals, despite the historical server column suffix _per_day.
pub fn trip_weights(
    gear: &[TripGear],
    nights: Option<i32>,
    food_g_per_day: i64,
    water: i64,
    fuel: i64,
) -> TripWeights {
    let days = (i64::from(nights.unwrap_or(0)) + 1).max(1);
    let base = gear
        .iter()
        .filter(|r| r.weight_class == WeightClass::Base)
        .map(|r| r.weight_g * r.quantity)
        .sum();
    let worn = gear
        .iter()
        .filter(|r| r.weight_class == WeightClass::Worn)
        .map(|r| r.weight_g * r.quantity)
        .sum();
    let food = food_g_per_day * days;
    let consumable = food + water + fuel;
    TripWeights {
        days,
        base,
        worn,
        food,
        water,
        fuel,
        consumable,
        pack: base + consumable,
        skin_out: base + consumable + worn,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TripFoodEntry {
    pub id: String,
    pub day_number: i32,
    pub meal_type: Option<String>,
    pub name: String,
    pub meal_id: Option<String>,
    pub servings: f64,
    pub weight_g: i64,
    pub kcal: i64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DayInput {
    pub day_number: i32,
    pub distance_mi: Option<f64>,
    pub gain_ft: Option<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TripFoodDay {
    pub day_number: i32,
    pub distance_mi: Option<f64>,
    pub gain_ft: Option<f64>,
    pub target_kcal: i64,
    pub planned_kcal: i64,
    pub planned_g: i64,
    pub entries: Vec<TripFoodEntry>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TripFood {
    pub params: EnergyParams,
    pub days: Vec<TripFoodDay>,
    pub total_kcal: i64,
    pub total_g: i64,
    pub avg_kcal: i64,
}
pub fn trip_food(
    params: &EnergyParams,
    nights: Option<i32>,
    day_rows: &[DayInput],
    entries: &[TripFoodEntry],
) -> TripFood {
    let count = (day_rows.len() as i64)
        .max(i64::from(nights.unwrap_or(0)) + 1)
        .max(i64::from(
            entries.iter().map(|e| e.day_number).max().unwrap_or(0),
        ))
        .max(1);
    let mut days = Vec::new();
    let mut total_kcal = 0;
    let mut total_g = 0;
    for day in 1..=count {
        let row = day_rows
            .iter()
            .rev()
            .find(|r| i64::from(r.day_number) == day);
        let distance_mi = row.and_then(|r| r.distance_mi);
        let gain_ft = row.and_then(|r| r.gain_ft);
        let mut es: Vec<_> = entries
            .iter()
            .filter(|e| i64::from(e.day_number) == day)
            .cloned()
            .collect();
        for entry in &mut es {
            if entry.servings == 0. || entry.servings.is_nan() {
                entry.servings = 1.;
            }
        }
        es.sort_by(|a, b| a.id.cmp(&b.id));
        let planned_kcal = es.iter().map(|e| e.kcal).sum();
        let planned_g = es.iter().map(|e| e.weight_g).sum();
        total_kcal += planned_kcal;
        total_g += planned_g;
        days.push(TripFoodDay {
            day_number: day as i32,
            distance_mi,
            gain_ft,
            target_kcal: day_target_kcal(params, distance_mi, gain_ft),
            planned_kcal,
            planned_g,
            entries: es,
        });
    }
    TripFood {
        params: *params,
        days,
        total_kcal,
        total_g,
        avg_kcal: js_round(total_kcal as f64 / count as f64) as i64,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ShoppingEntry {
    pub meal_id: Option<String>,
    pub ingredient_id: Option<String>,
    pub servings: f64,
    pub snapshot_name: String,
    pub snapshot_weight_g: i64,
    pub base_servings: Option<i32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShoppingLine {
    pub meal_id: String,
    pub ingredient_id: Option<String>,
    pub snapshot_name: String,
    pub amount_g: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShoppingItem {
    pub key: String,
    pub name: String,
    pub grams: i64,
}
/// Scale and round each recipe line per planned entry *before* consolidation.
/// Ingredient identities merge across recipes; orphaned snapshots merge by
/// trimmed lowercase name. Nonpositive line quantities are omitted.
/// Sorting uses lowercase lexical order; std has no ICU locale collation, so
/// accented or punctuation-heavy names can order differently from localeCompare.
pub fn shopping_list(entries: &[ShoppingEntry], lines: &[ShoppingLine]) -> Vec<ShoppingItem> {
    let mut acc = BTreeMap::<String, ShoppingItem>::new();
    let mut add = |id: Option<&str>, name: &str, grams: i64| {
        if grams <= 0 {
            return;
        }
        let key = id.map_or_else(
            || format!("n:{}", name.trim().to_lowercase()),
            |id| format!("i{id}"),
        );
        acc.entry(key.clone())
            .and_modify(|e| e.grams += grams)
            .or_insert_with(|| ShoppingItem {
                key,
                name: name.trim().to_owned(),
                grams,
            });
    };
    for e in entries {
        if let Some(meal_id) = &e.meal_id {
            let servings = if e.servings == 0. || e.servings.is_nan() {
                1.
            } else {
                e.servings
            };
            let scale = servings / f64::from(e.base_servings.unwrap_or(1).max(1));
            for line in lines.iter().filter(|l| &l.meal_id == meal_id) {
                add(
                    line.ingredient_id.as_deref(),
                    &line.snapshot_name,
                    js_round(line.amount_g as f64 * scale) as i64,
                );
            }
        } else {
            add(
                e.ingredient_id.as_deref(),
                &e.snapshot_name,
                e.snapshot_weight_g,
            );
        }
    }
    let mut result: Vec<_> = acc.into_values().collect();
    result.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    result
}
#[derive(Clone, Debug, PartialEq)]
pub struct MealWater {
    pub is_hot: bool,
    pub water_ml: Option<i64>,
    pub base_servings: Option<i32>,
    pub servings: f64,
}
pub fn cook_water_per_person(rows: &[MealWater]) -> i64 {
    js_round(
        rows.iter()
            .filter(|r| r.is_hot)
            .map(|r| {
                r.water_ml.unwrap_or(500) as f64 / f64::from(r.base_servings.unwrap_or(1).max(1))
                    * if r.servings == 0. || r.servings.is_nan() {
                        1.
                    } else {
                        r.servings
                    }
            })
            .sum::<f64>(),
    ) as i64
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_trip_amounts_below_half_round_down() {
        let servings = 0.9999999999999999;
        let entries = [ShoppingEntry {
            meal_id: Some("meal".into()),
            ingredient_id: None,
            servings,
            snapshot_name: "Meal".into(),
            snapshot_weight_g: 1,
            base_servings: Some(2),
        }];
        let lines = [ShoppingLine {
            meal_id: "meal".into(),
            ingredient_id: None,
            snapshot_name: "Rice".into(),
            amount_g: 1,
        }];
        assert!(shopping_list(&entries, &lines).is_empty());
        assert_eq!(
            cook_water_per_person(&[MealWater {
                is_hot: true,
                water_ml: Some(1),
                base_servings: Some(2),
                servings,
            }]),
            0
        );
    }
    #[test]
    fn weight_semantics() {
        let w = trip_weights(
            &[
                TripGear {
                    weight_g: 100,
                    quantity: 2,
                    weight_class: WeightClass::Base,
                    packed: false,
                },
                TripGear {
                    weight_g: 50,
                    quantity: 1,
                    weight_class: WeightClass::Worn,
                    packed: true,
                },
            ],
            Some(2),
            500,
            1000,
            200,
        );
        assert_eq!(
            (w.base, w.worn, w.food, w.consumable, w.pack, w.skin_out),
            (200, 50, 1500, 2700, 2900, 2950)
        );
        assert_eq!(trip_weights(&[], Some(-5), 1, 0, 0).days, 1);
    }
    #[test]
    fn hot_water_defaults_and_cold_exclusion() {
        assert_eq!(
            cook_water_per_person(&[
                MealWater {
                    is_hot: true,
                    water_ml: None,
                    base_servings: Some(2),
                    servings: 3.
                },
                MealWater {
                    is_hot: false,
                    water_ml: Some(900),
                    base_servings: None,
                    servings: 1.
                }
            ]),
            750
        );
    }
    #[test]
    fn empty_food_still_has_one_rest_day() {
        let p = trip_food(&EnergyParams::default(), None, &[], &[]);
        assert_eq!(p.days.len(), 1);
        assert_eq!(p.days[0].target_kcal, 1735);
        assert_eq!(p.avg_kcal, 0);
    }
    #[test]
    fn food_days_extend_to_last_meal_and_normalize_servings() {
        let params = EnergyParams::default();
        let entry = TripFoodEntry {
            id: "entry".into(),
            day_number: 3,
            meal_type: None,
            name: "Rice".into(),
            meal_id: None,
            servings: 0.,
            weight_g: 100,
            kcal: 350,
        };
        let result = trip_food(&params, None, &[], &[entry]);
        assert_eq!(result.params, params);
        assert_eq!(result.days.len(), 3);
        assert_eq!(result.days[2].entries[0].servings, 1.);
        assert_eq!(
            (result.total_g, result.total_kcal, result.avg_kcal),
            (100, 350, 117)
        );
    }
    #[test]
    fn shopping_rounds_each_line_and_merges_ids() {
        let e = ShoppingEntry {
            meal_id: Some("m".into()),
            ingredient_id: None,
            servings: 1.,
            snapshot_name: "ignored".into(),
            snapshot_weight_g: 1,
            base_servings: Some(2),
        };
        let line = ShoppingLine {
            meal_id: "m".into(),
            ingredient_id: Some("x".into()),
            snapshot_name: " Rice ".into(),
            amount_g: 1,
        };
        let items = shopping_list(&[e.clone(), e], &[line]);
        assert_eq!(
            items,
            vec![ShoppingItem {
                key: "ix".into(),
                name: "Rice".into(),
                grams: 2
            }]
        );
    }
}
