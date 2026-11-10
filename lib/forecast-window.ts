import { fmtDateRange } from "@/lib/util";

// NOAA WPC publishes precipitation forecasts as fixed periods, counted in whole
// days from the current forecast's start. These are its WMS layer ids.
const PERIODS = [
  { first: 0, last: 0, layer: "25" },
  { first: 1, last: 1, layer: "24" },
  { first: 2, last: 2, layer: "23" },
  { first: 3, last: 4, layer: "22" },
  { first: 5, last: 6, layer: "21" },
  { first: 0, last: 1, layer: "18" },
  { first: 0, last: 2, layer: "17" },
  { first: 0, last: 4, layer: "16" },
  { first: 0, last: 6, layer: "15" },
] as const;

const NEXT_FIVE_DAYS = PERIODS[7];
const LAST_FORECAST_DAY = 6;
const DAY_MS = 86_400_000;

export type ForecastWindow = { layer: string; title: string; note: string };

const UNITS = "Inches of rain and melted snow";

function isoDate(ms: number): string {
  return new Date(ms).toISOString().slice(0, 10);
}

function period(start: string, first: number, last: number): string {
  const from = isoDate(Date.parse(`${start}T00:00:00Z`) + first * DAY_MS);
  return fmtDateRange(from, last - first) ?? from;
}

// Picks the forecast period that best fits a trip. forecastStart is the UTC date
// (YYYY-MM-DD) NOAA's current forecast begins on.
export function forecastWindow(startDate: string | null, nights: number | null, forecastStart: string): ForecastWindow {
  const nextFiveDays = (note: string): ForecastWindow => ({ layer: NEXT_FIVE_DAYS.layer, title: "Forecast, next 5 days", note });
  if (!startDate) return nextFiveDays(UNITS);

  const dayOf = (iso: string) => Math.round((Date.parse(`${iso}T00:00:00Z`) - Date.parse(`${forecastStart}T00:00:00Z`)) / DAY_MS);
  const first = dayOf(startDate);
  const last = first + (nights ?? 0);
  if (last < 0) return nextFiveDays(UNITS);
  if (first > LAST_FORECAST_DAY) return nextFiveDays(`Trip forecast opens 7 days before you leave. ${UNITS}.`);

  const wantFirst = Math.max(first, 0);
  const wantLast = Math.min(last, LAST_FORECAST_DAY);
  const best = [...PERIODS]
    .filter((p) => p.first <= wantFirst && p.last >= wantLast)
    .sort((a, b) => a.last - a.first - (b.last - b.first))[0];

  const notes = [UNITS];
  if (best.first < wantFirst || best.last > wantLast) notes.push("Closest period NOAA publishes");
  if (last > LAST_FORECAST_DAY) notes.push(`Forecast ends ${period(forecastStart, LAST_FORECAST_DAY, LAST_FORECAST_DAY)}`);
  return { layer: best.layer, title: `Forecast, ${period(forecastStart, best.first, best.last)}`, note: notes.join(". ") };
}
