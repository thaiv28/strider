//! Pure counterparts of `lib/trip-propagation.ts` SQL updates.
use crate::units::{Dec, rounded_div};
const SCALE: i128 = 1_000_000_000;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ingredient {
    pub name: String,
    pub kcal_per_100g: Option<Dec>,
    pub density_g_ml: Option<Dec>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MealLine {
    pub snapshot_name: String,
    pub snapshot_kcal_per_100g: Option<Dec>,
    pub snapshot_density_g_ml: Option<Dec>,
    pub amount_g: i64,
    pub entered_amount: Option<Dec>,
    pub entered_unit: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TripMealSnapshot {
    pub snapshot_name: String,
    pub snapshot_kcal: i64,
    pub snapshot_weight_g: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GearSnapshot {
    pub snapshot_name: String,
    pub snapshot_weight_g: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GearItem {
    pub name: String,
    pub weight_g: Option<i64>,
}
/// Empty recipes return None: the SQL inner join deliberately preserves imported
/// or previous trip totals after the last recipe line is removed.
pub fn meal_totals(lines: &[MealLine]) -> Option<(i64, i64)> {
    if lines.is_empty() {
        return None;
    }
    Some((
        lines.iter().map(|l| l.amount_g).sum(),
        lines
            .iter()
            .map(|l| {
                rounded_div(
                    l.snapshot_kcal_per_100g.unwrap_or(Dec::ZERO).scaled() * i128::from(l.amount_g),
                    100 * SCALE,
                ) as i64
            })
            .sum(),
    ))
}
/// Returns (kcal, grams), rounding each scaled whole-recipe total once.
pub fn trip_meal_snapshot(totals: (i64, i64), servings: Dec, base_servings: i64) -> (i64, i64) {
    let denominator = i128::from(base_servings.max(1)) * SCALE;
    (
        rounded_div(i128::from(totals.1) * servings.scaled(), denominator) as i64,
        rounded_div(i128::from(totals.0) * servings.scaled(), denominator) as i64,
    )
}
fn volume_ml(unit: &str) -> Option<Dec> {
    Some(Dec::from_scaled(match unit {
        "ml" => 1_000_000_000,
        "l" => 1_000_000_000_000,
        "tsp" => 4_928_920_000,
        "tbsp" => 14_786_800_000,
        "floz" => 29_573_500_000,
        "cup" => 236_588_000_000,
        _ => return None,
    }))
}
pub fn meal_line_from_ingredient(line: &mut MealLine, ingredient: &Ingredient) {
    line.snapshot_name.clone_from(&ingredient.name);
    if let Some(kcal) = ingredient.kcal_per_100g {
        line.snapshot_kcal_per_100g = Some(Dec::from(kcal.round_half_away()));
    }
    line.snapshot_density_g_ml = ingredient.density_g_ml;
    if let (Some(density), Some(amount), Some(ml)) = (
        ingredient.density_g_ml,
        line.entered_amount,
        line.entered_unit.as_deref().and_then(volume_ml),
    ) {
        line.amount_g = rounded_div(
            amount.scaled() * ml.scaled() * density.scaled(),
            SCALE * SCALE * SCALE,
        ) as i64;
    }
}
pub fn standalone_trip_meal_from_ingredient(entry: &mut TripMealSnapshot, ingredient: &Ingredient) {
    entry.snapshot_name.clone_from(&ingredient.name);
    if let Some(kcal) = ingredient.kcal_per_100g {
        entry.snapshot_kcal = rounded_div(
            kcal.scaled() * i128::from(entry.snapshot_weight_g),
            100 * SCALE,
        ) as i64;
    }
}
pub fn trip_food_g_per_day(sum_snapshot_g: i64, nights: Option<i32>) -> i64 {
    rounded_div(
        i128::from(sum_snapshot_g),
        (i128::from(nights.unwrap_or(0)) + 1).max(1),
    ) as i64
}
pub fn trip_gear_from_item(snapshot: &mut GearSnapshot, item: &GearItem) {
    snapshot.snapshot_name.clone_from(&item.name);
    if let Some(weight) = item.weight_g {
        snapshot.snapshot_weight_g = weight;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line() -> MealLine {
        MealLine {
            snapshot_name: "Old".into(),
            snapshot_kcal_per_100g: Some(Dec::from(50)),
            snapshot_density_g_ml: Some(Dec::ONE),
            amount_g: 1,
            entered_amount: Some(Dec::ONE),
            entered_unit: Some("ml".into()),
        }
    }
    #[test]
    fn round_per_line_and_preserve_empty() {
        assert_eq!(meal_totals(&[]), None);
        assert_eq!(meal_totals(&[line(), line()]), Some((2, 2)));
        assert_eq!(trip_meal_snapshot((1, 1), Dec::ONE, 2), (1, 1));
        assert_eq!(trip_food_g_per_day(1, Some(1)), 1);
        assert_eq!(trip_food_g_per_day(30, Some(-3)), 30);
        assert_eq!(trip_food_g_per_day(1_000_000_000, Some(2_000_000_000)), 0);
    }
    #[test]
    fn ingredient_null_and_density_updates() {
        let mut l = line();
        let mut i = Ingredient {
            name: "New".into(),
            kcal_per_100g: None,
            density_g_ml: None,
        };
        meal_line_from_ingredient(&mut l, &i);
        assert_eq!(l.snapshot_name, "New");
        assert_eq!(l.snapshot_kcal_per_100g, Some(Dec::from(50)));
        assert_eq!(l.snapshot_density_g_ml, None);
        assert_eq!(l.amount_g, 1);
        i.density_g_ml = Some("1.5".parse().unwrap());
        i.kcal_per_100g = Some("100.5".parse().unwrap());
        meal_line_from_ingredient(&mut l, &i);
        assert_eq!(l.amount_g, 2);
        assert_eq!(l.snapshot_kcal_per_100g, Some(Dec::from(101)));
        let mut t = TripMealSnapshot {
            snapshot_name: "Old".into(),
            snapshot_kcal: 80,
            snapshot_weight_g: 1,
        };
        standalone_trip_meal_from_ingredient(&mut t, &i);
        assert_eq!(t.snapshot_kcal, 1);
        i.kcal_per_100g = None;
        standalone_trip_meal_from_ingredient(&mut t, &i);
        assert_eq!(t.snapshot_kcal, 1);
    }
    #[test]
    fn unknown_gear_weight_preserves_snapshot() {
        let mut s = GearSnapshot {
            snapshot_name: "Old".into(),
            snapshot_weight_g: 7,
        };
        trip_gear_from_item(
            &mut s,
            &GearItem {
                name: "New".into(),
                weight_g: None,
            },
        );
        assert_eq!(
            s,
            GearSnapshot {
                snapshot_name: "New".into(),
                snapshot_weight_g: 7
            }
        );
    }
}
