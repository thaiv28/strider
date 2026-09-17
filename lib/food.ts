import { OZ_TO_G } from "@/lib/util";

// Volume units → milliliters. Used for weight↔volume when a density is known.
export const VOLUME_ML: Record<string, number> = {
  ml: 1,
  l: 1000,
  tsp: 4.92892,
  tbsp: 14.7868,
  floz: 29.5735,
  cup: 236.588,
};
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
