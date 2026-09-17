// Research + output instructions appended after the user's free-text trip
// description to form the copy-pastable planning prompt. Prefers the "gpx-routing"
// MCP tools to build a real, elevation-bearing track; kept in sync with the
// importTrip() schema (the app derives distance + the day split from track +
// campsite positions, so the agent supplies those, not a day plan).
export const PLAN_PROMPT = `---

Using the trip described above, act as a backpacking trip planner. Research it using current, real sources — the managing agency / permit site and trail databases — and build the route geometry with the tools described below. Then output a SINGLE JSON code block and nothing else (no commentary before or after).

Rules:
- Decimal-degree coordinates; ISO dates (YYYY-MM-DD).
- Omit any field you genuinely can't determine rather than guessing.
- Do NOT include a day-by-day breakdown — the app computes each day's distance from where the campsites sit along the route.
- "trip.name": keep it SHORT — just the place/route name (e.g. "Enchanted Valley"). Do NOT append nights, distance, dates, or any descriptors.
- "trip.region": the geographic area only, no state (e.g. "Olympic Peninsula"). Do NOT put a managed-land name or state here.
- "trip.areaType": the GENERIC managed-land designation (e.g. "National Park", "National Forest", "State Forest", "Wilderness", "BLM"). Do NOT use the specific unit name like "Olympic National Park".

BUILDING THE ROUTE (preferred): use the "gpx-routing" MCP tools if available.
- Resolve every named landmark (trailhead, passes, lakes, campsites, junctions) to coordinates with \`geocode_place(query, focusLat?, focusLon?)\`. Always pass a focus point near the trip area — many names are ambiguous across regions. Sanity-check that each hit is actually in the right area; discard far-off matches.
- Call \`route_trail\` with the landmarks as ORDERED waypoints (start → … → end; repeat the start to close a loop), profile "foot-hiking". This snaps the waypoints to real trails and returns a GPX track WITH per-point elevation, plus a summary line giving total distance (mi) and ascent (ft).
    - For waypoints that are off-trail centroids (lake middles, summits), set \`snapRadiusMeters\` to a moderate value (~2000) so they attach to the nearest trail; only use -1 (unbounded) if 2000 fails, and then spot-check that leg.
    - Add enough intermediate on-trail waypoints that the snapped line follows the intended trail (not a shortcut) through junctions.
- From the returned GPX, extract each <trkpt> into the "route.track" array as [lat, lon, ele_m] (include the elevation third value — you have it). Use the tool's reported ascent for "trip.elevationGainFt". Do NOT set "trip.distanceMi" — the app computes it from the track.

BACKUP (only if the gpx-routing MCP is unavailable):
- Provide "route" as EITHER
    - "track": an array of [lat, lon] points that FOLLOW the trail, extracted from OpenStreetMap's way/relation geometry (add a third elevation-in-metres value per point only if you genuinely have it), OR
    - "gpxUrl": a public, directly downloadable .gpx URL (no login).
- Give "trip.elevationGainFt" from trail data (track geometry usually lacks elevation, so the app can't compute gain without it).
- Set "trip.distanceMi" only if you cannot provide a track at all.

Provide, most important first:
- "campsites": the established/designated campsites where you'd sleep, in trail order, one per night, with lat/lon (and eleFt if known).
- "route": as built above.
- "trip.elevationGainFt": total elevation gain in feet.
- Logistics: whether a permit is required and how to get it (agency, quota/lottery, reservation link), the trailhead, driving/access directions, and water availability.

Output JSON matching this shape (example values):
\`\`\`json
{
  "version": 1,
  "trip": {
    "name": "Enchanted Valley",
    "region": "Olympic Peninsula",
    "areaType": "National Park",
    "startDate": "2026-09-12",
    "nights": 3,
    "trailhead": "Graves Creek Trailhead",
    "permitRequired": true,
    "permitNotes": "Wilderness camping permit via recreation.gov; summer quota.",
    "drivingNotes": "US-101 to Lake Quinault, South Shore Rd ~19 mi to Graves Creek.",
    "waterSources": "Quinault River paralleled most of the way; filter/treat.",
    "elevationGainFt": 3200
  },
  "campsites": [
    { "night": 1, "lat": 47.5719, "lon": -123.5261, "eleFt": 900 },
    { "night": 2, "lat": 47.6203, "lon": -123.4801, "eleFt": 2000 },
    { "night": 3, "lat": 47.5719, "lon": -123.5261, "eleFt": 900 }
  ],
  "route": {
    "track": [[47.5145, -123.5807, 180], [47.52, -123.57, 210], [47.6203, -123.4801, 610]],
    "filename": "enchanted-valley.gpx"
  }
}
\`\`\`

Output only the JSON code block.`;
