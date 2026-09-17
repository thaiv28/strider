import { marked } from "marked";
import { gToOz } from "@/lib/util";
import { addDays, prettyDate, describe, type WxResult, type Climo } from "@/lib/weather";
import type { TripFood, ShoppingItem } from "@/lib/trip";
import { groupByLocation } from "@/lib/map-style";

// ── Templating ──────────────────────────────────────────────────────────────
// The report is a user-editable Markdown document with {{token}} placeholders.
// Scalar tokens fill inline (an empty one drops its whole line). Block tokens
// expand to multi-line Markdown and can be toggled off per-generation; an empty
// block also drops the section heading that immediately precedes it.

export const BLOCK_TOKENS = [
  { token: "route_map", label: "Route map (image)" },
  { token: "itinerary", label: "Day-by-day itinerary" },
  { token: "route", label: "Campsites" },
  { token: "weather", label: "Weather forecast" },
  { token: "food_plan", label: "Food plan" },
  { token: "shopping_list", label: "Grocery list (checkable)" },
  { token: "packing_list", label: "What to pack" },
] as const;
export type BlockToken = (typeof BLOCK_TOKENS)[number]["token"];
const BLOCK_SET = new Set<string>(BLOCK_TOKENS.map((b) => b.token));

export const DEFAULT_TEMPLATE = `# {{name}}

**When:** {{start_date}} – {{end_date}} · {{nights}} nights
**Where:** {{region}} · {{area_type}}
**Distance:** {{distance}} mi · {{gain}} ft gain
**Trailhead:** {{trailhead}}
_Prepared {{generated_on}}_

## Map
{{route_map}}

## Itinerary
{{itinerary}}

## Campsites
{{route}}

## Getting there
{{driving}}

## Permits
{{permits}}

## Water
{{water_sources}}

## Weather
{{weather}}

## Food
{{food_plan}}

## What to pack
{{packing_list}}

## Notes
{{planning_notes}}

## Grocery list
{{shopping_list}}
`;

export const DEFAULT_PACKING = `**Shelter & sleep**
- Tent or share (let me know)
- Sleeping bag rated to 20°F
- Sleeping pad

**Cooking & water**
- Stove + fuel
- Pot, spork, mug
- Water filter or treatment
- 2 L water capacity

**Wear & carry**
- 40–60 L pack
- Rain shell + insulating layer
- Headlamp + spare batteries
- First-aid & blister kit
`;

// Which block tokens actually appear in the template (drives the checkboxes).
export function templateBlocks(template: string): { token: BlockToken; label: string }[] {
  const present = new Set<string>();
  for (const m of template.matchAll(/\{\{(\w+)\}\}/g)) if (BLOCK_SET.has(m[1])) present.add(m[1]);
  return BLOCK_TOKENS.filter((b) => present.has(b.token));
}

// ── Data → block Markdown ────────────────────────────────────────────────────

export type ReportData = {
  name: string;
  region: string | null;
  areaType: string | null;
  startDate: string | null;
  nights: number | null;
  days: number;
  distanceMi: number | null;
  gainFt: number | null;
  trailhead: string | null;
  permits: string | null;
  waterSources: string | null;
  driving: string | null;
  planningNotes: string | null;
  dayBreakdown: { dayNumber: number; distanceMi: string | null; elevationGainFt: number | null }[];
  perDayAuto: { day: number; lat: number | null; lon: number | null }[];
  gpx: { distanceMi: number | null; elevationGainFt: number | null; minEleFt: number | null; maxEleFt: number | null } | null;
  campsites: { night: number; distanceMi: number; lat: number | null; lon: number | null }[];
  food: TripFood;
  shopping: ShoppingItem[];
  waterG: number;
  fuelG: number;
  packingList: string;
  // Full route for the static map image ([lat, lon] points).
  track: [number, number][];
  // Weather fallback coords + elevation when there are no campsites.
  lat: number | null;
  lon: number | null;
  elevationFt: number | null;
};

const coord = (lat: number | null, lon: number | null) =>
  lat != null && lon != null ? `${lat.toFixed(4)}, ${lon.toFixed(4)}` : "";

export function renderItinerary(d: ReportData): string {
  if (!d.dayBreakdown.length) return "";
  const auto = new Map(d.perDayAuto.map((a) => [a.day, a]));
  const rows = d.dayBreakdown
    .slice()
    .sort((a, b) => a.dayNumber - b.dayNumber)
    .map((r) => {
      const a = auto.get(r.dayNumber);
      const mi = r.distanceMi ? `${r.distanceMi} mi` : "—";
      const gain = r.elevationGainFt != null ? `${r.elevationGainFt.toLocaleString()} ft` : "—";
      return `| ${r.dayNumber} | ${mi} | ${gain} | ${coord(a?.lat ?? null, a?.lon ?? null) || "—"} |`;
    });
  return ["| Day | Distance | Gain | Camp (lat, lon) |", "|---|---|---|---|", ...rows].join("\n");
}

// Just the campsites — the route's distance/elevation stats live in the header
// and itinerary, so they're not repeated here. Co-located nights merge (e.g.
// "Nights 1/2") to match the map markers.
export function renderRoute(d: ReportData): string {
  if (!d.campsites.length) return "";
  const mi = (v: number) => +v.toFixed(2); // cap at two decimals
  return groupByLocation(d.campsites)
    .map((grp) => {
      const c = grp[0];
      const nights = grp.map((g) => g.night).join("/");
      return `- Night${grp.length > 1 ? "s" : ""} ${nights} — ${mi(c.distanceMi)} mi in — ${coord(c.lat, c.lon) || "coords TBD"}`;
    })
    .join("\n");
}

// Weather is fetched in the client (per day/coord) and passed in already-resolved.
export type WxRow = { label: string; date: string; result: WxResult | { kind: "climatology"; climo: Climo } };
export function renderWeather(rows: WxRow[]): string {
  if (!rows.length) return "";
  const head = ["| Day | Weather | High / Low | Precip | Wind |", "|---|---|---|---|---|"];
  const body = rows.map((r) => {
    const when = `${r.label} (${prettyDate(r.date)})`;
    if (r.result.kind === "unavailable") return `| ${when} | ${r.result.reason} | — | — | — |`;
    if (r.result.kind === "climatology") {
      const c = r.result.climo;
      const snow = c.snowPct > 0 ? `, ❄️ ${c.snowPct}% snow days` : "";
      return `| ${when} | ${c.snowPct >= 20 ? "❄️" : "📅"} typical (${c.years}-yr avg) | ${c.hiF}° / ${c.loF}° | ${c.precipIn.toFixed(2)}″, ${c.precipPct}% wet${snow} | — |`;
    }
    const w = r.result.day;
    const wx = describe(w.weather_code?.[0]);
    const hi = Math.round(w.temperature_2m_max[0]);
    const lo = Math.round(w.temperature_2m_min[0]);
    const precip = w.precipitation_sum[0] ?? 0;
    const pop = w.precipitation_probability_max?.[0];
    const precipCell = precip > 0 || pop != null ? `${precip.toFixed(2)}″${pop != null ? ` (${pop}%)` : ""}` : "—";
    const windCell = w.wind_speed_10m_max?.[0] != null ? `${Math.round(w.wind_speed_10m_max[0])} mph` : "—";
    return `| ${when} | ${wx.icon} ${wx.text} | ${hi}° / ${lo}° | ${precipCell} | ${windCell} |`;
  });
  return [...head, ...body].join("\n");
}

export function renderFoodPlan(d: ReportData): string {
  const f = d.food;
  const days = f.days
    .filter((day) => day.entries.length)
    .map((day) => {
      const lines = day.entries.map((e) => `- ${e.name}${e.isMeal && e.servings !== 1 ? ` ×${e.servings}` : ""}`);
      return [`**Day ${day.dayNumber}**`, ...lines].join("\n");
    });
  return days.join("\n\n").trim();
}

// Checkable Markdown (GFM task list) so it can be ticked off in a doc or notes app.
export function renderShoppingList(d: ReportData): string {
  if (!d.shopping.length) return "";
  return d.shopping.map((it) => `- [ ] ${it.name} — ${it.grams} g (${gToOz(it.grams).toFixed(1)} oz)`).join("\n");
}

// ── Assembly ─────────────────────────────────────────────────────────────────

export function buildScalars(d: ReportData): Record<string, string> {
  const end = d.startDate && d.nights != null ? addDays(d.startDate, d.nights) : null;
  const fmt = (iso: string | null) => (iso ? prettyDate(iso) : "");
  return {
    name: d.name ?? "",
    region: d.region ?? "",
    area_type: d.areaType ?? "",
    start_date: fmt(d.startDate),
    end_date: fmt(end),
    nights: d.nights != null ? String(d.nights) : "",
    days: String(d.days),
    distance: d.distanceMi != null ? String(d.distanceMi) : "",
    gain: d.gainFt != null ? d.gainFt.toLocaleString() : "",
    trailhead: d.trailhead ?? "",
    permits: d.permits ?? "",
    water_sources: d.waterSources ?? "",
    driving: d.driving ?? "",
    planning_notes: d.planningNotes ?? "",
    generated_on: prettyDate(new Date().toISOString().slice(0, 10)),
  };
}

// Substitute tokens, drop lines whose every token is empty, then remove headings
// left with no content and collapse the resulting blank runs.
export function renderReport(template: string, values: Record<string, string>, blocks: Record<string, string>): string {
  const resolve = (k: string) => (k in blocks ? blocks[k] : values[k] ?? "");
  const lines = template.split("\n");
  const out: string[] = [];
  for (const line of lines) {
    const tokens = [...line.matchAll(/\{\{(\w+)\}\}/g)];
    if (!tokens.length) {
      out.push(line);
      continue;
    }
    if (tokens.every((t) => resolve(t[1]).trim() === "")) continue; // whole line was just empty token(s)
    out.push(line.replace(/\{\{(\w+)\}\}/g, (_, k) => resolve(k)));
  }

  const isHeading = (s: string) => /^#{1,6}\s/.test(s.trim());
  const kept: string[] = [];
  for (let i = 0; i < out.length; i++) {
    if (isHeading(out[i])) {
      let j = i + 1;
      while (j < out.length && out[j].trim() === "") j++;
      if (j >= out.length || isHeading(out[j])) continue; // empty section — drop heading
    }
    kept.push(out[i]);
  }

  return kept
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/([^\n])\n(#{1,6} )/g, "$1\n\n$2") // blank line before every heading
    .trim();
}

export function markdownToHtml(md: string): string {
  return marked.parse(md, { async: false, gfm: true, breaks: true }) as string;
}
