"use client";

import dynamic from "next/dynamic";
import { Card, Badge } from "@/components/ui";
import { WeatherPanel } from "./weather-panel";
import { fmtWeight } from "@/lib/util";
import { prettyDate } from "@/lib/weather";
import type { TripFood, TripGearRow, ShoppingItem } from "@/lib/trip";

const RouteMap = dynamic(() => import("./route-map"), {
  ssr: false,
  loading: () => <div className="h-80 w-full animate-pulse rounded-md bg-panel2/50" />,
});

type SharedTrip = {
  name: string;
  status: "idea" | "planned" | "completed";
  startDate: string | null;
  nights: number | null;
  partySize: number;
  trailhead: string | null;
  permitNotes: string | null;
  drivingNotes: string | null;
  waterSources: string | null;
  planningNotes: string | null;
  packingList: string;
};

type Props = {
  trip: SharedTrip;
  region: string | null;
  areaType: string | null;
  distanceMi: number | null;
  elevationFt: number | null;
  lat: number | null;
  lon: number | null;
  days: number;
  weights: { baseG: number; wornG: number; foodG: number; waterG: number; fuelG: number; packG: number; skinOutG: number };
  groups: { category: string; rows: TripGearRow[] }[];
  dayBreakdown: { id: number; dayNumber: number; distanceMi: string | null; elevationGainFt: number | null }[];
  track: [number, number][];
  campsites: { id: number; night: number; distanceMi: number; lat: number | null; lon: number | null; eleFt: number | null }[];
  perDayAuto: { day: number; distanceMi: number; elevationGainFt: number; lat: number | null; lon: number | null }[];
  food: TripFood;
  shopping: ShoppingItem[];
  permit: { filename: string; mimeType: string } | null;
  shareToken: string;
};

const value = (v: string | null) => v?.trim() || "—";

export function SharedTripView(props: Props) {
  const { trip } = props;
  const endCoord = props.perDayAuto.length ? props.perDayAuto[props.perDayAuto.length - 1] : null;
  const campByDay = new Map(props.perDayAuto.map((d) => [d.day, d]));

  return (
    <main data-testid="shared-trip" className="mx-auto max-w-4xl px-4 py-8 sm:px-8">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="eyebrow">View-only trip plan</div>
          <h1 className="font-display mt-1 text-3xl font-bold tracking-tight">{trip.name}</h1>
          <div className="readout mt-1 text-sm text-muted">
            {[props.region, props.areaType, trip.startDate ? prettyDate(trip.startDate) : "date TBD"].filter(Boolean).join(" · ")}
          </div>
        </div>
        <Badge tone={trip.status === "completed" ? "current" : trip.status === "planned" ? "wishlist" : "neutral"}>{trip.status}</Badge>
      </div>

      <section className="mt-7">
        <div className="eyebrow">Overview</div>
        <Card className="mt-2 grid grid-cols-2 gap-4 p-4 sm:grid-cols-4">
          <Metric label="Distance" value={props.distanceMi == null ? "—" : `${props.distanceMi} mi`} />
          <Metric label="Elevation gain" value={props.elevationFt == null ? "—" : `${props.elevationFt.toLocaleString()} ft`} />
          <Metric label="Length" value={trip.nights == null ? "—" : `${trip.nights + 1} days / ${trip.nights} nights`} />
          <Metric label="Party" value={`${trip.partySize} ${trip.partySize === 1 ? "person" : "people"}`} />
          <Metric label="Base weight" value={fmtWeight(props.weights.baseG)} />
          <Metric label="Food" value={fmtWeight(props.weights.foodG)} />
          <Metric label="Pack weight" value={fmtWeight(props.weights.packG)} />
          <Metric label="Skin-out" value={fmtWeight(props.weights.skinOutG)} />
        </Card>
      </section>

      {props.dayBreakdown.length > 0 && (
        <section className="mt-7">
          <div className="eyebrow">Itinerary</div>
          <Card className="mt-2 overflow-x-auto p-4">
            <table className="w-full min-w-[34rem] border-collapse text-sm">
              <thead><tr className="border-b text-left"><th className="py-2">Day</th><th>Distance</th><th>Gain</th><th>Camp</th></tr></thead>
              <tbody>{props.dayBreakdown.map((d) => { const camp = campByDay.get(d.dayNumber); return (
                <tr key={d.id} className="border-b last:border-0"><td className="py-2">Day {d.dayNumber}</td><td className="readout">{d.distanceMi ? `${d.distanceMi} mi` : "—"}</td><td className="readout">{d.elevationGainFt == null ? "—" : `${d.elevationGainFt.toLocaleString()} ft`}</td><td className="readout">{camp?.lat != null && camp.lon != null ? `${camp.lat.toFixed(4)}, ${camp.lon.toFixed(4)}` : "—"}</td></tr>
              ); })}</tbody>
            </table>
          </Card>
        </section>
      )}

      {props.track.length > 0 && (
        <section className="mt-7 overflow-hidden rounded-lg border bg-panel">
          <RouteMap track={props.track} campsites={props.campsites} />
        </section>
      )}

      <section className="mt-7">
        <WeatherPanel lat={props.lat} lon={props.lon} startDate={trip.startDate} days={props.days} nights={trip.nights ?? props.campsites.length} campsites={props.campsites} endCoord={endCoord} elevationFt={props.elevationFt} />
      </section>

      <section className="mt-7">
        <div className="eyebrow">Planning details</div>
        <div className="mt-2 grid gap-3 sm:grid-cols-2">
          <TextCard label="Trailhead" text={trip.trailhead} />
          <TextCard label="Permits" text={trip.permitNotes} />
          <TextCard label="Water sources" text={trip.waterSources} />
          <TextCard label="Driving / access" text={trip.drivingNotes} />
        </div>
        <div className="mt-3"><TextCard label="Planning notes" text={trip.planningNotes} /></div>
        {props.permit && (
          <Card className="mt-3 p-4 text-sm">
            <div className="eyebrow">Permit document</div>
            <a className="mt-1 inline-block text-accent underline" href={`/share/trips/${props.shareToken}/permit`} target="_blank" rel="noreferrer">{props.permit.filename}</a>
          </Card>
        )}
      </section>

      <section className="mt-7">
        <div className="eyebrow">Gear</div>
        <div className="mt-2 space-y-3">
          {props.groups.length === 0 && <Card className="p-4 text-sm text-muted">No gear added.</Card>}
          {props.groups.map((group) => (
            <Card key={group.category} className="overflow-hidden">
              <div className="border-b bg-panel2/50 px-4 py-2 font-display font-semibold">{group.category}</div>
              {group.rows.map((item) => (
                <div key={item.id} className="flex items-center justify-between gap-4 border-b px-4 py-2 text-sm last:border-0">
                  <span>{item.name}{item.quantity > 1 ? ` ×${item.quantity}` : ""}</span>
                  <span className="readout shrink-0 text-muted">{item.packed ? "✓ packed · " : ""}{item.weightClass} · {fmtWeight(item.weightG * item.quantity)}</span>
                </div>
              ))}
            </Card>
          ))}
        </div>
      </section>

      <section className="mt-7">
        <div className="eyebrow">Food plan</div>
        <div className="mt-2 grid gap-3 sm:grid-cols-2">
          {props.food.days.map((day) => (
            <Card key={day.dayNumber} className="p-4">
              <div className="font-display font-semibold">Day {day.dayNumber}</div>
              {day.entries.length ? <ul className="mt-2 list-disc space-y-1 pl-5 text-sm">{day.entries.map((entry) => <li key={entry.id}>{entry.name}{entry.isMeal && entry.servings !== 1 ? ` ×${entry.servings}` : ""}</li>)}</ul> : <p className="mt-2 text-sm text-muted">No meals planned.</p>}
              <div className="readout mt-3 text-xs text-muted">{day.plannedKcal.toLocaleString()} kcal · {fmtWeight(day.plannedG)}</div>
            </Card>
          ))}
        </div>
      </section>

      <section className="mt-7 grid gap-3 sm:grid-cols-2">
        <Card className="p-4">
          <div className="eyebrow">Grocery list</div>
          {props.shopping.length ? <ul className="mt-2 space-y-1 text-sm">{props.shopping.map((item) => <li key={item.key}>□ {item.name} — {item.grams} g</li>)}</ul> : <p className="mt-2 text-sm text-muted">No groceries listed.</p>}
        </Card>
        <Card className="p-4">
          <div className="eyebrow">Packing list</div>
          <div className="mt-2 whitespace-pre-wrap text-sm">{value(trip.packingList)}</div>
        </Card>
      </section>

      <p className="mt-8 text-center text-xs text-muted">Live view · updates automatically when the trip plan changes</p>
    </main>
  );
}

function Metric({ label, value: metric }: { label: string; value: string }) {
  return <div><div className="eyebrow">{label}</div><div className="readout mt-1 text-sm">{metric}</div></div>;
}

function TextCard({ label, text }: { label: string; text: string | null }) {
  return <Card className="p-4"><div className="eyebrow">{label}</div><div className="mt-1 whitespace-pre-wrap text-sm">{value(text)}</div></Card>;
}
