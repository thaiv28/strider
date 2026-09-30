"use server";

import { fetchClimatology, type Climo } from "@/lib/weather";
import { fetchSnowDepths, type SnowQuery, type SnowResult } from "@/lib/snow";
import { getCurrentUserId } from "@/lib/gear";

// Server-side so the ~10 archive requests per location happen once and are cached.
export async function getClimatology(lat: number | null, lon: number | null, dateISO: string): Promise<Climo | null> {
  await getCurrentUserId();
  return fetchClimatology(lat, lon, dateISO);
}

// Snow depth for the Trips page. Takes coordinates rather than trip ids, so no
// trip data is read; the cap bounds upstream requests per call.
export async function getSnowDepths(queries: SnowQuery[]): Promise<SnowResult[]> {
  await getCurrentUserId();
  const valid = Array.isArray(queries) ? queries.slice(0, 200) : [];
  const clean = valid.map((q) => ({
    lat: Number(q?.lat),
    lon: Number(q?.lon),
    date: typeof q?.date === "string" && /^\d{4}-\d{2}-\d{2}$/.test(q.date) ? q.date : null,
  }));
  const ok = clean.map((q) => Math.abs(q.lat) <= 90 && Math.abs(q.lon) <= 180);
  const results = await fetchSnowDepths(clean.filter((_, i) => ok[i]));
  let k = 0;
  return clean.map((_, i) => (ok[i] ? results[k++] : { nowIn: null, trip: null }));
}
