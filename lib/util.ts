import { clsx, type ClassValue } from "clsx";

export function cn(...inputs: ClassValue[]) {
  return clsx(inputs);
}

export const OZ_TO_G = 28.3495;

export const gToOz = (g: number) => g / OZ_TO_G;
export const ozToG = (oz: number) => Math.round(oz * OZ_TO_G);

/** Primary weight display: oz under a pound, lb + oz at/above. */
export function fmtWeight(g: number | null | undefined): string {
  if (g == null) return "—";
  const oz = gToOz(g);
  if (oz < 16) return `${oz.toFixed(oz < 1 ? 2 : 1)} oz`;
  const lb = Math.floor(oz / 16);
  const rem = oz - lb * 16;
  return `${lb} lb ${rem.toFixed(1)} oz`;
}

export const fmtOz = (g: number | null | undefined) =>
  g == null ? "—" : `${gToOz(g).toFixed(2)} oz`;

export const fmtLbs = (g: number | null | undefined) =>
  g == null ? "—" : `${(gToOz(g) / 16).toFixed(1)} lbs`;

/** Cents → "$1,299" (no cents when whole) / "$12.50". */
export function fmtUsd(cents: number | null | undefined): string {
  if (cents == null) return "—";
  const d = cents / 100;
  return `$${d.toLocaleString("en-US", {
    minimumFractionDigits: cents % 100 === 0 ? 0 : 2,
    maximumFractionDigits: 2,
  })}`;
}
