pub const GAS_PER_LITER_G: f64 = 12.;
pub const DEFAULT_COOK_WATER_ML: i64 = 500;
pub const CANISTERS: [(i64, i64); 3] = [(110, 200), (230, 375), (450, 655)];
#[derive(Clone, Debug, PartialEq)]
pub struct FuelPlan {
    pub cook_water_ml: f64,
    pub liters: f64,
    pub needed_gas_g: i64,
    pub gas_provided_g: i64,
    pub packed_weight_g: i64,
    pub label: String,
}
pub fn fuel_plan(cook_water_ml: f64) -> FuelPlan {
    let liters = cook_water_ml / 1000.;
    let needed_gas_g = (liters * GAS_PER_LITER_G).ceil() as i64;
    let (gas_provided_g, packed_weight_g, label) = if needed_gas_g <= 0 {
        (0, 0, "none".into())
    } else if let Some(&(net, full)) = CANISTERS.iter().find(|&&(net, _)| net >= needed_gas_g) {
        (net, full, format!("one {net} g canister"))
    } else {
        let count = (needed_gas_g + 449) / 450;
        (
            450 * count,
            655 * count,
            format!("{count} × 450 g canisters"),
        )
    };
    FuelPlan {
        cook_water_ml,
        liters,
        needed_gas_g,
        gas_provided_g,
        packed_weight_g,
        label,
    }
}
