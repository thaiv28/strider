use crate::format::js_round;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergyParams {
    pub bmr: f64,
    pub cal_per_mile: f64,
    pub ft_per_mile: f64,
}
impl Default for EnergyParams {
    fn default() -> Self {
        Self {
            bmr: 1735.,
            cal_per_mile: 200.,
            ft_per_mile: 625.,
        }
    }
}
pub fn mifflin_bmr(
    sex: Option<&str>,
    weight_lb: Option<f64>,
    height_in: Option<f64>,
    age: Option<f64>,
) -> Option<i64> {
    let (sex, weight, height, age) = (sex.filter(|s| !s.is_empty())?, weight_lb?, height_in?, age?);
    if weight == 0. || height == 0. || age == 0. {
        return None;
    }
    Some(js_round(
        10. * (weight * 0.453592) + 6.25 * (height * 2.54) - 5. * age
            + if sex == "female" { -161. } else { 5. },
    ) as i64)
}
pub fn day_target_kcal(p: &EnergyParams, distance_mi: Option<f64>, gain_ft: Option<f64>) -> i64 {
    js_round(
        p.bmr
            + (distance_mi.unwrap_or(0.) + gain_ft.unwrap_or(0.) / p.ft_per_mile) * p.cal_per_mile,
    ) as i64
}
