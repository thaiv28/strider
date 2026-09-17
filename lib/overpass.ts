// Real-world campsite POIs from OpenStreetMap via the Overpass API. Coverage of
// backcountry/dispersed sites varies by area; developed sites are well tagged.

export type CampPoi = { lat: number; lon: number; name: string | null; kind: string };
export type Bbox = [south: number, west: number, north: number, east: number];

export function trackBbox(track: [number, number][], padDeg = 0.01): Bbox | null {
  if (!track.length) return null;
  let s = 90, w = 180, n = -90, e = -180;
  for (const [la, lo] of track) {
    if (la < s) s = la;
    if (la > n) n = la;
    if (lo < w) w = lo;
    if (lo > e) e = lo;
  }
  return [s - padDeg, w - padDeg, n + padDeg, e + padDeg];
}

const MAX_POIS = 60;

// The public overpass-api.de endpoint frequently queues requests; racing mirrors
// and taking the first success cuts the common multi-second lead-in.
const MIRRORS = [
  "https://overpass-api.de/api/interpreter",
  "https://overpass.kumi.systems/api/interpreter",
  "https://overpass.private.coffee/api/interpreter",
];

const cache = new Map<string, CampPoi[]>();
const cacheKey = (b: Bbox) => b.map((n) => n.toFixed(2)).join(",");

// localStorage survives page refresh (the module cache does not). Keyed by bbox
// with a TTL, since POI coverage changes slowly.
const LS_PREFIX = "pois:";
const LS_TTL_MS = 7 * 864e5;

function lsGet(key: string): CampPoi[] | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(LS_PREFIX + key);
    if (!raw) return null;
    const { at, pois } = JSON.parse(raw) as { at: number; pois: CampPoi[] };
    if (Date.now() - at > LS_TTL_MS) return null;
    return pois;
  } catch {
    return null;
  }
}

function lsSet(key: string, pois: CampPoi[]): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(LS_PREFIX + key, JSON.stringify({ at: Date.now(), pois }));
  } catch {
    /* quota or disabled — non-fatal */
  }
}

export async function fetchCampsitePois(bbox: Bbox): Promise<CampPoi[]> {
  const key = cacheKey(bbox);
  const hit = cache.get(key) ?? lsGet(key);
  if (hit) {
    cache.set(key, hit);
    return hit;
  }

  const [s, w, n, e] = bbox;
  const box = `(${s},${w},${n},${e})`;
  const q =
    `[out:json][timeout:25];(` +
    `node["tourism"="camp_site"]${box};` +
    `node["tourism"="wilderness_hut"]${box};` +
    `node["tourism"="camp_pitch"]${box};` +
    `);out body ${MAX_POIS};`;
  const body = "data=" + encodeURIComponent(q);

  try {
    // Promise.any resolves on the first mirror that returns 2xx JSON.
    const j = await Promise.any(
      MIRRORS.map(async (url) => {
        const r = await fetch(url, { method: "POST", body });
        if (!r.ok) throw new Error(String(r.status));
        return r.json();
      }),
    );
    const pois: CampPoi[] = (j.elements ?? [])
      .filter((el: { lat?: number; lon?: number }) => el.lat != null && el.lon != null)
      .map((el: { lat: number; lon: number; tags?: Record<string, string> }) => ({
        lat: el.lat,
        lon: el.lon,
        name: el.tags?.name ?? null,
        kind: el.tags?.tourism ?? "camp_site",
      }))
      .slice(0, MAX_POIS);
    cache.set(key, pois);
    lsSet(key, pois);
    return pois;
  } catch {
    return [];
  }
}
