// Snow depth from Open-Meteo, the same keyless source as lib/weather.ts.
// Values are modeled (forecast models / ERA5 reanalysis) at a single grid
// point, so they are a planning signal rather than a station measurement.

import { addDays } from "@/lib/weather";

export type SnowQuery = { lat: number; lon: number; date: string | null };

// observed: reanalysis/model value for a past date. forecast: within the
// ~16-day horizon. typical: mean for the calendar date across recent years.
export type TripDateSnow = { kind: "observed" | "forecast" | "typical"; inches: number; years?: number };
export type SnowResult = { nowIn: number | null; trip: TripDateSnow | null };

const FORECAST = "https://api.open-meteo.com/v1/forecast";
const ARCHIVE = "https://archive-api.open-meteo.com/v1/archive";
const FORECAST_DAYS = 16;
// ERA5 lands in the archive about five days late; the forecast API covers the gap.
const ARCHIVE_LAG_DAYS = 6;
const TYPICAL_YEARS = 10;

const toIn = (m: number) => m * 39.3701;
const round = (inches: number) => Math.max(0, Math.round(inches));
const mean = (a: number[]) => a.reduce((s, x) => s + x, 0) / a.length;
const coordKey = (q: { lat: number; lon: number }) => `${q.lat.toFixed(3)},${q.lon.toFixed(3)}`;

// Open-Meteo rejects large bursts, and a page of future trips fans out to ten
// archive days each, so cap in-flight requests and retry one throttled reply.
const MAX_IN_FLIGHT = 6;
let inFlight = 0;
const waiting: (() => void)[] = [];

async function getJson(url: string, revalidate: number): Promise<any> {
  if (inFlight >= MAX_IN_FLIGHT) await new Promise<void>((resolve) => waiting.push(resolve));
  inFlight++;
  try {
    for (let attempt = 0; attempt < 2; attempt++) {
      try {
        const r = await fetch(url, { next: { revalidate } });
        if (r.ok) return await r.json();
        if (r.status !== 429 && r.status < 500) return null;
      } catch {
        /* network error: retry once */
      }
      if (attempt === 0) await new Promise((resolve) => setTimeout(resolve, 750));
    }
    return null;
  } finally {
    inFlight--;
    waiting.shift()?.();
  }
}

// Current depth for many coordinates in one request per chunk; Open-Meteo
// accepts comma-separated coordinate lists and returns an array.
async function fetchCurrent(coords: { lat: number; lon: number }[]): Promise<Map<string, number | null>> {
  const out = new Map<string, number | null>();
  for (let i = 0; i < coords.length; i += 50) {
    const chunk = coords.slice(i, i + 50);
    const j = await getJson(
      `${FORECAST}?latitude=${chunk.map((c) => c.lat).join(",")}&longitude=${chunk.map((c) => c.lon).join(",")}` +
        `&current=snow_depth&timezone=auto`,
      1800,
    );
    const list = Array.isArray(j) ? j : j ? [j] : [];
    chunk.forEach((c, k) => {
      const m = list[k]?.current?.snow_depth;
      out.set(coordKey(c), typeof m === "number" ? round(toIn(m)) : null);
    });
  }
  return out;
}

async function archiveDay(lat: number, lon: number, date: string): Promise<number | null> {
  const j = await getJson(
    `${ARCHIVE}?latitude=${lat}&longitude=${lon}&start_date=${date}&end_date=${date}&daily=snow_depth_mean&timezone=auto`,
    7 * 86400,
  );
  const m = j?.daily?.snow_depth_mean?.[0];
  return typeof m === "number" ? toIn(m) : null;
}

async function forecastDay(lat: number, lon: number, date: string): Promise<number | null> {
  const j = await getJson(
    `${FORECAST}?latitude=${lat}&longitude=${lon}&start_date=${date}&end_date=${date}&hourly=snow_depth&timezone=auto`,
    3600,
  );
  const hours: unknown[] = j?.hourly?.snow_depth ?? [];
  const vals = hours.filter((v): v is number => typeof v === "number");
  return vals.length ? toIn(mean(vals)) : null;
}

async function typicalDay(lat: number, lon: number, date: string): Promise<TripDateSnow | null> {
  const md = date.slice(5);
  const lastYear = new Date().getUTCFullYear() - 1;
  const years = Array.from({ length: TYPICAL_YEARS }, (_, i) => lastYear - i)
    // Feb 29 only exists in leap years; fall back to Feb 28 elsewhere.
    .map((y) => (md === "02-29" && !(y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0)) ? `${y}-02-28` : `${y}-${md}`));
  const vals = (await Promise.all(years.map((d) => archiveDay(lat, lon, d)))).filter((v): v is number => v != null);
  return vals.length ? { kind: "typical", inches: round(mean(vals)), years: vals.length } : null;
}

async function tripDateSnow(lat: number, lon: number, date: string, today: string): Promise<TripDateSnow | null> {
  if (date > addDays(today, FORECAST_DAYS - 1)) return typicalDay(lat, lon, date);
  if (date < addDays(today, -ARCHIVE_LAG_DAYS)) {
    const v = await archiveDay(lat, lon, date);
    return v == null ? null : { kind: "observed", inches: round(v) };
  }
  const v = await forecastDay(lat, lon, date);
  return v == null ? null : { kind: date < today ? "observed" : "forecast", inches: round(v) };
}

export async function fetchSnowDepths(queries: SnowQuery[]): Promise<SnowResult[]> {
  const today = new Date().toISOString().slice(0, 10);
  const unique = [...new Map(queries.map((q) => [coordKey(q), { lat: q.lat, lon: q.lon }])).values()];
  const [current, trips] = await Promise.all([
    fetchCurrent(unique),
    Promise.all(queries.map((q) => (q.date ? tripDateSnow(q.lat, q.lon, q.date, today) : Promise.resolve(null)))),
  ]);
  return queries.map((q, i) => ({ nowIn: current.get(coordKey(q)) ?? null, trip: trips[i] }));
}
