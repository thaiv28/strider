import { notFound, redirect } from "next/navigation";
import { eq, or } from "drizzle-orm";
import { auth } from "@/auth";
import { db, schema } from "@/src/db/index";
import { getTripFood, getShoppingList, getTripView } from "@/lib/trip";
import { DEFAULT_PACKING } from "@/lib/report";
import { SharedTripView } from "@/components/trip/shared-trip-view";
import type { Metadata } from "next";

export const dynamic = "force-dynamic";

async function sharedTrip(token: string) {
  const [row] = await db.select({ id: schema.trip.id, userId: schema.trip.userId, shareToken: schema.trip.shareToken, editToken: schema.trip.editToken })
    .from(schema.trip).where(or(eq(schema.trip.shareToken, token), eq(schema.trip.editToken, token)));
  return row ?? null;
}

export async function generateMetadata({ params }: { params: Promise<{ token: string }> }): Promise<Metadata> {
  const { token } = await params;
  const [row] = await db
    .select({ name: schema.trip.name, region: schema.trip.region, startDate: schema.trip.startDate })
    .from(schema.trip)
    .where(or(eq(schema.trip.shareToken, token), eq(schema.trip.editToken, token)));
  if (!row) notFound();
  const title = `${row.name} · Strider`;
  const details = [row.region, row.startDate].filter(Boolean).join(" · ");
  const description = details
    ? `Explore the trip plan for ${row.name} (${details}) on Strider.`
    : `Explore the trip plan for ${row.name} on Strider.`;
  return {
    title: { absolute: title },
    description,
    openGraph: {
      title,
      description,
      siteName: "Strider",
      type: "website",
      url: `/share/trips/${encodeURIComponent(token)}`,
    },
    twitter: { card: "summary_large_image", title, description },
    robots: { index: false, follow: false, noarchive: true },
  };
}

export default async function SharedTripPage({ params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  const shared = await sharedTrip(token);
  if (!shared) notFound();

  const session = await auth();
  const visitorId = Number(session?.user?.id);
  if (shared.editToken === token) {
    if (!visitorId) redirect(`/login?callbackUrl=${encodeURIComponent(`/share/trips/${token}`)}`);
    if (visitorId !== shared.userId) {
      await db.insert(schema.tripShareVisit).values({ tripId: shared.id, userId: visitorId, permission: "edit" })
        .onConflictDoNothing();
    }
    redirect(`/trips/${shared.id}`);
  }
  if (visitorId && visitorId !== shared.userId) {
    await db.insert(schema.tripShareVisit).values({ tripId: shared.id, userId: visitorId, permission: "view" })
      .onConflictDoNothing();
  }

  const view = await getTripView(shared.id, shared.userId);
  if (!view || view.trip.shareToken !== token) notFound();
  const [food, shopping, settingsRows, permitRows] = await Promise.all([
    getTripFood(shared.id, view.trip.userId),
    getShoppingList(shared.id, shared.userId),
    db.select({ packingDefault: schema.reportSettings.packingDefault }).from(schema.reportSettings).where(eq(schema.reportSettings.userId, view.trip.userId)),
    db.select({ filename: schema.tripPermit.filename, mimeType: schema.tripPermit.mimeType }).from(schema.tripPermit).where(eq(schema.tripPermit.tripId, shared.id)),
  ]);
  const groceryPeople = view.trip.groceryPeople ?? view.trip.partySize;
  const removed = new Set(view.trip.groceryRemoved ?? []);

  return (
    <SharedTripView
      trip={{
        name: view.trip.name,
        status: view.trip.status,
        startDate: view.trip.startDate,
        nights: view.trip.nights,
        partySize: view.trip.partySize,
        trailhead: view.trip.trailhead,
        permitNotes: view.trip.permitNotes,
        drivingNotes: view.trip.drivingNotes,
        waterSources: view.trip.waterSources,
        planningNotes: view.trip.planningNotes,
        packingList: view.trip.packingList ?? settingsRows[0]?.packingDefault ?? DEFAULT_PACKING,
      }}
      region={view.region}
      areaType={view.areaType}
      distanceMi={view.distanceMi}
      elevationFt={view.elevationFt}
      lat={view.lat}
      lon={view.lon}
      days={view.days}
      weights={view.weights}
      groups={view.groups}
      dayBreakdown={view.dayBreakdown}
      track={view.gpx?.track ?? []}
      campsites={view.campsites}
      perDayAuto={view.perDayAuto}
      food={food}
      shopping={shopping.filter((item) => !removed.has(item.key)).map((item) => ({ ...item, grams: item.grams * groceryPeople }))}
      permit={permitRows[0] ?? null}
      shareToken={token}
    />
  );
}
