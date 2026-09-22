import { notFound } from "next/navigation";
import { and, eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getTripView } from "@/lib/trip";
import { addDays, prettyDate } from "@/lib/weather";
import { composeTopoMap } from "@/lib/topo-map";
import { PrintButton } from "@/components/trip/print-button";
import { getCurrentUserId } from "@/lib/gear";

export const dynamic = "force-dynamic";

export async function generateMetadata({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const userId = await getCurrentUserId();
  const [t] = await db.select({ name: schema.trip.name }).from(schema.trip).where(and(eq(schema.trip.id, Number(id)), eq(schema.trip.userId, userId)));
  return { title: `${t?.name ?? "Trip"} (print)` };
}

const fmtMi = (v: string | number | null) => (v == null || v === "" ? "—" : `${v} mi`);
const fmtFt = (v: number | null) => (v == null ? "—" : `${v.toLocaleString()} ft`);
const coord = (lat: number | null, lon: number | null) =>
  lat != null && lon != null ? `${lat.toFixed(4)}, ${lon.toFixed(4)}` : "—";

export default async function TripPrintPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const tripId = Number(id);
  const userId = await getCurrentUserId();
  const view = await getTripView(tripId, userId);
  if (!view) notFound();

  const t = view.trip;
  const track = view.gpx?.track ?? [];
  // High-res map rendered landscape then rotated 90° server-side to portrait.
  // 1600×1240 → rotated 1240×1600 (aspect ~0.775 ≈ a full letter page), so at
  // width:100% on a zero-margin page it fills the sheet.
  const mapPng = await composeTopoMap({ track, campsites: view.campsites.map((c) => ({ night: c.night, lat: c.lat, lon: c.lon })), width: 1600, height: 1240, pad: 0.06, rotate: true });
  const mapSrc = mapPng ? `data:image/png;base64,${mapPng.toString("base64")}` : null;
  const permitRows = await db.select().from(schema.tripPermit).where(eq(schema.tripPermit.tripId, tripId));
  const permit = permitRows[0] ?? null;
  const permitIsImage = permit?.mimeType.startsWith("image/");

  const end = t.startDate && t.nights != null ? addDays(t.startDate, t.nights) : null;
  const dates = t.startDate ? `${prettyDate(t.startDate)}${end ? ` – ${prettyDate(end)}` : ""}` : "date TBD";
  const when = [dates, t.nights != null ? `${t.nights} night${t.nights === 1 ? "" : "s"}` : null].filter(Boolean).join(" · ");
  const place = [view.region, view.areaType].filter(Boolean).join(" · ");

  const campByDay = new Map(view.perDayAuto.map((a) => [a.day, a]));
  const dayRows = [...view.dayBreakdown].sort((a, b) => a.dayNumber - b.dayNumber);

  return (
    <div className="print-doc mx-auto max-w-3xl px-8 py-8">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <h1 className="font-display text-2xl font-bold leading-tight">{t.name}</h1>
          <div className="readout mt-1 whitespace-nowrap text-sm text-muted">{when}</div>
          {place && <div className="readout mt-0.5 text-sm text-muted">{place}</div>}
          <div className="readout mt-0.5 text-sm text-muted">
            {fmtMi(view.distanceMi)} · {fmtFt(view.elevationFt)} gain
            {t.trailhead ? ` · from ${t.trailhead}` : ""}
          </div>
        </div>
        <PrintButton />
      </div>

      {dayRows.length > 0 && (
        <table className="mb-4 w-full border-collapse text-sm">
          <thead>
            <tr className="border-b-2 text-left">
              <th className="py-1 pr-3 font-semibold">Day</th>
              <th className="py-1 pr-3 font-semibold">Distance</th>
              <th className="py-1 pr-3 font-semibold">Gain</th>
              <th className="py-1 font-semibold">Camp (lat, lon)</th>
            </tr>
          </thead>
          <tbody>
            {dayRows.map((d) => {
              const camp = campByDay.get(d.dayNumber);
              return (
                <tr key={d.dayNumber} className="border-b">
                  <td className="py-1 pr-3">Day {d.dayNumber}</td>
                  <td className="readout py-1 pr-3">{fmtMi(d.distanceMi)}</td>
                  <td className="readout py-1 pr-3">{fmtFt(d.elevationGainFt)}</td>
                  <td className="readout py-1">{coord(camp?.lat ?? null, camp?.lon ?? null)}</td>
                </tr>
              );
            })}
            <tr className="border-t-2 font-semibold">
              <td className="py-1 pr-3">Total</td>
              <td className="readout py-1 pr-3">{fmtMi(view.distanceMi)}</td>
              <td className="readout py-1 pr-3">{fmtFt(view.elevationFt)}</td>
              <td />
            </tr>
          </tbody>
        </table>
      )}

      {t.permitNotes && (
        <div className="mb-4">
          <div className="eyebrow">Permit</div>
          <p className="mt-1 whitespace-pre-wrap text-sm">{t.permitNotes}</p>
        </div>
      )}

      {permit && !permitIsImage && (
        <div className="rounded-md border p-3 text-sm">
          <span className="font-semibold">Permit attached:</span>{" "}
          <a href={`/trips/${tripId}/permit`} target="_blank" rel="noreferrer" className="text-accent underline">
            {permit.filename}
          </a>
          <span className="text-muted"> — PDF; open the link to print it separately.</span>
        </div>
      )}

      {mapSrc && (
        <div className="map-page">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={mapSrc} alt="USGS topo route map" />
        </div>
      )}

      {permit && permitIsImage && (
        <div className="permit-page">
          <div className="eyebrow">Permit document</div>
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={`/trips/${tripId}/permit`} alt="Permit" className="mt-1 w-full rounded-md border" />
        </div>
      )}
    </div>
  );
}
