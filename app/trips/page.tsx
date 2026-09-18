import { getCurrentUserId } from "@/lib/gear";
import { getTripsSummary } from "@/lib/trip";
import { LogbookView } from "@/components/trip/logbook-view";
import { NewTripButton } from "@/components/trip/new-trip-button";
import { PlanWithAI } from "@/components/trip/plan-with-ai";

export const dynamic = "force-dynamic";
export const metadata = { title: "Trips" };

export default async function TripsPage() {
  const userId = await getCurrentUserId();
  const trips = await getTripsSummary(userId);

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
        <LogbookView trips={trips} />
      </div>
    </div>
  );
}
