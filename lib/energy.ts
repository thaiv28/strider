// Mifflin–St Jeor basal metabolic rate from imperial body stats. Returns null
// until every input is present. men: +5, women: −161.
export function mifflinBmr(sex: string | null, weightLb: number | null, heightIn: number | null, age: number | null): number | null {
  if (!sex || !weightLb || !heightIn || !age) return null;
  const kg = weightLb * 0.453592;
  const cm = heightIn * 2.54;
  const base = 10 * kg + 6.25 * cm - 5 * age;
  return Math.round(base + (sex === "female" ? -161 : 5));
}

export type EnergyParams = {
  bmr: number;
  calPerMile: number;
  ftPerMile: number;
  sex: string | null;
  weightLb: number | null;
  heightIn: number | null;
  ageYears: number | null;
};
