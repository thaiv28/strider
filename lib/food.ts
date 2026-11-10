import { OZ_TO_G } from "@/lib/util";

// Volume units → milliliters. Used for weight↔volume when a density is known.
export const VOLUME_ML = {
  ml: 1,
  l: 1000,
  tsp: 4.92892,
  tbsp: 14.7868,
  floz: 29.5735,
  cup: 236.588,
} as const;
export type VolumeUnit = keyof typeof VOLUME_ML;

export const kcalPerOz = (kcalPer100g: number | null): number | null =>
  kcalPer100g == null ? null : (kcalPer100g / 100) * OZ_TO_G;

export const kcalForGrams = (kcalPer100g: number | null, grams: number): number =>
  kcalPer100g == null ? 0 : Math.round((kcalPer100g / 100) * grams);

/** Grams for a volume, given density (g/mL). null when density unknown. */
export const gramsFromVolume = (amount: number, unit: VolumeUnit, density: number | null): number | null =>
  density == null ? null : Math.round(amount * VOLUME_ML[unit] * density);

/** Volume (in `unit`) for a mass, given density. null when density unknown. */
export const volumeFromGrams = (grams: number, unit: VolumeUnit, density: number | null): number | null =>
  density == null || density === 0 ? null : grams / density / VOLUME_ML[unit];

export const num = (s: string | null | undefined): number | null =>
  s == null || s === "" ? null : Number(s);

export const isAmountUnit = (u: string): boolean =>
  u === "g" || u === "oz" || Object.prototype.hasOwnProperty.call(VOLUME_ML, u);

export function parseVolumeUnit(label: string): VolumeUnit | null {
  const text = label.toLowerCase().replace(/\./g, "");
  if (/\b(?:fl\s+oz|fluid\s+ounces?)\b/.test(text)) return "floz";
  const aliases: Record<string, VolumeUnit> = {
    cup: "cup", cups: "cup", tablespoon: "tbsp", tablespoons: "tbsp", tbsp: "tbsp", tbs: "tbsp",
    teaspoon: "tsp", teaspoons: "tsp", tsp: "tsp", milliliter: "ml", milliliters: "ml",
    millilitre: "ml", millilitres: "ml", ml: "ml", liter: "l", liters: "l", litre: "l", litres: "l", l: "l",
  };
  for (const word of text.split(/[^a-z]+/)) if (aliases[word]) return aliases[word];
  return null;
}

export function densityFromServing(text: string | null | undefined, gramsHint?: number | null): number | null {
  if (!text) return null;
  const grams = gramsHint ?? Number(text.match(/(\d+(?:\.\d+)?)\s*(?:grams?|gr|g)\b/i)?.[1]);
  if (!Number.isFinite(grams) || grams <= 0) return null;
  const portions = text.matchAll(/(\d+\s+\d+\/\d+|\d+\/\d+|\d+(?:\.\d+)?)\s*([a-z]+(?:\.?(?:\s+)(?:oz|ounces?))?\.?)/gi);
  for (const match of portions) {
    const unit = parseVolumeUnit(match[2]);
    if (!unit) continue;
    const qty = match[1].split(/\s+/).reduce((sum, part) => {
      const [a, b] = part.split("/").map(Number);
      return sum + (b == null ? a : a / b);
    }, 0);
    const density = +(grams / (qty * VOLUME_ML[unit])).toFixed(3);
    return Number.isFinite(density) && density >= 0.02 && density <= 2.5 ? density : null;
  }
  return null;
}
