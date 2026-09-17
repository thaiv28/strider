// Server-side static topo map for the trip report: stitches USGS Topo raster
// tiles for the route's bounds, then draws the route line and numbered campsite
// pins on top — the same basemap the interactive trip map uses, rendered to a
// self-contained PNG so it travels with a copied/downloaded report. Pure JS
// (pngjs) because native image libs won't load on this host's glibc.

import { PNG } from "pngjs";
import jpeg from "jpeg-js";
import { MAP_COLORS, rgb, isLoopTrack, groupByLocation } from "@/lib/map-style";

export type MapCampsite = { night: number; lat: number | null; lon: number | null };

const TILE = 256;
const MAX_ZOOM = 16; // USGS Topo max native zoom
const TILE_URL = (z: number, x: number, y: number) =>
  `https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/tile/${z}/${y}/${x}`;

const ROUTE = rgb(MAP_COLORS.route);
const CAMP = rgb(MAP_COLORS.camp);
const START = rgb(MAP_COLORS.start);
const END = rgb(MAP_COLORS.end);
const SLATE = rgb(MAP_COLORS.slate);

const lonToWorldX = (lon: number, z: number) => ((lon + 180) / 360) * TILE * 2 ** z;
const latToWorldY = (lat: number, z: number) => {
  const r = (lat * Math.PI) / 180;
  return ((1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2) * TILE * 2 ** z;
};

// 3×5 pixel glyphs for the campsite night numbers + the S/E endpoint labels.
const GLYPHS: Record<string, string[]> = {
  "0": ["111", "101", "101", "101", "111"],
  "1": ["010", "110", "010", "010", "111"],
  "2": ["111", "001", "111", "100", "111"],
  "3": ["111", "001", "111", "001", "111"],
  "4": ["101", "101", "111", "001", "001"],
  "5": ["111", "100", "111", "001", "111"],
  "6": ["111", "100", "111", "101", "111"],
  "7": ["111", "001", "010", "010", "010"],
  "8": ["111", "101", "111", "101", "111"],
  "9": ["111", "101", "111", "001", "111"],
  S: ["111", "100", "111", "001", "111"],
  E: ["111", "100", "110", "100", "111"],
  "/": ["001", "001", "010", "100", "100"],
};

export async function composeTopoMap(opts: {
  track: [number, number][];
  campsites: MapCampsite[];
  width?: number;
  height?: number;
  pad?: number; // fraction of bbox added as margin; smaller → tighter zoom
  rotate?: boolean; // rotate the finished image 90° clockwise
}): Promise<Buffer | null> {
  const { track, campsites, width = 800, height = 500, pad = 0.12, rotate = false } = opts;
  if (!track.length) return null;

  const camps = campsites.filter((c) => c.lat != null && c.lon != null) as { night: number; lat: number; lon: number }[];
  const lats = [...track.map((p) => p[0]), ...camps.map((c) => c.lat)];
  const lons = [...track.map((p) => p[1]), ...camps.map((c) => c.lon)];
  let minLat = Math.min(...lats), maxLat = Math.max(...lats);
  let minLon = Math.min(...lons), maxLon = Math.max(...lons);
  const latPad = (maxLat - minLat) * pad || 0.01;
  const lonPad = (maxLon - minLon) * pad || 0.01;
  minLat -= latPad; maxLat += latPad; minLon -= lonPad; maxLon += lonPad;

  let z = MAX_ZOOM;
  for (; z >= 1; z--) {
    const spanX = lonToWorldX(maxLon, z) - lonToWorldX(minLon, z);
    const spanY = latToWorldY(minLat, z) - latToWorldY(maxLat, z);
    if (spanX <= width && spanY <= height) break;
  }

  const cx = (lonToWorldX(minLon, z) + lonToWorldX(maxLon, z)) / 2;
  const cy = (latToWorldY(minLat, z) + latToWorldY(maxLat, z)) / 2;
  const originX = Math.round(cx - width / 2);
  const originY = Math.round(cy - height / 2);

  const out = new PNG({ width, height });
  out.data.fill(0xff); // white backing for any tiles that fail to load

  const nTiles = 2 ** z;
  const tx0 = Math.floor(originX / TILE), tx1 = Math.floor((originX + width) / TILE);
  const ty0 = Math.floor(originY / TILE), ty1 = Math.floor((originY + height) / TILE);
  const jobs: Promise<void>[] = [];
  for (let tx = tx0; tx <= tx1; tx++) {
    for (let ty = ty0; ty <= ty1; ty++) {
      if (tx < 0 || ty < 0 || tx >= nTiles || ty >= nTiles) continue;
      jobs.push(blitTile(out, tx, ty, z, originX, originY));
    }
  }
  await Promise.all(jobs);

  const toPx = (lat: number, lon: number): [number, number] => [lonToWorldX(lon, z) - originX, latToWorldY(lat, z) - originY];

  // Scale line/markers with the output size so a high-res print map keeps the
  // same visual proportions as the 800-wide report map.
  const k = Math.max(1, Math.round(width / 800));

  for (let i = 1; i < track.length; i++) {
    const [x0, y0] = toPx(track[i - 1][0], track[i - 1][1]);
    const [x1, y1] = toPx(track[i][0], track[i][1]);
    thickLine(out, x0, y0, x1, y1, 2 * k, ROUTE);
  }

  for (const grp of groupByLocation(camps)) {
    const [x, y] = toPx(grp[0].lat, grp[0].lon);
    marker(out, x, y, CAMP, grp.map((c) => c.night).join("/"), k);
  }

  // Endpoints drawn last so they stay visible even if a campsite overlaps them.
  // A loop trip (start ≈ end) collapses to one combined S/E marker.
  const [sx, sy] = toPx(track[0][0], track[0][1]);
  const end = track[track.length - 1];
  const [ex, ey] = toPx(end[0], end[1]);
  if (isLoopTrack(track)) {
    markerCombined(out, (sx + ex) / 2, (sy + ey) / 2, k);
  } else {
    marker(out, sx, sy, START, "S", k);
    marker(out, ex, ey, END, "E", k);
  }

  return PNG.sync.write(rotate ? rotate90(out) : out);
}

// Rotate 90° clockwise into a new PNG (width/height swap). Used so the print
// page can show a landscape map sideways on a portrait sheet without fragile
// CSS transforms.
function rotate90(src: PNG): PNG {
  const dst = new PNG({ width: src.height, height: src.width });
  for (let y = 0; y < src.height; y++)
    for (let x = 0; x < src.width; x++) {
      const nx = src.height - 1 - y, ny = x;
      const s = (y * src.width + x) * 4, d = (ny * dst.width + nx) * 4;
      dst.data[d] = src.data[s];
      dst.data[d + 1] = src.data[s + 1];
      dst.data[d + 2] = src.data[s + 2];
      dst.data[d + 3] = 255;
    }
  return dst;
}

// USGS serves these tiles as JPEG, but tolerate PNG too (magic-byte sniff).
function decodeTile(buf: Buffer): { width: number; height: number; data: Uint8Array | Buffer } | null {
  if (buf[0] === 0x89 && buf[1] === 0x50) return PNG.sync.read(buf);
  if (buf[0] === 0xff && buf[1] === 0xd8) return jpeg.decode(buf, { formatAsRGBA: true, useTArray: true });
  return null;
}

async function blitTile(out: PNG, tx: number, ty: number, z: number, originX: number, originY: number): Promise<void> {
  let tile: { width: number; height: number; data: Uint8Array | Buffer } | null;
  try {
    const res = await fetch(TILE_URL(z, tx, ty));
    if (!res.ok) return;
    tile = decodeTile(Buffer.from(await res.arrayBuffer()));
  } catch {
    return;
  }
  if (!tile) return;
  const dx = tx * TILE - originX, dy = ty * TILE - originY;
  for (let y = 0; y < tile.height; y++) {
    const oy = dy + y;
    if (oy < 0 || oy >= out.height) continue;
    for (let x = 0; x < tile.width; x++) {
      const ox = dx + x;
      if (ox < 0 || ox >= out.width) continue;
      const s = (y * tile.width + x) * 4, d = (oy * out.width + ox) * 4;
      out.data[d] = tile.data[s];
      out.data[d + 1] = tile.data[s + 1];
      out.data[d + 2] = tile.data[s + 2];
      out.data[d + 3] = 255;
    }
  }
}

function setPx(out: PNG, x: number, y: number, [r, g, b]: number[]): void {
  x = Math.round(x); y = Math.round(y);
  if (x < 0 || y < 0 || x >= out.width || y >= out.height) return;
  const d = (y * out.width + x) * 4;
  out.data[d] = r; out.data[d + 1] = g; out.data[d + 2] = b; out.data[d + 3] = 255;
}

function disc(out: PNG, cx: number, cy: number, rad: number, color: number[]): void {
  for (let dy = -rad; dy <= rad; dy++)
    for (let dx = -rad; dx <= rad; dx++)
      if (dx * dx + dy * dy <= rad * rad) setPx(out, cx + dx, cy + dy, color);
}

// Stamp a disc of radius `rad` along the segment for a rounded, anti-gap line.
function thickLine(out: PNG, x0: number, y0: number, x1: number, y1: number, rad: number, color: number[]): void {
  const steps = Math.max(1, Math.ceil(Math.hypot(x1 - x0, y1 - y0)));
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    disc(out, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, rad, color);
  }
}

// A point dot plus a legible label chip (white box, colored border + text)
// above it — the label sits off the marker so it stays readable over terrain.
// `k` scales everything so a high-res map keeps the report map's proportions.
function marker(out: PNG, x: number, y: number, color: number[], label: string, k: number): void {
  disc(out, x, y, 6 * k, [255, 255, 255]);
  disc(out, x, y, 4 * k, color);
  chip(out, x, y, label, color, () => color, k);
}

// Loop trip: start ≈ end. One dot (green fill, red ring) with an "S/E" chip.
function markerCombined(out: PNG, x: number, y: number, k: number): void {
  disc(out, x, y, 6 * k, [255, 255, 255]);
  disc(out, x, y, 5 * k, END);
  disc(out, x, y, 4 * k, START);
  chip(out, x, y, "S/E", SLATE, (ch) => (ch === "S" ? START : ch === "E" ? END : SLATE), k);
}

// Draw the label chip above the dot at (x,y); `border` colors the outline and
// `colorOf` picks each glyph's color (so "S/E" can be green/red).
function chip(out: PNG, x: number, y: number, label: string, border: number[], colorOf: (ch: string) => number[], k: number): void {
  const s = 3 * k, gw = 3 * s, gh = 5 * s, gap = s, padX = 5 * k, padY = 4 * k, dot = 6 * k;
  const textW = label.length * gw + (label.length - 1) * gap;
  const w = textW + padX * 2, h = gh + padY * 2, r = 4 * k;
  const left = Math.round(x - w / 2);
  let top = Math.round(y - dot - 4 * k - h); // above the dot
  if (top < 1) top = Math.round(y + dot + 4 * k); // flip below if it would clip the top edge
  roundRect(out, left - k, top - k, w + 2 * k, h + 2 * k, r + k, border);
  roundRect(out, left, top, w, h, r, [255, 255, 255]);
  drawLabel(out, x, top + h / 2, label, s, colorOf);
}

function roundRect(out: PNG, x: number, y: number, w: number, h: number, r: number, color: number[]): void {
  for (let dy = 0; dy < h; dy++)
    for (let dx = 0; dx < w; dx++) {
      const outCorner = (cx: number, cy: number) => (dx - cx) ** 2 + (dy - cy) ** 2 > r * r;
      if (dx < r && dy < r && outCorner(r, r)) continue;
      if (dx >= w - r && dy < r && outCorner(w - r - 1, r)) continue;
      if (dx < r && dy >= h - r && outCorner(r, h - r - 1)) continue;
      if (dx >= w - r && dy >= h - r && outCorner(w - r - 1, h - r - 1)) continue;
      setPx(out, x + dx, y + dy, color);
    }
}

// Center `text` (glyphs only) at (cx,cy), scaled by `s`; `colorOf` picks each
// glyph's color.
function drawLabel(out: PNG, cx: number, cy: number, text: string, s: number, colorOf: (ch: string) => number[]): void {
  const gw = 3 * s, gh = 5 * s, gap = s;
  const total = text.length * gw + (text.length - 1) * gap;
  let x = cx - total / 2;
  for (const ch of text) {
    const glyph = GLYPHS[ch];
    if (glyph) {
      const color = colorOf(ch);
      for (let r = 0; r < 5; r++)
        for (let cc = 0; cc < 3; cc++)
          if (glyph[r][cc] === "1")
            for (let sy = 0; sy < s; sy++)
              for (let sx = 0; sx < s; sx++)
                setPx(out, x + cc * s + sx, cy - gh / 2 + r * s + sy, color);
    }
    x += gw + gap;
  }
}
