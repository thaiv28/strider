import "server-only";
import { asc, eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import type { Tx } from "@/lib/trip-propagation";
import { computeDaySegments } from "@/lib/trip";

export async function renumberDays(tripId: number, reader: typeof db | Tx = db) {
  const rows = await reader
    .select({ id: schema.tripDay.id })
    .from(schema.tripDay)
    .where(eq(schema.tripDay.tripId, tripId))
    .orderBy(asc(schema.tripDay.dayNumber), asc(schema.tripDay.id));
  for (let i = 0; i < rows.length; i++) {
    await reader.update(schema.tripDay).set({ dayNumber: i + 1 }).where(eq(schema.tripDay.id, rows[i].id));
  }
}


export async function deriveTripFromDays(tripId: number, reader: typeof db | Tx = db) {
  const rows = await reader
    .select({ distanceMi: schema.tripDay.distanceMi, elevationGainFt: schema.tripDay.elevationGainFt })
    .from(schema.tripDay)
    .where(eq(schema.tripDay.tripId, tripId));
  const anyDist = rows.some((r) => r.distanceMi != null);
  const anyGain = rows.some((r) => r.elevationGainFt != null);
  const distSum = rows.reduce((s, r) => s + (r.distanceMi != null ? Number(r.distanceMi) : 0), 0);
  const gainSum = rows.reduce((s, r) => s + (r.elevationGainFt ?? 0), 0);
  await reader
    .update(schema.trip)
    .set({
      distanceMi: anyDist ? String(+distSum.toFixed(2)) : null,
      elevationGainFt: anyGain ? gainSum : null,
      nights: rows.length ? rows.length - 1 : null,
    })
    .where(eq(schema.trip.id, tripId));
}


export async function syncDaysFromCampsites(tripId: number, reader: typeof db | Tx = db) {
  const [gpx] = await reader
    .select({ points: schema.tripGpx.points })
    .from(schema.tripGpx)
    .where(eq(schema.tripGpx.tripId, tripId));
  const camps = await reader
    .select()
    .from(schema.tripCampsite)
    .where(eq(schema.tripCampsite.tripId, tripId))
    .orderBy(asc(schema.tripCampsite.distanceMi));
  const segs = computeDaySegments(
    gpx?.points ?? [],
    camps.map((c) => ({ distanceMi: Number(c.distanceMi), lat: c.lat != null ? Number(c.lat) : null, lon: c.lon != null ? Number(c.lon) : null })),
  );
  await reader.delete(schema.tripDay).where(eq(schema.tripDay.tripId, tripId));
  if (segs.length)
    await reader.insert(schema.tripDay).values(
      segs.map((s) => ({ tripId, dayNumber: s.day, distanceMi: String(s.distanceMi), elevationGainFt: s.elevationGainFt })),
    );
  await deriveTripFromDays(tripId, reader);
}

