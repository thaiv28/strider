import { and, eq, asc, inArray, sql } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { DEFAULT_COOK_WATER_ML } from "@/lib/fuel";

export type TripGearRow = {
  id: number;
  gearItemId: number | null;
  name: string;
  weightG: number; // per-unit
  quantity: number;
  weightClass: "base" | "worn";
  packed: boolean;
  category: string | null;
};

const n = (s: string | null) => (s == null ? null : Number(s));

export type TripFoodEntry = {
  id: number;
  dayNumber: number;
  mealType: string | null;
  name: string;
  isMeal: boolean;
  mealId: number | null;
  servings: number;
  weightG: number;
  kcal: number;
};
export type TripFoodDay = {
  dayNumber: number;
  distanceMi: number | null;
  gainFt: number | null;
  targetKcal: number;
  plannedKcal: number;
  plannedG: number;
  entries: TripFoodEntry[];
};
export type TripFood = {
  params: { bmr: number; calPerMile: number; ftPerMile: number };
  days: TripFoodDay[];
  totalKcal: number;
  totalG: number;
  avgKcal: number;
};

// Estimated burn for a day: BMR plus energy-miles (distance + gain converted to
// equivalent miles) × cal/mile. A day with no distance/gain is a rest day (BMR).
export function dayTargetKcal(p: { bmr: number; calPerMile: number; ftPerMile: number }, distMi: number | null, gainFt: number | null): number {
  const energyMiles = (distMi ?? 0) + (gainFt ?? 0) / p.ftPerMile;
  return Math.round(p.bmr + energyMiles * p.calPerMile);
}

export async function getTripFood(tripId: number, userId: number): Promise<TripFood> {
  const [ep] = await db.select().from(schema.energyParams).where(eq(schema.energyParams.userId, userId));
  const params = { bmr: ep?.bmr ?? 1735, calPerMile: ep?.calPerEnergyMile ?? 200, ftPerMile: ep?.ftPerEnergyMile ?? 625 };

  const [t] = await db
    .select({ nights: schema.trip.nights })
    .from(schema.trip)
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  if (!t) throw new Error("Trip not found");
  const dayRows = await db.select().from(schema.tripDay).where(eq(schema.tripDay.tripId, tripId));
  const dayMap = new Map(dayRows.map((d) => [d.dayNumber, d]));

  const rows = await db.select().from(schema.tripMeal).where(eq(schema.tripMeal.tripId, tripId));
  const byDay = new Map<number, TripFoodEntry[]>();
  for (const r of rows) {
    const e: TripFoodEntry = {
      id: r.id,
      dayNumber: r.dayNumber,
      mealType: r.mealType,
      name: r.snapshotName,
      isMeal: r.mealId != null,
      mealId: r.mealId,
      servings: Number(r.servings) || 1,
      weightG: r.snapshotWeightG,
      kcal: r.snapshotKcal,
    };
    if (!byDay.has(e.dayNumber)) byDay.set(e.dayNumber, []);
    byDay.get(e.dayNumber)!.push(e);
  }

  const maxMealDay = rows.reduce((m, r) => Math.max(m, r.dayNumber), 0);
  const D = Math.max(dayRows.length, (t?.nights ?? 0) + 1, maxMealDay, 1);

  const days: TripFoodDay[] = [];
  let totalKcal = 0;
  let totalG = 0;
  for (let k = 1; k <= D; k++) {
    const d = dayMap.get(k);
    const distMi = d ? n(d.distanceMi) : null;
    const gainFt = d?.elevationGainFt ?? null;
    const entries = (byDay.get(k) ?? []).sort((a, b) => a.id - b.id);
    const plannedKcal = entries.reduce((s, e) => s + e.kcal, 0);
    const plannedG = entries.reduce((s, e) => s + e.weightG, 0);
    totalKcal += plannedKcal;
    totalG += plannedG;
    days.push({ dayNumber: k, distanceMi: distMi, gainFt, targetKcal: dayTargetKcal(params, distMi, gainFt), plannedKcal, plannedG, entries });
  }

  return { params, days, totalKcal, totalG, avgKcal: Math.round(totalKcal / D) };
}

export type ShoppingItem = { key: string; name: string; grams: number };

// Consolidated grocery list: every planned meal's ingredients (scaled to the
// planned servings) plus standalone ingredient entries, summed per ingredient.
// Grams are for a single serving-plan; callers scale by the party/people count.
export async function getShoppingList(tripId: number, userId: number): Promise<ShoppingItem[]> {
  const [owned] = await db
    .select({ id: schema.trip.id })
    .from(schema.trip)
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  if (!owned) throw new Error("Trip not found");
  const entries = await db
    .select({
      mealId: schema.tripMeal.mealId,
      ingredientId: schema.tripMeal.ingredientId,
      servings: schema.tripMeal.servings,
      snapshotName: schema.tripMeal.snapshotName,
      snapshotWeightG: schema.tripMeal.snapshotWeightG,
      baseServings: schema.meal.baseServings,
    })
    .from(schema.tripMeal)
    .leftJoin(schema.meal, eq(schema.tripMeal.mealId, schema.meal.id))
    .where(eq(schema.tripMeal.tripId, tripId));

  const mealIds = [...new Set(entries.filter((e) => e.mealId != null).map((e) => e.mealId!))];
  const miRows = mealIds.length
    ? await db
        .select({
          mealId: schema.mealIngredient.mealId,
          ingredientId: schema.mealIngredient.ingredientId,
          snapshotName: schema.mealIngredient.snapshotName,
          amountG: schema.mealIngredient.amountG,
        })
        .from(schema.mealIngredient)
        .where(inArray(schema.mealIngredient.mealId, mealIds))
    : [];
  const miByMeal = new Map<number, typeof miRows>();
  for (const r of miRows) {
    const list = miByMeal.get(r.mealId) ?? [];
    list.push(r);
    miByMeal.set(r.mealId, list);
  }

  // Key on ingredient id when known so the same pantry item merges across meals;
  // fall back to the (case-folded) snapshot name for id-less entries.
  type Acc = { name: string; grams: number; ingredientId: number | null };
  const acc = new Map<string, Acc>();
  const add = (ingredientId: number | null, name: string, grams: number) => {
    if (grams <= 0) return;
    const key = ingredientId != null ? `i${ingredientId}` : `n:${name.trim().toLowerCase()}`;
    const cur = acc.get(key);
    if (cur) cur.grams += grams;
    else acc.set(key, { name: name.trim(), grams, ingredientId });
  };

  for (const e of entries) {
    if (e.mealId != null) {
      const scale = (Number(e.servings) || 1) / Math.max(1, e.baseServings ?? 1);
      for (const it of miByMeal.get(e.mealId) ?? []) add(it.ingredientId, it.snapshotName, Math.round(it.amountG * scale));
    } else {
      add(e.ingredientId, e.snapshotName, e.snapshotWeightG);
    }
  }

  return [...acc.entries()]
    .map(([key, a]) => ({ key, name: a.name, grams: a.grams }))
    .sort((x, y) => x.name.localeCompare(y.name));
}

export type RoutePt = { d: number; lat: number; lon: number; e: number };
export type DaySegment = { day: number; distanceMi: number; elevationGainFt: number; lat: number | null; lon: number | null };

// Split a route into per-day segments at each campsite (+ a final leg to the
// endpoint). Day k covers [bounds[k-1], bounds[k]]; gain uses a 13ft hysteresis.
export function computeDaySegments(
  pts: RoutePt[],
  campsites: { distanceMi: number; lat: number | null; lon: number | null }[],
): DaySegment[] {
  if (!pts.length || !campsites.length) return [];
  const endD = pts[pts.length - 1].d;
  const bounds = [0, ...campsites.map((c) => c.distanceMi), endD];
  const gainBetween = (d0: number, d1: number) => {
    let g = 0;
    let ref: number | null = null;
    for (const p of pts) {
      if (p.d < d0 || p.d > d1) continue;
      if (ref == null) ref = p.e;
      else if (Math.abs(p.e - ref) >= 13) {
        if (p.e > ref) g += p.e - ref;
        ref = p.e;
      }
    }
    return Math.round(g);
  };
  const out: DaySegment[] = [];
  for (let k = 1; k <= campsites.length + 1; k++) {
    const isLast = k === campsites.length + 1;
    const src = isLast ? pts[pts.length - 1] : campsites[k - 1];
    out.push({
      day: k,
      distanceMi: +(bounds[k] - bounds[k - 1]).toFixed(2),
      elevationGainFt: gainBetween(bounds[k - 1], bounds[k]),
      lat: src?.lat ?? null,
      lon: src?.lon ?? null,
    });
  }
  return out;
}

export type TripSummary = {
  id: number;
  name: string;
  status: "idea" | "planned" | "completed";
  startDate: string | null;
  region: string | null;
  nights: number | null;
  distanceMi: number | null;
  elevationFt: number | null;
  baseG: number;
  skinOutG: number;
  track: [number, number][] | null;
};

export async function getTripsSummary(userId: number, sharedIds?: number[]): Promise<TripSummary[]> {
  if (sharedIds && !sharedIds.length) return [];
  const filter = sharedIds ? inArray(schema.trip.id, sharedIds) : eq(schema.trip.userId, userId);
  const rows = await db
    .select()
    .from(schema.trip)
    .leftJoin(schema.trail, eq(schema.trip.trailId, schema.trail.id))
    .leftJoin(schema.tripGpx, eq(schema.tripGpx.tripId, schema.trip.id))
    .where(filter)
    .orderBy(sql`${schema.trip.startDate} DESC NULLS LAST`, asc(schema.trip.id));
  const gear = await db
    .select({
      tripId: schema.tripGear.tripId,
      weightClass: schema.tripGear.weightClass,
      weightG: schema.tripGear.snapshotWeightG,
      quantity: schema.tripGear.quantity,
    })
    .from(schema.tripGear)
    .innerJoin(schema.trip, eq(schema.tripGear.tripId, schema.trip.id))
    .where(filter);

  const agg = new Map<number, { base: number; worn: number }>();
  for (const g of gear) {
    const a = agg.get(g.tripId) ?? { base: 0, worn: 0 };
    a[g.weightClass] += g.weightG * g.quantity;
    agg.set(g.tripId, a);
  }

  return rows.map((r) => {
    const t = r.trip;
    const a = agg.get(t.id) ?? { base: 0, worn: 0 };
    const days = Math.max(1, (t.nights ?? 0) + 1);
    const consumableG = t.foodGPerDay * days + t.waterGPerDay + t.fuelGPerDay;
    return {
      id: t.id,
      name: t.name,
      status: t.status,
      startDate: t.startDate,
      region: t.region ?? r.trail?.region ?? null,
      nights: t.nights,
      distanceMi:
        n(t.distanceMi) ?? (r.trip_gpx ? n(r.trip_gpx.distanceMi) : null) ?? (r.trail ? n(r.trail.typicalDistanceMi) : null),
      elevationFt: t.elevationGainFt ?? r.trip_gpx?.elevationGainFt ?? r.trail?.typicalElevationFt ?? null,
      baseG: a.base,
      skinOutG: a.base + consumableG + a.worn,
      track: r.trip_gpx?.track ?? null,
    };
  });
}

export async function getTripView(tripId: number, userId: number) {
  const [row] = await db
    .select()
    .from(schema.trip)
    .leftJoin(schema.trail, eq(schema.trip.trailId, schema.trail.id))
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  if (!row) return null;
  const t = row.trip;
  const trail = row.trail;

  const gearRows = await db
    .select()
    .from(schema.tripGear)
    .where(eq(schema.tripGear.tripId, tripId))
    .orderBy(asc(schema.tripGear.sortOrder), asc(schema.tripGear.snapshotName));

  const dayRows = await db
    .select()
    .from(schema.tripDay)
    .where(eq(schema.tripDay.tripId, tripId))
    .orderBy(asc(schema.tripDay.dayNumber));

  const [gpxRow] = await db.select().from(schema.tripGpx).where(eq(schema.tripGpx.tripId, tripId));

  const campRows = await db
    .select()
    .from(schema.tripCampsite)
    .where(eq(schema.tripCampsite.tripId, tripId))
    .orderBy(asc(schema.tripCampsite.distanceMi));
  const campsites = campRows.map((c, i) => ({
    id: c.id,
    night: i + 1,
    distanceMi: n(c.distanceMi) ?? 0,
    lat: n(c.lat),
    lon: n(c.lon),
    eleFt: c.eleFt,
  }));

  const perDayAuto = computeDaySegments(gpxRow?.points ?? [], campsites);

  // Cook water for ONE person, summed over planned HOT meals (per-serving water
  // × planned servings). A hot meal with no explicit water uses the default
  // boil volume. Cold meals and ingredient-only entries contribute nothing.
  // Party size scales this in the UI.
  const mealWaterRows = await db
    .select({
      isHot: schema.meal.isHot,
      waterMl: schema.meal.waterMl,
      baseServings: schema.meal.baseServings,
      servings: schema.tripMeal.servings,
    })
    .from(schema.tripMeal)
    .leftJoin(schema.meal, eq(schema.tripMeal.mealId, schema.meal.id))
    .where(eq(schema.tripMeal.tripId, tripId));
  let cookWaterMlPerPerson = 0;
  for (const r of mealWaterRows) {
    if (!r.isHot) continue;
    const bs = Math.max(1, r.baseServings ?? 1);
    cookWaterMlPerPerson += ((r.waterMl ?? DEFAULT_COOK_WATER_ML) / bs) * (Number(r.servings) || 1);
  }
  cookWaterMlPerPerson = Math.round(cookWaterMlPerPerson);

  const gear: TripGearRow[] = gearRows.map((g) => ({
    id: g.id,
    gearItemId: g.gearItemId,
    name: g.snapshotName,
    weightG: g.snapshotWeightG,
    quantity: g.quantity,
    weightClass: g.weightClass,
    packed: g.packed,
    category: g.snapshotCategory,
  }));

  const days = Math.max(1, (t.nights ?? 0) + 1);
  const baseG = gear.filter((g) => g.weightClass === "base").reduce((s, g) => s + g.weightG * g.quantity, 0);
  const wornG = gear.filter((g) => g.weightClass === "worn").reduce((s, g) => s + g.weightG * g.quantity, 0);
  const foodG = t.foodGPerDay * days;
  // Water & fuel are flat totals (average carried), not per-day rates.
  const waterG = t.waterGPerDay;
  const fuelG = t.fuelGPerDay;
  const consumableG = foodG + waterG + fuelG;

  // Group gear by category in the library's category order.
  const cats = await db
    .select({ name: schema.gearCategory.name })
    .from(schema.gearCategory)
    .where(eq(schema.gearCategory.userId, t.userId))
    .orderBy(asc(schema.gearCategory.sortOrder));
  const order = new Map(cats.map((c, i) => [c.name, i]));
  const byCat = new Map<string, TripGearRow[]>();
  for (const g of gear) {
    const key = g.category ?? "Uncategorized";
    if (!byCat.has(key)) byCat.set(key, []);
    byCat.get(key)!.push(g);
  }
  const groups = [...byCat.entries()]
    .map(([category, rows]) => ({ category, rows }))
    .sort((a, b) => (order.get(a.category) ?? 99) - (order.get(b.category) ?? 99));

  return {
    trip: t,
    trailName: trail?.name ?? null,
    region: t.region ?? trail?.region ?? null,
    areaType: t.areaType ?? trail?.areaType ?? null,
    // Precedence: trip's own value → GPX-derived → trail default.
    distanceMi: n(t.distanceMi) ?? (gpxRow ? n(gpxRow.distanceMi) : null) ?? (trail ? n(trail.typicalDistanceMi) : null),
    elevationFt: t.elevationGainFt ?? gpxRow?.elevationGainFt ?? trail?.typicalElevationFt ?? null,
    lat: n(t.lat) ?? (gpxRow ? n(gpxRow.startLat) : null),
    lon: n(t.lon) ?? (gpxRow ? n(gpxRow.startLon) : null),
    days,
    cookWaterMlPerPerson,
    weights: { baseG, wornG, foodG, waterG, fuelG, consumableG, packG: baseG + consumableG, skinOutG: baseG + consumableG + wornG },
    groups,
    gearCount: gear.length,
    packedCount: gear.filter((g) => g.packed).length,
    dayBreakdown: dayRows.map((d) => ({
      id: d.id,
      dayNumber: d.dayNumber,
      distanceMi: d.distanceMi,
      elevationGainFt: d.elevationGainFt,
    })),
    gpx: gpxRow
      ? {
          filename: gpxRow.filename,
          startLat: n(gpxRow.startLat),
          startLon: n(gpxRow.startLon),
          distanceMi: n(gpxRow.distanceMi),
          elevationGainFt: gpxRow.elevationGainFt,
          minEleFt: gpxRow.minEleFt,
          maxEleFt: gpxRow.maxEleFt,
          track: gpxRow.track ?? [],
          profile: gpxRow.profile ?? [],
          points: gpxRow.points ?? [],
        }
      : null,
    campsites,
    perDayAuto,
  };
}
