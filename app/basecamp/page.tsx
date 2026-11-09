import { eq, asc } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId, getDefaultBaseBreakdown } from "@/lib/gear";
import { effectiveRouteStats } from "@/lib/trip";
import { HomeView, type TripCard, type NextReady } from "@/components/home/home-view";

export const dynamic = "force-dynamic";
export const metadata = { title: { absolute: "Basecamp · Strider" } };

export default async function BasecampPage() {
  const userId = await getCurrentUserId();
  const [rows, base] = await Promise.all([
    db
      .select({
        id: schema.trip.id,
        name: schema.trip.name,
        status: schema.trip.status,
        startDate: schema.trip.startDate,
        nights: schema.trip.nights,
        region: schema.trip.region,
        trailRegion: schema.trail.region,
        areaType: schema.trip.areaType,
        trailAreaType: schema.trail.areaType,
        distanceMi: schema.trip.distanceMi,
        elevationGainFt: schema.trip.elevationGainFt,
        gpxDistanceMi: schema.tripGpx.distanceMi,
        gpxElevationGainFt: schema.tripGpx.elevationGainFt,
        typicalDistanceMi: schema.trail.typicalDistanceMi,
        typicalElevationFt: schema.trail.typicalElevationFt,
        track: schema.tripGpx.track,
      })
      .from(schema.trip)
      .leftJoin(schema.trail, eq(schema.trip.trailId, schema.trail.id))
      .leftJoin(schema.tripGpx, eq(schema.tripGpx.tripId, schema.trip.id))
      .where(eq(schema.trip.userId, userId))
      .orderBy(asc(schema.trip.startDate)),
    getDefaultBaseBreakdown(userId),
  ]);

  const trips: TripCard[] = rows.map((t) => {
    const route = effectiveRouteStats(
      t,
      { distanceMi: t.gpxDistanceMi, elevationGainFt: t.gpxElevationGainFt },
      t,
    );
    return {
      id: t.id,
      name: t.name,
      status: t.status,
      startDate: t.startDate,
      nights: t.nights,
      region: t.region ?? t.trailRegion ?? null,
      areaType: t.areaType ?? t.trailAreaType ?? null,
      distanceMi: route.distanceMi ?? 0,
      elevationFt: route.elevationFt,
      track: t.track ?? null,
    };
  });

  // Readiness chips for the next planned objective reflect what's actually attached.
  const next = trips.find((t) => t.status === "planned");
  let nextReady: NextReady | undefined;
  if (next) {
    const [gear, meals] = await Promise.all([
      db.select({ id: schema.tripGear.id }).from(schema.tripGear).where(eq(schema.tripGear.tripId, next.id)).limit(1),
      db.select({ id: schema.tripMeal.id }).from(schema.tripMeal).where(eq(schema.tripMeal.tripId, next.id)).limit(1),
    ]);
    nextReady = { gear: gear.length > 0, meals: meals.length > 0, gpx: (next.track?.length ?? 0) > 1 };
  }

  return <HomeView trips={trips} baseG={base.baseG} byCategory={base.byCategory} nextReady={nextReady} />;
}
