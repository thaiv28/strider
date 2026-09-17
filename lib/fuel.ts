// Canister-fuel model for the trip fuel calculator. Pure math (client-safe).
//
// Assumes a Soto WindMaster canister stove in average temps/wind/altitude. The
// gas-per-liter figure is empirical for an efficient upright canister stove
// bringing water to a boil; cold, wind, and altitude raise it.
export const GAS_PER_LITER_G = 12;

// Reasonable boil volume for a hot meal that doesn't specify its own — one
// freeze-dried entrée or a mug of coffee + oats is ~2 cups.
export const DEFAULT_COOK_WATER_ML = 500;

// Standard isobutane/propane canisters: net gas + typical FULL (packed) weight
// in grams — the empty canister adds ~90–210 g of tare you actually carry.
export const CANISTERS = [
  { net: 110, full: 200 },
  { net: 230, full: 375 },
  { net: 450, full: 655 },
] as const;

export type FuelPlan = {
  cookWaterMl: number; // total for the whole party
  liters: number;
  neededGasG: number; // gas that must be burned
  gasProvidedG: number; // net gas in the recommended canister(s)
  packedWeightG: number; // full weight of the canister(s) — what you carry
  label: string; // e.g. "one 110 g canister"
};

// Recommend the smallest single canister whose net gas covers the need; above
// the largest size, stack the largest. Rounding up to a full canister is the
// safety margin.
export function fuelPlan(cookWaterMl: number): FuelPlan {
  const liters = cookWaterMl / 1000;
  const neededGasG = Math.ceil(liters * GAS_PER_LITER_G);
  const base = { cookWaterMl, liters, neededGasG };
  if (neededGasG <= 0) return { ...base, gasProvidedG: 0, packedWeightG: 0, label: "none" };

  const single = CANISTERS.find((c) => c.net >= neededGasG);
  if (single)
    return { ...base, gasProvidedG: single.net, packedWeightG: single.full, label: `one ${single.net} g canister` };

  const big = CANISTERS[CANISTERS.length - 1];
  const count = Math.ceil(neededGasG / big.net);
  return {
    ...base,
    gasProvidedG: big.net * count,
    packedWeightG: big.full * count,
    label: `${count} × ${big.net} g canisters`,
  };
}
