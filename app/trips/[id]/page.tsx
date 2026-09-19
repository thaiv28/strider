import { notFound } from "next/navigation";
import { eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId, getGearView, getLoadoutData } from "@/lib/gear";
import { getTripView, getTripFood, getShoppingList } from "@/lib/trip";
import { getMeals, getIngredients } from "@/lib/pantry";
import { DEFAULT_TEMPLATE, DEFAULT_PACKING, type ReportData } from "@/lib/report";
import { TripView } from "@/components/trip/trip-view";

export const dynamic = "force-dynamic";

export async function generateMetadata({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [t] = await db.select({ name: schema.trip.name }).from(schema.trip).where(eq(schema.trip.id, Number(id)));
  return { title: t?.name ?? "Trip" };
}

// Normalize a saved template at render time (non-destructive): rename the old
// "Route" heading to "Campsites" and append the grocery block if it's missing,
// so both changes reach templates saved before them.
function normalizeReportTemplate(template: string): string {
  let t = template.replace(/^##\s+Route\b.*$/im, "## Campsites");
  if (!t.includes("{{shopping_list}}")) t = `${t.trimEnd()}\n\n## Grocery list\n{{shopping_list}}\n`;
  return t;
}

export default async function TripPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ from?: string; y?: string; m?: string }>;
}) {
  const { id } = await params;
  const { from, y, m } = await searchParams;
  const calHref = y != null && m != null ? `/calendar?y=${y}&m=${m}` : "/calendar";
  const back = from === "calendar" ? { href: calHref, label: "Calendar" } : { href: "/trips", label: "Logbook" };
  const tripId = Number(id);
  const view = await getTripView(tripId);
  if (!view) notFound();

  const userId = await getCurrentUserId();
  const [gearView, loadoutData, tripFood, shopping, meals, ingredients, reportRows, permitRows] = await Promise.all([
    getGearView(userId),
    getLoadoutData(userId),
    getTripFood(tripId, userId),
    getShoppingList(tripId),
    getMeals(userId),
    getIngredients(userId),
    db.select().from(schema.reportSettings).where(eq(schema.reportSettings.userId, userId)),
    db
      .select({ filename: schema.tripPermit.filename, mimeType: schema.tripPermit.mimeType, sizeBytes: schema.tripPermit.sizeBytes })
      .from(schema.tripPermit)
      .where(eq(schema.tripPermit.tripId, tripId)),
  ]);
  const reportSettings = reportRows[0];
  const packingDefault = reportSettings?.packingDefault ?? DEFAULT_PACKING;
  const tripPackingList = view.trip.packingList ?? packingDefault;
  const library = gearView.groups
    .flatMap((g) => g.rows)
    .filter((r) => r.status === "current")
    .map((r) => ({
      id: r.id,
      name: r.name,
      category: r.slotName,
      weightG: r.weightG,
      quantity: r.quantity,
      defaultWeightClass: r.defaultWeightClass,
    }));

  const t = view.trip;
  const groceryPeople = t.groceryPeople ?? t.partySize;
  const groceryRemoved = t.groceryRemoved ?? [];
  const removedSet = new Set(groceryRemoved);
  const report: ReportData = {
    name: t.name,
    region: view.region,
    areaType: view.areaType,
    startDate: t.startDate,
    nights: t.nights,
    days: view.days,
    distanceMi: view.distanceMi,
    gainFt: view.elevationFt,
    trailhead: t.trailhead,
    permits: t.permitNotes,
    waterSources: t.waterSources,
    driving: t.drivingNotes,
    planningNotes: t.planningNotes,
    dayBreakdown: view.dayBreakdown.map((d) => ({ dayNumber: d.dayNumber, distanceMi: d.distanceMi, elevationGainFt: d.elevationGainFt })),
    perDayAuto: view.perDayAuto.map((a) => ({ day: a.day, lat: a.lat, lon: a.lon })),
    gpx: view.gpx
      ? { distanceMi: view.gpx.distanceMi, elevationGainFt: view.gpx.elevationGainFt, minEleFt: view.gpx.minEleFt, maxEleFt: view.gpx.maxEleFt }
      : null,
    campsites: view.campsites.map((c) => ({ night: c.night, distanceMi: c.distanceMi, lat: c.lat, lon: c.lon })),
    food: tripFood,
    shopping: shopping.filter((i) => !removedSet.has(i.key)).map((i) => ({ key: i.key, name: i.name, grams: i.grams * groceryPeople })),
    waterG: t.waterGPerDay,
    fuelG: t.fuelGPerDay,
    packingList: tripPackingList,
    track: view.gpx?.track ?? [],
    lat: view.lat,
    lon: view.lon,
    elevationFt: view.gpx?.maxEleFt ?? null,
  };

  return (
    <TripView
      trip={{
        id: t.id,
        name: t.name,
        status: t.status,
        startDate: t.startDate,
        nights: t.nights,
        partySize: t.partySize,
        lat: t.lat,
        lon: t.lon,
        distanceMi: t.distanceMi,
        elevationGainFt: t.elevationGainFt,
        foodGPerDay: t.foodGPerDay,
        waterGPerDay: t.waterGPerDay,
        fuelGPerDay: t.fuelGPerDay,
        trailhead: t.trailhead,
        permitRequired: t.permitRequired,
        permitNotes: t.permitNotes,
        drivingNotes: t.drivingNotes,
        waterSources: t.waterSources,
        planningNotes: t.planningNotes,
        tripReport: t.tripReport,
        shareToken: t.shareToken,
      }}
      region={view.region}
      areaType={view.areaType}
      trailName={view.trailName}
      effectiveDistance={view.distanceMi}
      effectiveElevation={view.elevationFt}
      lat={view.lat}
      lon={view.lon}
      days={view.days}
      cookWaterMlPerPerson={view.cookWaterMlPerPerson}
      weights={view.weights}
      groups={view.groups}
      gearCount={view.gearCount}
      packedCount={view.packedCount}
      dayBreakdown={view.dayBreakdown}
      gpx={view.gpx}
      campsites={view.campsites}
      perDayAuto={view.perDayAuto}
      loadouts={loadoutData.loadouts}
      library={library}
      food={tripFood}
      shopping={shopping}
      groceryPeople={groceryPeople}
      groceryRemoved={groceryRemoved}
      meals={meals.map((m) => ({ id: m.id, name: m.name, mealType: m.mealType, baseServings: m.baseServings, totalKcal: m.totalKcal, totalG: m.totalG }))}
      ingredients={ingredients.map((i) => ({ id: i.id, name: i.name, kcalPer100g: i.kcalPer100g, defaultServingG: i.defaultServingG }))}
      report={report}
      reportTemplate={normalizeReportTemplate(reportSettings?.template ?? DEFAULT_TEMPLATE)}
      tripPackingList={tripPackingList}
      permit={permitRows[0] ?? null}
      backHref={back.href}
      backLabel={back.label}
    />
  );
}
