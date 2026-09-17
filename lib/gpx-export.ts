// Build a device-ready GPX for handoff to Garmin Explore (which syncs it to an
// inReach). Emits the route as a single track plus named waypoints for the
// start/end and each campsite, merging co-located points the same way the maps
// do so "Night 1/2" reads as one waypoint. Client-safe (no server deps).

import { groupByLocation, isLoopTrack } from "@/lib/map-style";

export type ExportPoint = { lat: number; lon: number; e: number }; // e in feet
export type ExportCampsite = { night: number; lat: number | null; lon: number | null; eleFt: number | null };

const FT_TO_M = 1 / 3.28084;

const esc = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const c = (n: number) => n.toFixed(6);

function wpt(lat: number, lon: number, name: string, sym: string, eleFt: number | null): string {
  const ele = eleFt != null ? `<ele>${(eleFt * FT_TO_M).toFixed(1)}</ele>` : "";
  return `  <wpt lat="${c(lat)}" lon="${c(lon)}">${ele}<name>${esc(name)}</name><sym>${sym}</sym></wpt>`;
}

// A safe .gpx filename derived from the trip name.
export function gpxFilename(tripName: string): string {
  const base = tripName.trim().replace(/[^\w.-]+/g, "-").replace(/^-+|-+$/g, "") || "route";
  return `${base}.gpx`;
}

export function buildGpx(tripName: string, points: ExportPoint[], campsites: ExportCampsite[]): string {
  const track = points.map((p) => [p.lat, p.lon] as [number, number]);
  const loop = isLoopTrack(track);

  const wpts: string[] = [];
  const first = points[0];
  const last = points[points.length - 1];
  if (first) {
    if (loop && last) {
      wpts.push(wpt(first.lat, first.lon, "Start / End", "Trail Head", first.e));
    } else {
      wpts.push(wpt(first.lat, first.lon, "Start", "Trail Head", first.e));
      if (last) wpts.push(wpt(last.lat, last.lon, "End", "Flag, Red", last.e));
    }
  }
  const camps = campsites.filter((s) => s.lat != null && s.lon != null);
  for (const grp of groupByLocation(camps)) {
    const s = grp[0];
    const nights = grp.map((g) => g.night).join("/");
    const label = `Night${grp.length > 1 ? "s" : ""} ${nights}`;
    wpts.push(wpt(s.lat!, s.lon!, label, "Campground", s.eleFt));
  }

  const trkpts = points
    .map((p) => `      <trkpt lat="${c(p.lat)}" lon="${c(p.lon)}"><ele>${(p.e * FT_TO_M).toFixed(1)}</ele></trkpt>`)
    .join("\n");

  return `<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Backpack" xmlns="http://www.topografix.com/GPX/1/1">
  <metadata><name>${esc(tripName)}</name></metadata>
${wpts.join("\n")}
  <trk><name>${esc(tripName)}</name><trkseg>
${trkpts}
  </trkseg></trk>
</gpx>
`;
}
