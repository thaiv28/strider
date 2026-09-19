"use server";

import { revalidatePath } from "next/cache";
import { eq, and, asc, max, sql } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { parseGpx } from "@/lib/gpx";
import { computeDaySegments } from "@/lib/trip";
import { computeMealTotals, upsertIngredientFromHit } from "@/lib/food-server";
import { composeTopoMap, type MapCampsite } from "@/lib/topo-map";
import { kcalForGrams } from "@/lib/food";
import type { FoodHit } from "@/lib/food-search";
import { randomBytes } from "node:crypto";

function bump(tripId: number) {
  revalidatePath(`/trips/${tripId}`);
  revalidatePath("/trips");
  revalidatePath("/calendar");
  revalidatePath("/basecamp");
}

export async function createTripShareLink(tripId: number): Promise<string> {
  const userId = await getCurrentUserId();
  const [existing] = await db
    .select({ token: schema.trip.shareToken })
    .from(schema.trip)
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  if (!existing) throw new Error("Trip not found");
  if (existing.token) return existing.token;

  const token = randomBytes(24).toString("base64url");
  await db
    .update(schema.trip)
    .set({ shareToken: token, updatedAt: new Date() })
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  revalidatePath(`/trips/${tripId}`);
  return token;
}

export async function revokeTripShareLink(tripId: number) {
  const userId = await getCurrentUserId();
  await db
    .update(schema.trip)
    .set({ shareToken: null, updatedAt: new Date() })
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  revalidatePath(`/trips/${tripId}`);
}

export async function createTrip(name: string, loadoutId?: number): Promise<number> {
  const userId = await getCurrentUserId();
  const [t] = await db
    .insert(schema.trip)
    .values({ userId, name: name.trim() || "New Trip", status: "planned" })
    .returning({ id: schema.trip.id });
  if (loadoutId) await seedTripFromLoadout(t.id, loadoutId);
  revalidatePath("/trips");
  return t.id;
}

export async function deleteTrip(tripId: number) {
  await db.delete(schema.trip).where(eq(schema.trip.id, tripId));
  revalidatePath("/trips");
  revalidatePath("/calendar");
  revalidatePath("/basecamp");
}

/* eslint-disable @typescript-eslint/no-explicit-any */
function trackToGpxXml(track: any[]): string {
  const pts = track
    .filter((p) => Array.isArray(p) && Number.isFinite(p[0]) && Number.isFinite(p[1]))
    .map((p) => `<trkpt lat="${p[0]}" lon="${p[1]}">${Number.isFinite(p[2]) ? `<ele>${p[2]}</ele>` : ""}</trkpt>`)
    .join("");
  return `<gpx><trk><trkseg>${pts}</trkseg></trk></gpx>`;
}

function nearest(points: { d: number; lat: number; lon: number; e: number }[], lat: number, lon: number) {
  let best = points[0];
  let bd = Infinity;
  for (const p of points) {
    const d = (p.lat - lat) ** 2 + (p.lon - lon) ** 2;
    if (d < bd) { bd = d; best = p; }
  }
  return best;
}

// Create a fully-populated trip from an agent-authored plan file (see the "Plan
// with AI" prompt). Reuses the campsite/day/GPX derivation the manual UI uses.
export async function importTrip(json: string): Promise<{ tripId?: number; error?: string }> {
  // Tolerate agents that wrap the JSON in prose or a ```json fence.
  const fence = json.match(/```(?:json)?\s*([\s\S]*?)```/i);
  let body = (fence ? fence[1] : json).trim();
  const first = body.indexOf("{");
  const last = body.lastIndexOf("}");
  if (first > 0 || last < body.length - 1) body = body.slice(first, last + 1);
  let data: any;
  try {
    data = JSON.parse(body);
  } catch {
    return { error: "Couldn't find valid JSON. Paste the JSON code block the agent produced." };
  }
  const t = data?.trip;
  if (!t || typeof t.name !== "string" || !t.name.trim()) return { error: "The file needs a trip.name field." };

  const userId = await getCurrentUserId();
  const [row] = await db
    .insert(schema.trip)
    .values({
      userId,
      name: t.name.trim().slice(0, 200),
      status: "planned",
      startDate: typeof t.startDate === "string" ? t.startDate : null,
      nights: Number.isFinite(t.nights) ? t.nights : null,
      region: t.region ?? null,
      areaType: t.areaType ?? null,
      trailhead: t.trailhead ?? null,
      permitRequired: typeof t.permitRequired === "boolean" ? t.permitRequired : null,
      permitNotes: t.permitNotes ?? null,
      drivingNotes: t.drivingNotes ?? null,
      waterSources: t.waterSources ?? null,
      distanceMi: t.distanceMi != null ? String(t.distanceMi) : null,
      elevationGainFt: Number.isFinite(t.elevationGainFt) ? Math.round(t.elevationGainFt) : null,
    })
    .returning({ id: schema.trip.id });
  const tripId = row.id;

  const route = data.route ?? {};
  let parsed: ReturnType<typeof parseGpx> = null;
  try {
    if (Array.isArray(route.track) && route.track.length > 1) parsed = parseGpx(trackToGpxXml(route.track));
    else if (typeof route.gpxUrl === "string" && route.gpxUrl.startsWith("http")) {
      const r = await fetch(route.gpxUrl);
      if (r.ok) parsed = parseGpx(await r.text());
    }
  } catch {
    parsed = null;
  }
  if (parsed) {
    await db.insert(schema.tripGpx).values({
      tripId,
      filename: typeof route.filename === "string" ? route.filename : `${t.name.trim()}.gpx`,
      startLat: String(parsed.startLat),
      startLon: String(parsed.startLon),
      distanceMi: String(parsed.distanceMi),
      elevationGainFt: parsed.elevationGainFt,
      minEleFt: parsed.minEleFt,
      maxEleFt: parsed.maxEleFt,
      track: parsed.track,
      profile: parsed.profile,
      points: parsed.points,
    });
  }

  const camps = (Array.isArray(data.campsites) ? data.campsites : []).filter(
    (c: any) => Number.isFinite(c?.lat) && Number.isFinite(c?.lon),
  );
  const totalMi = t.distanceMi != null ? Number(t.distanceMi) : parsed?.distanceMi ?? 0;
  const campRows = camps.map((c: any, i: number) => {
    let distanceMi = 0;
    let eleFt = Number.isFinite(c.eleFt) ? Math.round(c.eleFt) : null;
    if (parsed?.points.length) {
      const p = nearest(parsed.points, c.lat, c.lon);
      distanceMi = p.d;
      if (eleFt == null) eleFt = Math.round(p.e);
    } else {
      distanceMi = totalMi ? ((i + 1) / (camps.length + 1)) * totalMi : i + 1;
    }
    return siteVals(tripId, { distanceMi, lat: c.lat, lon: c.lon, eleFt });
  });
  if (campRows.length) await db.insert(schema.tripCampsite).values(campRows);

  // Derive the day split + distance from the track and campsite positions only
  // when a real track exists; otherwise keep the agent's researched numbers
  // (deriveTripFromDays would null them out with no day rows to sum).
  if (campRows.length && parsed?.points.length) {
    await syncDaysFromCampsites(tripId);
    // Track geometry is usually elevation-less (e.g. OSM), so the computed gain
    // is 0 — fall back to the agent's researched total gain in that case.
    if (Number.isFinite(t.elevationGainFt)) {
      const [cur] = await db.select({ g: schema.trip.elevationGainFt }).from(schema.trip).where(eq(schema.trip.id, tripId));
      if (!cur?.g) await db.update(schema.trip).set({ elevationGainFt: Math.round(t.elevationGainFt) }).where(eq(schema.trip.id, tripId));
    }
  }

  revalidatePath("/trips");
  revalidatePath("/calendar");
  revalidatePath("/basecamp");
  return { tripId };
}

// Copy a loadout's gear into the trip as a frozen snapshot. Only seeds an empty
// trip (copy-once semantics; afterward the trip's gear is edited directly).
export async function seedTripFromLoadout(tripId: number, loadoutId: number) {
  const existing = await db
    .select({ id: schema.tripGear.id })
    .from(schema.tripGear)
    .where(eq(schema.tripGear.tripId, tripId))
    .limit(1);
  if (existing.length) return;

  const members = await db
    .select({
      gearItemId: schema.gearItem.id,
      name: schema.gearItem.name,
      weightG: schema.gearItem.weightG,
      quantity: schema.gearItem.quantity,
      cls: schema.loadoutItem.weightClass,
      category: schema.gearCategory.name,
    })
    .from(schema.loadoutItem)
    .innerJoin(schema.gearItem, eq(schema.loadoutItem.gearItemId, schema.gearItem.id))
    .leftJoin(schema.gearCategory, eq(schema.gearItem.categoryId, schema.gearCategory.id))
    .where(eq(schema.loadoutItem.loadoutId, loadoutId))
    // Preserve the gear-page order: category order, then item order within it.
    .orderBy(asc(schema.gearCategory.sortOrder), asc(schema.gearItem.sortOrder));

  if (members.length)
    await db.insert(schema.tripGear).values(
      members.map((m, i) => ({
        tripId,
        gearItemId: m.gearItemId,
        snapshotName: m.name,
        snapshotWeightG: m.weightG ?? 0,
        snapshotCategory: m.category,
        weightClass: m.cls,
        quantity: m.quantity,
        sortOrder: i,
      })),
    );
  bump(tripId);
}

export async function addTripGear(tripId: number, gearItemId: number) {
  const [g] = await db
    .select({
      name: schema.gearItem.name,
      weightG: schema.gearItem.weightG,
      quantity: schema.gearItem.quantity,
      cls: schema.gearItem.defaultWeightClass,
      category: schema.gearCategory.name,
    })
    .from(schema.gearItem)
    .leftJoin(schema.gearCategory, eq(schema.gearItem.categoryId, schema.gearCategory.id))
    .where(eq(schema.gearItem.id, gearItemId));
  if (!g) return;
  const [{ max }] = await db
    .select({ max: sql<number>`coalesce(max(${schema.tripGear.sortOrder}), -1)::int` })
    .from(schema.tripGear)
    .where(eq(schema.tripGear.tripId, tripId));
  await db
    .insert(schema.tripGear)
    .values({
      tripId,
      gearItemId,
      snapshotName: g.name,
      snapshotWeightG: g.weightG ?? 0,
      snapshotCategory: g.category,
      weightClass: g.cls === "worn" ? "worn" : "base",
      quantity: g.quantity,
      sortOrder: max + 1,
    })
    .onConflictDoNothing({ target: [schema.tripGear.tripId, schema.tripGear.gearItemId] });
  bump(tripId);
}

export async function removeTripGear(id: number, tripId: number) {
  await db.delete(schema.tripGear).where(eq(schema.tripGear.id, id));
  bump(tripId);
}

// Re-insert a previously removed gear item (undo). Appends at the category end.
export async function restoreTripGear(
  tripId: number,
  item: {
    gearItemId: number | null;
    name: string;
    weightG: number;
    category: string | null;
    weightClass: "base" | "worn";
    quantity: number;
    packed: boolean;
  },
) {
  const [{ max }] = await db
    .select({ max: sql<number>`coalesce(max(${schema.tripGear.sortOrder}), -1)::int` })
    .from(schema.tripGear)
    .where(eq(schema.tripGear.tripId, tripId));
  await db
    .insert(schema.tripGear)
    .values({
      tripId,
      gearItemId: item.gearItemId,
      snapshotName: item.name,
      snapshotWeightG: item.weightG,
      snapshotCategory: item.category,
      weightClass: item.weightClass,
      quantity: item.quantity,
      packed: item.packed,
      sortOrder: max + 1,
    })
    .onConflictDoNothing({ target: [schema.tripGear.tripId, schema.tripGear.gearItemId] });
  bump(tripId);
}

export async function setTripGearClass(id: number, tripId: number, cls: "base" | "worn") {
  await db.update(schema.tripGear).set({ weightClass: cls }).where(eq(schema.tripGear.id, id));
  bump(tripId);
}

export async function setTripGearQty(id: number, tripId: number, qty: number) {
  await db
    .update(schema.tripGear)
    .set({ quantity: Math.max(1, Math.round(qty)) })
    .where(eq(schema.tripGear.id, id));
  bump(tripId);
}

export async function setTripGearPacked(id: number, tripId: number, packed: boolean) {
  await db.update(schema.tripGear).set({ packed }).where(eq(schema.tripGear.id, id));
  bump(tripId);
}

export async function clearTripGear(tripId: number) {
  await db.delete(schema.tripGear).where(eq(schema.tripGear.tripId, tripId));
  bump(tripId);
}

// Reorder within a category (cosmetic) or move to another (changes the item's
// snapshot category). orderedIds = the target category's visible order.
export async function moveTripGear(id: number, tripId: number, toCategory: string, orderedIds: number[]) {
  await db.transaction(async (tx) => {
    await tx.update(schema.tripGear).set({ snapshotCategory: toCategory }).where(eq(schema.tripGear.id, id));
    const all = await tx
      .select({ id: schema.tripGear.id })
      .from(schema.tripGear)
      .where(and(eq(schema.tripGear.tripId, tripId), eq(schema.tripGear.snapshotCategory, toCategory)))
      .orderBy(asc(schema.tripGear.sortOrder));
    const seen = new Set(orderedIds);
    const finalOrder = [...orderedIds, ...all.map((a) => a.id).filter((x) => !seen.has(x))];
    for (let i = 0; i < finalOrder.length; i++)
      await tx.update(schema.tripGear).set({ sortOrder: i }).where(eq(schema.tripGear.id, finalOrder[i]));
  });
  bump(tripId);
}

// The day rows are the single source of truth for the itinerary: the trip's
// distance (sum), elevation gain (sum) and nights (days − 1) are re-derived
// from them after every change. Nights is never set directly.
export async function upsertTripDay(
  tripId: number,
  dayNumber: number,
  patch: { distanceMi?: string | null; elevationGainFt?: number | null },
) {
  const [ex] = await db
    .select({ id: schema.tripDay.id })
    .from(schema.tripDay)
    .where(and(eq(schema.tripDay.tripId, tripId), eq(schema.tripDay.dayNumber, dayNumber)))
    .limit(1);
  if (ex) await db.update(schema.tripDay).set(patch).where(eq(schema.tripDay.id, ex.id));
  else await db.insert(schema.tripDay).values({ tripId, dayNumber, ...patch });
  await deriveTripFromDays(tripId);
  bump(tripId);
}

export async function addTripDay(tripId: number) {
  const rows = await db
    .select({ dayNumber: schema.tripDay.dayNumber })
    .from(schema.tripDay)
    .where(eq(schema.tripDay.tripId, tripId));
  const next = rows.reduce((m, r) => Math.max(m, r.dayNumber), 0) + 1;
  await db.insert(schema.tripDay).values({ tripId, dayNumber: next });
  await deriveTripFromDays(tripId);
  bump(tripId);
}

export async function deleteTripDay(tripId: number, id: number) {
  await db.delete(schema.tripDay).where(eq(schema.tripDay.id, id));
  await renumberDays(tripId);
  await deriveTripFromDays(tripId);
  bump(tripId);
}

async function renumberDays(tripId: number) {
  const rows = await db
    .select({ id: schema.tripDay.id })
    .from(schema.tripDay)
    .where(eq(schema.tripDay.tripId, tripId))
    .orderBy(asc(schema.tripDay.dayNumber), asc(schema.tripDay.id));
  for (let i = 0; i < rows.length; i++) {
    await db.update(schema.tripDay).set({ dayNumber: i + 1 }).where(eq(schema.tripDay.id, rows[i].id));
  }
}

async function deriveTripFromDays(tripId: number) {
  const rows = await db
    .select({ distanceMi: schema.tripDay.distanceMi, elevationGainFt: schema.tripDay.elevationGainFt })
    .from(schema.tripDay)
    .where(eq(schema.tripDay.tripId, tripId));
  const anyDist = rows.some((r) => r.distanceMi != null);
  const anyGain = rows.some((r) => r.elevationGainFt != null);
  const distSum = rows.reduce((s, r) => s + (r.distanceMi != null ? Number(r.distanceMi) : 0), 0);
  const gainSum = rows.reduce((s, r) => s + (r.elevationGainFt ?? 0), 0);
  await db
    .update(schema.trip)
    .set({
      distanceMi: anyDist ? String(+distSum.toFixed(2)) : null,
      elevationGainFt: anyGain ? gainSum : null,
      nights: rows.length ? rows.length - 1 : null,
    })
    .where(eq(schema.trip.id, tripId));
}

// Regenerate the day rows from campsite positions (each campsite ends a day,
// plus a final leg to the endpoint). Called whenever campsites change.
async function syncDaysFromCampsites(tripId: number) {
  const [gpx] = await db
    .select({ points: schema.tripGpx.points })
    .from(schema.tripGpx)
    .where(eq(schema.tripGpx.tripId, tripId));
  const camps = await db
    .select()
    .from(schema.tripCampsite)
    .where(eq(schema.tripCampsite.tripId, tripId))
    .orderBy(asc(schema.tripCampsite.distanceMi));
  const segs = computeDaySegments(
    gpx?.points ?? [],
    camps.map((c) => ({ distanceMi: Number(c.distanceMi), lat: c.lat != null ? Number(c.lat) : null, lon: c.lon != null ? Number(c.lon) : null })),
  );
  await db.delete(schema.tripDay).where(eq(schema.tripDay.tripId, tripId));
  if (segs.length)
    await db.insert(schema.tripDay).values(
      segs.map((s) => ({ tripId, dayNumber: s.day, distanceMi: String(s.distanceMi), elevationGainFt: s.elevationGainFt })),
    );
  await deriveTripFromDays(tripId);
}

type Site = { distanceMi: number; lat: number | null; lon: number | null; eleFt: number | null };
const siteVals = (tripId: number, c: Site) => ({
  tripId,
  distanceMi: String(c.distanceMi),
  lat: c.lat != null ? String(c.lat) : null,
  lon: c.lon != null ? String(c.lon) : null,
  eleFt: c.eleFt,
});

export async function addCampsite(tripId: number, c: Site): Promise<number> {
  const [r] = await db
    .insert(schema.tripCampsite)
    .values(siteVals(tripId, c))
    .returning({ id: schema.tripCampsite.id });
  await syncDaysFromCampsites(tripId);
  bump(tripId);
  return r.id;
}

export async function moveCampsite(id: number, tripId: number, c: Site) {
  await db
    .update(schema.tripCampsite)
    .set({
      distanceMi: String(c.distanceMi),
      lat: c.lat != null ? String(c.lat) : null,
      lon: c.lon != null ? String(c.lon) : null,
      eleFt: c.eleFt,
    })
    .where(eq(schema.tripCampsite.id, id));
  await syncDaysFromCampsites(tripId);
  bump(tripId);
}

export async function removeCampsite(id: number, tripId: number) {
  await db.delete(schema.tripCampsite).where(eq(schema.tripCampsite.id, id));
  await syncDaysFromCampsites(tripId);
  bump(tripId);
}

export async function clearCampsites(tripId: number) {
  await db.delete(schema.tripCampsite).where(eq(schema.tripCampsite.tripId, tripId));
  await syncDaysFromCampsites(tripId);
  bump(tripId);
}

type TripPatch = Partial<{
  name: string;
  status: "idea" | "planned" | "completed";
  startDate: string | null;
  nights: number | null;
  partySize: number;
  groceryPeople: number | null;
  groceryRemoved: string[];
  region: string | null;
  areaType: string | null;
  lat: string | null;
  lon: string | null;
  distanceMi: string | null;
  elevationGainFt: number | null;
  foodGPerDay: number;
  waterGPerDay: number;
  fuelGPerDay: number;
  trailhead: string | null;
  permitRequired: boolean | null;
  permitNotes: string | null;
  drivingNotes: string | null;
  waterSources: string | null;
  planningNotes: string | null;
  tripReport: string | null;
}>;

export async function uploadGpx(tripId: number, fd: FormData): Promise<string | null> {
  const f = fd.get("gpx");
  if (!(f instanceof File) || f.size === 0) return "No file selected.";
  const parsed = parseGpx(await f.text());
  if (!parsed) return "Couldn't read track points from that GPX.";
  await db.delete(schema.tripGpx).where(eq(schema.tripGpx.tripId, tripId));
  await db.insert(schema.tripGpx).values({
    tripId,
    filename: f.name,
    startLat: String(parsed.startLat),
    startLon: String(parsed.startLon),
    distanceMi: String(parsed.distanceMi),
    elevationGainFt: parsed.elevationGainFt,
    minEleFt: parsed.minEleFt,
    maxEleFt: parsed.maxEleFt,
    track: parsed.track,
    profile: parsed.profile,
    points: parsed.points,
  });
  bump(tripId);
  return null;
}

export async function removeGpx(tripId: number) {
  await db.delete(schema.tripGpx).where(eq(schema.tripGpx.tripId, tripId));
  bump(tripId);
}

// Re-insert a removed GPX (undo).
export async function restoreGpx(
  tripId: number,
  g: {
    filename: string | null;
    startLat: number | null;
    startLon: number | null;
    distanceMi: number | null;
    elevationGainFt: number | null;
    minEleFt: number | null;
    maxEleFt: number | null;
    track: [number, number][];
    profile: { d: number; e: number }[];
  },
) {
  await db.delete(schema.tripGpx).where(eq(schema.tripGpx.tripId, tripId));
  await db.insert(schema.tripGpx).values({
    tripId,
    filename: g.filename,
    startLat: g.startLat != null ? String(g.startLat) : null,
    startLon: g.startLon != null ? String(g.startLon) : null,
    distanceMi: g.distanceMi != null ? String(g.distanceMi) : null,
    elevationGainFt: g.elevationGainFt,
    minEleFt: g.minEleFt,
    maxEleFt: g.maxEleFt,
    track: g.track,
    profile: g.profile,
  });
  bump(tripId);
}

export async function updateTrip(tripId: number, patch: TripPatch) {
  // Guard the numeric coordinate columns against non-numeric input.
  for (const k of ["lat", "lon"] as const) {
    if (patch[k] != null && !Number.isFinite(Number(patch[k]))) delete patch[k];
  }
  await db
    .update(schema.trip)
    .set({ ...patch, updatedAt: new Date() })
    .where(eq(schema.trip.id, tripId));
  bump(tripId);
}

// Build the report's static map as a self-contained PNG data URI: USGS Topo
// tiles stitched for the route bounds with the route line + campsite pins drawn
// on top. Returns null (no map) when there's no track.
export async function routeMapUrl(track: [number, number][], campsites: MapCampsite[]): Promise<string | null> {
  if (!track.length) return null;
  // Tight frame + higher res so the report map reads zoomed-in and sharp.
  const png = await composeTopoMap({ track, campsites, width: 1000, height: 625, pad: 0.04 });
  return png ? `data:image/png;base64,${png.toString("base64")}` : null;
}

// ---- permit upload (PDF or image, stored inline in the DB) ----

const PERMIT_MAX_BYTES = 15 * 1024 * 1024;
const PERMIT_TYPES = new Set(["application/pdf", "image/png", "image/jpeg", "image/webp", "image/heic"]);

export async function uploadPermit(tripId: number, form: FormData): Promise<{ ok: boolean; error?: string }> {
  const file = form.get("file");
  if (!(file instanceof File) || file.size === 0) return { ok: false, error: "No file." };
  if (!PERMIT_TYPES.has(file.type)) return { ok: false, error: "Use a PDF or image (PNG/JPG/WEBP)." };
  if (file.size > PERMIT_MAX_BYTES) return { ok: false, error: "File exceeds 15 MB." };

  const data = Buffer.from(await file.arrayBuffer());
  const row = { tripId, filename: file.name, mimeType: file.type, sizeBytes: file.size, data, uploadedAt: new Date() };
  await db
    .insert(schema.tripPermit)
    .values(row)
    .onConflictDoUpdate({ target: schema.tripPermit.tripId, set: row });
  bump(tripId);
  return { ok: true };
}

export async function deletePermit(tripId: number) {
  await db.delete(schema.tripPermit).where(eq(schema.tripPermit.tripId, tripId));
  bump(tripId);
}

export async function saveTripPackingList(tripId: number, packingList: string) {
  await db
    .update(schema.trip)
    .set({ packingList, updatedAt: new Date() })
    .where(eq(schema.trip.id, tripId));
  bump(tripId);
}

// ---- trip food plan ----

// Sum of planned food weight drives the trip's food consumable so the weight
// bar reflects the real menu. Uses the same day count as getTripView.
async function syncFoodWeight(tripId: number) {
  const rows = await db.select({ g: schema.tripMeal.snapshotWeightG }).from(schema.tripMeal).where(eq(schema.tripMeal.tripId, tripId));
  const [t] = await db.select({ nights: schema.trip.nights }).from(schema.trip).where(eq(schema.trip.id, tripId));
  const days = Math.max(1, (t?.nights ?? 0) + 1);
  const totalG = rows.reduce((s, r) => s + r.g, 0);
  await db.update(schema.trip).set({ foodGPerDay: Math.round(totalG / days) }).where(eq(schema.trip.id, tripId));
}

type AddFood =
  | { kind: "meal"; mealId: number; servings: number }
  | { kind: "ingredient"; ingredientId: number; grams: number }
  | { kind: "hit"; hit: FoodHit; grams: number };

export async function addTripMeal(tripId: number, dayNumber: number, entry: AddFood) {
  let name: string, kcal: number, weightG: number;
  let servings = 1, mealId: number | null = null, ingredientId: number | null = null;

  if (entry.kind === "meal") {
    const tot = await computeMealTotals(entry.mealId);
    const scale = entry.servings / (tot.baseServings || 1);
    name = tot.name;
    kcal = Math.round(tot.kcal * scale);
    weightG = Math.round(tot.g * scale);
    servings = entry.servings;
    mealId = entry.mealId;
  } else {
    const ing =
      entry.kind === "hit"
        ? await upsertIngredientFromHit(entry.hit)
        : (await db.select().from(schema.ingredient).where(eq(schema.ingredient.id, entry.ingredientId)))[0];
    if (!ing) throw new Error("Ingredient not found");
    const grams = entry.grams;
    name = ing.name;
    kcal = kcalForGrams(ing.kcalPer100g != null ? Number(ing.kcalPer100g) : null, grams);
    weightG = grams;
    ingredientId = ing.id;
  }

  const [{ m } = { m: -1 }] = await db
    .select({ m: max(schema.tripMeal.sortOrder) })
    .from(schema.tripMeal)
    .where(and(eq(schema.tripMeal.tripId, tripId), eq(schema.tripMeal.dayNumber, dayNumber)));
  await db.insert(schema.tripMeal).values({
    tripId,
    dayNumber,
    mealId,
    ingredientId,
    servings: String(servings),
    snapshotName: name,
    snapshotKcal: kcal,
    snapshotWeightG: weightG,
    sortOrder: (m ?? -1) + 1,
  });
  await syncFoodWeight(tripId);
  bump(tripId);
}

// Scale a planned entry: qty is servings for a meal, grams for an ingredient.
export async function setTripMealQty(id: number, tripId: number, qty: number) {
  const [row] = await db.select().from(schema.tripMeal).where(eq(schema.tripMeal.id, id));
  if (!row) return;
  if (row.mealId != null) {
    const old = Number(row.servings) || 1;
    const scale = (qty || 1) / old;
    await db
      .update(schema.tripMeal)
      .set({ servings: String(Math.max(1, qty)), snapshotKcal: Math.round(row.snapshotKcal * scale), snapshotWeightG: Math.round(row.snapshotWeightG * scale) })
      .where(eq(schema.tripMeal.id, id));
  } else {
    const old = row.snapshotWeightG || 1;
    const ratio = Math.max(0, qty) / old;
    await db
      .update(schema.tripMeal)
      .set({ snapshotWeightG: Math.max(0, Math.round(qty)), snapshotKcal: Math.round(row.snapshotKcal * ratio) })
      .where(eq(schema.tripMeal.id, id));
  }
  await syncFoodWeight(tripId);
  bump(tripId);
}

export async function removeTripMeal(id: number, tripId: number) {
  await db.delete(schema.tripMeal).where(eq(schema.tripMeal.id, id));
  await syncFoodWeight(tripId);
  bump(tripId);
}

// Copy a single planned entry to every other day (appended to each day's end).
export async function copyTripMealToAll(id: number, tripId: number) {
  const [row] = await db.select().from(schema.tripMeal).where(eq(schema.tripMeal.id, id));
  if (!row) return;
  const [t] = await db.select({ nights: schema.trip.nights }).from(schema.trip).where(eq(schema.trip.id, tripId));
  const dayRows = await db.select({ dayNumber: schema.tripDay.dayNumber }).from(schema.tripDay).where(eq(schema.tripDay.tripId, tripId));
  const D = Math.max(dayRows.length, (t?.nights ?? 0) + 1, row.dayNumber, 1);
  for (let k = 1; k <= D; k++) {
    if (k === row.dayNumber) continue;
    const [{ m } = { m: -1 }] = await db
      .select({ m: max(schema.tripMeal.sortOrder) })
      .from(schema.tripMeal)
      .where(and(eq(schema.tripMeal.tripId, tripId), eq(schema.tripMeal.dayNumber, k)));
    await db.insert(schema.tripMeal).values({
      tripId, dayNumber: k, mealType: row.mealType, mealId: row.mealId, ingredientId: row.ingredientId,
      servings: row.servings, snapshotName: row.snapshotName, snapshotKcal: row.snapshotKcal, snapshotWeightG: row.snapshotWeightG, sortOrder: (m ?? -1) + 1,
    });
  }
  await syncFoodWeight(tripId);
  bump(tripId);
}
