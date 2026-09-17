// Shared Open-Meteo helpers used by the trip weather panel and the report
// generator so both format forecasts identically.

export function addDays(iso: string, n: number): string {
  const d = new Date(iso + "T00:00:00Z");
  d.setUTCDate(d.getUTCDate() + n);
  return d.toISOString().slice(0, 10);
}

export function prettyDate(iso: string): string {
  return new Date(iso + "T12:00:00").toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
}

export function clock(iso: string | undefined): string {
  if (!iso) return "—";
  const [h, m] = iso.slice(11, 16).split(":").map(Number);
  const ampm = h < 12 ? "am" : "pm";
  const hr = h % 12 === 0 ? 12 : h % 12;
  return `${hr}:${String(m).padStart(2, "0")} ${ampm}`;
}

// WMO weather codes → glyph + short description (Open-Meteo `weather_code`).
export function describe(code: number | undefined): { icon: string; text: string } {
  switch (code) {
    case 0: return { icon: "☀️", text: "Clear" };
    case 1: return { icon: "🌤️", text: "Mainly clear" };
    case 2: return { icon: "⛅", text: "Partly cloudy" };
    case 3: return { icon: "☁️", text: "Overcast" };
    case 45: case 48: return { icon: "🌫️", text: "Fog" };
    case 51: return { icon: "🌦️", text: "Light drizzle" };
    case 53: return { icon: "🌦️", text: "Drizzle" };
    case 55: return { icon: "🌧️", text: "Heavy drizzle" };
    case 56: case 57: return { icon: "🌧️", text: "Freezing drizzle" };
    case 61: return { icon: "🌦️", text: "Light rain" };
    case 63: return { icon: "🌧️", text: "Rain" };
    case 65: return { icon: "🌧️", text: "Heavy rain" };
    case 66: case 67: return { icon: "🌧️", text: "Freezing rain" };
    case 71: return { icon: "🌨️", text: "Light snow" };
    case 73: return { icon: "🌨️", text: "Snow" };
    case 75: return { icon: "❄️", text: "Heavy snow" };
    case 77: return { icon: "🌨️", text: "Snow grains" };
    case 80: return { icon: "🌦️", text: "Light showers" };
    case 81: return { icon: "🌧️", text: "Showers" };
    case 82: return { icon: "⛈️", text: "Heavy showers" };
    case 85: case 86: return { icon: "🌨️", text: "Snow showers" };
    case 95: return { icon: "⛈️", text: "Thunderstorm" };
    case 96: case 99: return { icon: "⛈️", text: "Thunderstorm, hail" };
    default: return { icon: "•", text: "—" };
  }
}

export type DailyWx = {
  temperature_2m_max: number[];
  temperature_2m_min: number[];
  precipitation_sum: number[];
  wind_speed_10m_max: number[];
  sunrise: string[];
  sunset: string[];
  weather_code: number[];
  precipitation_probability_max?: number[];
};

export type WxResult =
  | { kind: "forecast" | "historical"; day: DailyWx }
  | { kind: "unavailable"; reason: string };

// Typical conditions for a calendar date, averaged from the ERA5 archive over
// recent years — used beyond the ~16-day forecast horizon.
export type Climo = { years: number; hiF: number; loF: number; precipIn: number; precipPct: number; snowIn: number; snowPct: number };

const FORECAST_DAYS = 16;
export function isFarFuture(dateISO: string): boolean {
  const today = new Date().toISOString().slice(0, 10);
  return dateISO >= today && dateISO > new Date(Date.now() + FORECAST_DAYS * 864e5).toISOString().slice(0, 10);
}

const climoCache = new Map<string, Climo | null>();

// Average a ±windowDays window around the target month/day across the last
// yearsBack complete years. No API key; reuses the Open-Meteo archive.
export async function fetchClimatology(
  lat: number | null,
  lon: number | null,
  dateISO: string,
  yearsBack = 10,
  windowDays = 3,
): Promise<Climo | null> {
  if (lat == null || lon == null || !Number.isFinite(lat) || !Number.isFinite(lon)) return null;
  const md = dateISO.slice(5); // MM-DD
  const key = `${lat.toFixed(2)},${lon.toFixed(2)},${md},${yearsBack},${windowDays}`;
  if (climoCache.has(key)) return climoCache.get(key)!;

  const nowYear = new Date().getUTCFullYear();
  const years = Array.from({ length: yearsBack }, (_, i) => nowYear - 1 - i);
  const maxT: number[] = [];
  const minT: number[] = [];
  const precip: number[] = [];
  const snow: number[] = []; // cm
  let contributing = 0;

  await Promise.all(
    years.map(async (y) => {
      const anchor = `${y}-${md}`;
      const start = addDays(anchor, -windowDays);
      const end = addDays(anchor, windowDays);
      try {
        const r = await fetch(
          `https://archive-api.open-meteo.com/v1/archive?latitude=${lat}&longitude=${lon}` +
            `&start_date=${start}&end_date=${end}&daily=temperature_2m_max,temperature_2m_min,precipitation_sum,snowfall_sum` +
            `&temperature_unit=fahrenheit&precipitation_unit=inch&timezone=auto`,
        );
        const j = await r.json();
        const d = j?.daily;
        if (!d?.time?.length) return;
        contributing++;
        for (let i = 0; i < d.time.length; i++) {
          if (d.temperature_2m_max?.[i] != null) maxT.push(d.temperature_2m_max[i]);
          if (d.temperature_2m_min?.[i] != null) minT.push(d.temperature_2m_min[i]);
          if (d.precipitation_sum?.[i] != null) precip.push(d.precipitation_sum[i]);
          if (d.snowfall_sum?.[i] != null) snow.push(d.snowfall_sum[i]);
        }
      } catch {
        /* skip this year */
      }
    }),
  );

  if (!maxT.length || !precip.length) {
    climoCache.set(key, null);
    return null;
  }
  const mean = (a: number[]) => a.reduce((s, x) => s + x, 0) / a.length;
  const climo: Climo = {
    years: contributing,
    hiF: Math.round(mean(maxT)),
    loF: Math.round(mean(minT)),
    precipIn: +mean(precip).toFixed(2),
    precipPct: Math.round((100 * precip.filter((p) => p >= 0.01).length) / precip.length),
    // snowfall_sum is cm; convert the average to inches for display.
    snowIn: snow.length ? +(mean(snow) / 2.54).toFixed(2) : 0,
    snowPct: snow.length ? Math.round((100 * snow.filter((s) => s > 0).length) / snow.length) : 0,
  };
  climoCache.set(key, climo);
  return climo;
}

// Fetch a single day's forecast/archive for one coordinate. Returns a reason
// string instead of throwing when coords are missing or the date is out of range.
export async function fetchDay(
  lat: number | null,
  lon: number | null,
  date: string,
  elevationFt: number | null,
): Promise<WxResult> {
  if (lat == null || lon == null || !Number.isFinite(lat) || !Number.isFinite(lon))
    return { kind: "unavailable", reason: "no coordinates" };
  const today = new Date().toISOString().slice(0, 10);
  const past = date < today;
  if (!past && date > new Date(Date.now() + 16 * 864e5).toISOString().slice(0, 10))
    return { kind: "unavailable", reason: "too far out for forecast" };

  const vars =
    "weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,sunrise,sunset" +
    (past ? "" : ",precipitation_probability_max");
  const base = past ? "https://archive-api.open-meteo.com/v1/archive" : "https://api.open-meteo.com/v1/forecast";
  const elev = elevationFt != null ? `&elevation=${Math.round(elevationFt / 3.28084)}` : "";
  try {
    const r = await fetch(
      `${base}?latitude=${lat}&longitude=${lon}&start_date=${date}&end_date=${date}&daily=${vars}` +
        `&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch&timezone=auto${elev}`,
    );
    const j = await r.json();
    if (!j?.daily?.time?.length) return { kind: "unavailable", reason: "no data" };
    return { kind: past ? "historical" : "forecast", day: j.daily as DailyWx };
  } catch {
    return { kind: "unavailable", reason: "unavailable" };
  }
}
