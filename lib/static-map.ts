// Geoapify Static Maps URL for the trip report: terrain basemap + route line +
// numbered planned-campsite pins + real OSM campsite POIs. Pure builder; the API
// key is injected by the server action that calls it.

export type MapCampsite = { night: number; lat: number | null; lon: number | null };
export type MapPoi = { lat: number; lon: number };

const ROUTE = "c42e63", CAMP = "1f7a70", POI = "a08a4e";
const hash = (c: string) => "%23" + c;

// Thin a dense track to bound the URL length while keeping both endpoints.
function decimate(track: [number, number][], max = 80): [number, number][] {
  if (track.length <= max) return track;
  const step = Math.ceil(track.length / max);
  const out: [number, number][] = [];
  for (let i = 0; i < track.length; i += step) out.push(track[i]);
  const last = track[track.length - 1];
  if (out[out.length - 1] !== last) out.push(last);
  return out;
}

// track is [lat, lon][]; Geoapify geometry/markers take lon,lat order.
export function geoapifyStaticUrl(opts: {
  key: string;
  track: [number, number][];
  campsites: MapCampsite[];
  pois: MapPoi[];
  width?: number;
  height?: number;
}): string | null {
  const { key, track, campsites, pois, width = 800, height = 480 } = opts;
  if (!track.length || !key) return null;

  const poly = decimate(track).map(([la, lo]) => `${lo.toFixed(5)},${la.toFixed(5)}`).join(",");
  const geometry = `polyline:${poly};linecolor:${hash(ROUTE)};linewidth:5`;

  const markers: string[] = [];
  for (const p of pois.slice(0, 40))
    markers.push(`lonlat:${p.lon.toFixed(5)},${p.lat.toFixed(5)};type:circle;color:${hash(POI)};size:small`);
  for (const c of campsites)
    if (c.lat != null && c.lon != null)
      markers.push(`lonlat:${c.lon.toFixed(5)},${c.lat.toFixed(5)};type:material;color:${hash(CAMP)};size:medium;text:${c.night}`);

  const params = [
    "style=osm-bright",
    `width=${width}`,
    `height=${height}`,
    `geometry=${geometry}`,
    markers.length ? `marker=${markers.join("|")}` : "",
    `apiKey=${key}`,
  ].filter(Boolean);
  return `https://maps.geoapify.com/v1/staticmap?${params.join("&")}`;
}
