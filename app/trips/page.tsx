import { getCurrentUserId } from "@/lib/gear";
import { getTripsSummary } from "@/lib/trip";
import { LogbookView } from "@/components/trip/logbook-view";
import { NewTripButton } from "@/components/trip/new-trip-button";
import { PlanWithAI } from "@/components/trip/plan-with-ai";
import { db, schema } from "@/src/db/index";
import { eq } from "drizzle-orm";

export const dynamic = "force-dynamic";
export const metadata = { title: "Trips" };

export default async function TripsPage({ searchParams }: { searchParams: Promise<{ section?: string }> }) {
  const userId = await getCurrentUserId();
  const { section } = await searchParams;
  const [trips, visits] = await Promise.all([
    getTripsSummary(userId),
    db.select({ tripId: schema.tripShareVisit.tripId, permission: schema.tripShareVisit.permission,
      ownerId: schema.trip.userId, viewToken: schema.trip.shareToken, editToken: schema.trip.editToken })
      .from(schema.tripShareVisit)
      .innerJoin(schema.trip, eq(schema.trip.id, schema.tripShareVisit.tripId))
      .where(eq(schema.tripShareVisit.userId, userId)),
  ]);
  const sharedIds = [...new Set(visits.filter((v) => v.ownerId !== userId && (v.permission === "view" ? v.viewToken : v.editToken)).map((v) => v.tripId))];
  const summaries = await getTripsSummary(userId, sharedIds);
  const shared = summaries.map((t) => {
    const edit = visits.find((v) => v.tripId === t.id && v.permission === "edit" && v.editToken);
    const view = visits.find((v) => v.tripId === t.id && v.permission === "view" && v.viewToken);
    return { ...t, permission: edit ? "edit" as const : "view" as const,
      href: edit ? `/trips/${t.id}` : `/share/trips/${view!.viewToken}` };
  });

  return (
    <div className="mx-auto max-w-4xl px-4 py-6 sm:px-8 sm:py-10">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <div className="eyebrow">◇ Logbook</div>
          <h1 className="font-display mt-1 text-3xl font-bold tracking-tight">Trips</h1>
        </div>
        <div className="flex items-center gap-2">
          <PlanWithAI />
          <NewTripButton />
        </div>
      </div>

      <div className="mt-6">
        <LogbookView trips={trips} shared={shared} initialSection={section === "shared" ? "shared" : "mine"} />
      </div>
    </div>
  );
}
