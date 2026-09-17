export type ParsedGpx = {
  startLat: number;
  startLon: number;
  distanceMi: number;
  elevationGainFt: number;
  minEleFt: number | null;
  maxEleFt: number | null;
  track: [number, number][];
  profile: { d: number; e: number }[];
  points: { d: number; lat: number; lon: number; e: number }[];
};

const M_TO_FT = 3.28084;
const M_TO_MI = 1 / 1609.344;

function haversineM(a: [number, number], b: [number, number]): number {
  const R = 6371000;
  const dLat = ((b[0] - a[0]) * Math.PI) / 180;
  const dLon = ((b[1] - a[1]) * Math.PI) / 180;
  const la1 = (a[0] * Math.PI) / 180;
  const la2 = (b[0] * Math.PI) / 180;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(la1) * Math.cos(la2) * Math.sin(dLon / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(h));
}

function downsample<T>(arr: T[], max: number): T[] {
  if (arr.length <= max) return arr;
  const step = Math.ceil(arr.length / max);
  const out: T[] = [];
  for (let i = 0; i < arr.length; i += step) out.push(arr[i]);
  if (out[out.length - 1] !== arr[arr.length - 1]) out.push(arr[arr.length - 1]);
  return out;
}

// Parse trackpoints (or route points) into route stats + downsampled map/profile.
export function parseGpx(xml: string): ParsedGpx | null {
  const tag = xml.includes("<trkpt") ? "trkpt" : "rtept";
  const re = new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)<\\/${tag}>|<${tag}\\b[^>]*/>`, "g");
  const pts: { lat: number; lon: number; ele: number | null }[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(xml))) {
    const openTag = m[0].slice(0, m[0].indexOf(">") + 1);
    const lat = Number(/\blat="([-\d.]+)"/.exec(openTag)?.[1]);
    const lon = Number(/\blon="([-\d.]+)"/.exec(openTag)?.[1]);
    if (!Number.isFinite(lat) || !Number.isFinite(lon)) continue;
    const eleStr = m[1] ? /<ele>([-\d.]+)<\/ele>/.exec(m[1])?.[1] : undefined;
    const ele = eleStr != null ? Number(eleStr) : null;
    pts.push({ lat, lon, ele });
  }
  if (pts.length < 2) return null;

  // Elevation gain via hysteresis: only bank a rise once elevation moves at
  // least THRESH from the running reference, so noise and dense sub-metre steps
  // on a steady climb don't get discarded (or double-counted).
  const THRESH_M = 4;
  let distM = 0;
  let gainM = 0;
  let ref: number | null = null;
  let lastEle: number | null = null;
  let minE = Infinity;
  let maxE = -Infinity;
  const full: { d: number; lat: number; lon: number; e: number }[] = [];
  for (let i = 0; i < pts.length; i++) {
    if (i > 0) distM += haversineM([pts[i - 1].lat, pts[i - 1].lon], [pts[i].lat, pts[i].lon]);
    let ele = pts[i].ele;
    if (ele == null) ele = lastEle; // carry forward across missing points
    else lastEle = ele;
    if (ele != null) {
      minE = Math.min(minE, ele);
      maxE = Math.max(maxE, ele);
      if (ref == null) ref = ele;
      else if (Math.abs(ele - ref) >= THRESH_M) {
        if (ele > ref) gainM += ele - ref;
        ref = ele;
      }
    }
    full.push({
      d: +(distM * M_TO_MI).toFixed(3),
      lat: pts[i].lat,
      lon: pts[i].lon,
      e: ele != null ? Math.round(ele * M_TO_FT) : 0,
    });
  }
  const points = downsample(full, 800);

  return {
    startLat: pts[0].lat,
    startLon: pts[0].lon,
    distanceMi: +(distM * M_TO_MI).toFixed(2),
    elevationGainFt: Math.round(gainM * M_TO_FT),
    minEleFt: minE === Infinity ? null : Math.round(minE * M_TO_FT),
    maxEleFt: maxE === -Infinity ? null : Math.round(maxE * M_TO_FT),
    track: points.map((p) => [p.lat, p.lon] as [number, number]),
    profile: points.map((p) => ({ d: p.d, e: p.e })),
    points,
  };
}
