// Single source of truth for the trip map's look, shared by the interactive
// Leaflet map (components/trip/route-map.tsx) and the baked static renderer
// (lib/topo-map.ts) used for the report and print. Change a color or the
// loop rule here and it applies everywhere. Client-safe (no server deps).

export const MAP_COLORS = {
  route: "#c42e63",
  camp: "#1f7a70",
  campRing: "#1f2937",
  start: "#2e7d32",
  startRing: "#14532d",
  end: "#b3261e",
  endRing: "#7f1d1d",
  slate: "#374151",
} as const;

export function rgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

const COINCIDENT = 3e-4; // ~33 m: close enough to treat as the same spot

// A loop trip: start and end sit on the same spot, so the two endpoint markers
// would cover each other and should collapse into one.
export function isLoopTrack(track: [number, number][]): boolean {
  if (track.length < 2) return false;
  const a = track[0], b = track[track.length - 1];
  return Math.abs(a[0] - b[0]) < COINCIDENT && Math.abs(a[1] - b[1]) < COINCIDENT;
}

// Group items that share a location (within COINCIDENT) so co-located campsites
// collapse into one marker labeled e.g. "1/2". Items missing coords stand alone.
export function groupByLocation<T extends { lat: number | null; lon: number | null }>(items: T[]): T[][] {
  const groups: T[][] = [];
  for (const it of items) {
    if (it.lat == null || it.lon == null) {
      groups.push([it]);
      continue;
    }
    const g = groups.find((grp) => {
      const h = grp[0];
      return h.lat != null && h.lon != null && Math.abs(h.lat - it.lat!) < COINCIDENT && Math.abs(h.lon - it.lon!) < COINCIDENT;
    });
    if (g) g.push(it);
    else groups.push([it]);
  }
  return groups;
}
