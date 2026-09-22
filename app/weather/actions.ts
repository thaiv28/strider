"use server";

import { fetchClimatology, type Climo } from "@/lib/weather";
import { getCurrentUserId } from "@/lib/gear";

// Server-side so the ~10 archive requests per location happen once and are cached.
export async function getClimatology(lat: number | null, lon: number | null, dateISO: string): Promise<Climo | null> {
  await getCurrentUserId();
  return fetchClimatology(lat, lon, dateISO);
}
