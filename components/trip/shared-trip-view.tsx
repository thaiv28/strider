"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { ArrowLeft } from "lucide-react";
import { TripBand } from "./trip-band";
import { ContourBand } from "@/components/contour-band";
import { RoutePanel, tripLayout, TripSectionBar, ElevationSparkline, type RoutePoint } from "./route-panel";
import { RouteMapThumb } from "./route-map-thumb";
import { Card, Button, Chip } from "@/components/ui";
import { WeatherPanel } from "./weather-panel";
import { fmtWeight, fmtLbs, fmtDateRange } from "@/lib/util";
import type { TripFood, TripGearRow, ShoppingItem } from "@/lib/trip";

const sections = [["logistics", "Logistics"], ["planning", "Planning"], ["gear", "Gear"], ["food", "Food"], ["weather", "Weather"]] as const;
type Section = typeof sections[number][0];

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
  signedIn: boolean;
  gpx: { points: RoutePoint[]; minEleFt: number | null; maxEleFt: number | null } | null;
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
  const hasRoute = !!props.gpx?.points.length;
  const [minimized, setMinimized] = useState(false);
  const [isPhone, setIsPhone] = useState(false);
  const [tab, setTab] = useState<Section>("logistics");
  const [phoneView, setPhoneView] = useState<Section | "map">(hasRoute ? "map" : "logistics");
  useEffect(() => {
    const query = window.matchMedia("(max-width: 639px)");
    const update = () => setIsPhone(query.matches);
    update();
    setMinimized(localStorage.getItem("strider-route-minimized") === "true");
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  const toggleRoute = (value: boolean) => { setMinimized(value); localStorage.setItem("strider-route-minimized", String(value)); };
  const pickTab = (key: Section) => { setTab(key); setPhoneView(key); };
  const activeTab = isPhone && phoneView !== "map" ? phoneView : tab;
  const dates = fmtDateRange(trip.startDate, trip.nights) ?? "No dates";
  const endCoord = props.perDayAuto.length ? props.perDayAuto[props.perDayAuto.length - 1] : null;

  return (
    <main data-testid="shared-trip" className={tripLayout.frame}>
      {!props.signedIn && <header className="relative hidden shrink-0 overflow-hidden bg-header text-headerink sm:block">
        <ContourBand /><div className="relative mx-auto flex min-h-16 max-w-[1136px] items-center gap-6 px-6"><Link href="/" className="font-display text-[22px] font-bold">Strider</Link><span className="flex-1 text-sm text-headermuted">View-only trip plan</span><Link href="/login" className="inline-flex min-h-11 items-center rounded-xl bg-headerink px-4 text-sm font-semibold text-headeractionink hover:opacity-90">Sign in</Link></div>
      </header>}
      <header className={`${phoneView === "logistics" ? "hidden" : "block"} relative shrink-0 overflow-hidden bg-header text-headerink sm:hidden`}>
        <ContourBand /><div className="relative flex min-h-[72px] items-center gap-2 px-3">
          <Link href={props.signedIn ? "/trips?section=shared" : "/"} aria-label={props.signedIn ? "Back to Trips" : "Strider home"} className="grid h-11 w-11 shrink-0 place-items-center"><ArrowLeft size={21} /></Link>
          <div className="min-w-0 flex-1"><div className="truncate font-display text-[21px] font-bold">{trip.name}</div><div className="truncate text-xs text-headermuted">{dates} · View-only trip plan</div></div>
          {!props.signedIn && <Link href="/login" className="inline-flex min-h-11 shrink-0 items-center rounded-xl border border-headerborder px-3 text-sm font-semibold hover:bg-headeractive">Sign in</Link>}
        </div>
      </header>
      <TripBand name={trip.name} region={props.region} startDate={trip.startDate} nights={trip.nights} status={trip.status} distance={props.distanceMi} gain={props.elevationFt} packG={props.weights.packG} skinOutG={props.weights.skinOutG} backHref={props.signedIn ? "/trips?section=shared" : undefined} phoneVisible={phoneView === "logistics"} overlap={hasRoute} />
      <div className={tripLayout.split}>
        {hasRoute && props.gpx && <aside aria-label="Route map and elevation" className={tripLayout.map(phoneView)}>
          <div className={`${minimized && !isPhone ? "hidden" : "flex"} relative h-full min-h-0 flex-1 flex-col overflow-hidden bg-panel sm:rounded-[18px] sm:shadow-[0_8px_28px_rgba(23,56,45,0.16)]`}>
            <RoutePanel readOnly points={props.gpx.points} minFt={props.gpx.minEleFt} maxFt={props.gpx.maxEleFt} campsites={props.campsites} mapActions={<Button className="w-24 !bg-white px-0 !text-[#18261f] shadow max-sm:!hidden" onClick={() => toggleRoute(true)}>Hide map</Button>} />
          </div>
          {minimized && !isPhone && <Card className="flex items-center gap-3 p-3"><RouteMapThumb track={props.track} className="h-14 w-20 shrink-0" /><ElevationSparkline points={props.gpx.points} /><span className="text-sm text-muted">{(props.gpx.minEleFt ?? Math.min(...props.gpx.points.map((p) => p.e))).toLocaleString()} to {(props.gpx.maxEleFt ?? Math.max(...props.gpx.points.map((p) => p.e))).toLocaleString()} ft</span><Button variant="outline" className="ml-auto" onClick={() => toggleRoute(false)}>Show map and elevation</Button></Card>}
        </aside>}
        <div className={tripLayout.content(hasRoute, phoneView)}>
          <div className="hidden flex-wrap gap-1.5 sm:flex">{sections.map(([key, label]) => <Chip key={key} selected={activeTab === key} onClick={() => pickTab(key)}>{label}</Chip>)}</div>
          <div className="mt-3">
      {activeTab === "logistics" && <div className="grid items-start gap-3.5 desk:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <Card className="p-4">
          <h2 className="text-[17px] font-bold">Details</h2>
          <div className="mt-2.5 grid grid-cols-2 gap-x-3 gap-y-2.5">
            <Metric label="Start date" value={dates} /><Metric label="Party size" value={String(trip.partySize)} />
            <Metric label="Region" value={value(props.region)} /><Metric label="Area type" value={value(props.areaType)} />
            <Metric label="Distance" value={props.distanceMi == null ? "—" : `${props.distanceMi} mi`} /><Metric label="Elevation gain" value={props.elevationFt == null ? "—" : `${props.elevationFt.toLocaleString()} ft`} />
          </div>
        </Card>
        <Card className="p-4">
          <h2 className="text-[17px] font-bold">By day</h2>
          <div className="mt-2">
            {props.dayBreakdown.length === 0 && <p className="text-sm text-muted">No days yet.</p>}
            {props.dayBreakdown.map((day) => <div key={day.id} className="flex justify-between gap-3 border-b py-2 text-sm last:border-0"><span className="font-semibold">Day {day.dayNumber}</span><span className="tabular-nums text-muted">{day.distanceMi ?? "—"} mi · {day.elevationGainFt?.toLocaleString() ?? "—"} ft</span></div>)}
          </div>
        </Card>

      </div>}
      {activeTab === "weather" && <section className="mt-3">
        <WeatherPanel lat={props.lat} lon={props.lon} startDate={trip.startDate} days={props.days} nights={trip.nights ?? props.campsites.length} campsites={props.campsites} endCoord={endCoord} elevationFt={props.gpx?.maxEleFt ?? null} />
      </section>}
      {activeTab === "planning" &&
      <section>
        <div className="grid gap-3 sm:grid-cols-2">
          <TextCard label="Trailhead" text={trip.trailhead} />
          <TextCard label="Permits" text={trip.permitNotes} />
          <TextCard label="Water sources" text={trip.waterSources} />
          <TextCard label="Driving / access" text={trip.drivingNotes} />
        </div>
        <div className="mt-3"><TextCard label="Planning notes" text={trip.planningNotes} /></div>
        {props.permit && (
          <Card className="mt-3 p-4 text-sm">
            <h2 className="text-[17px] font-bold">Permit document</h2>
            <a className="mt-1 inline-block text-accent underline" href={`/share/trips/${props.shareToken}/permit`} target="_blank" rel="noreferrer">{props.permit.filename}</a>
          </Card>
        )}
      </section>

      }
      {activeTab === "gear" && <section className="mt-3">
        <div className="text-sm text-muted">{props.groups.flatMap((g) => g.rows).filter((r) => r.packed).length}/{props.groups.flatMap((g) => g.rows).length} packed · Base {fmtLbs(props.weights.baseG)} · Consumable {fmtLbs(props.weights.foodG + props.weights.waterG + props.weights.fuelG)} · Worn {fmtLbs(props.weights.wornG)}</div>
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

      }
      {activeTab === "food" && <>
      <section>
        <div className="grid gap-3 sm:grid-cols-2">
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
          <h2 className="text-[17px] font-bold">Grocery list</h2>
          {props.shopping.length ? <ul className="mt-2 space-y-1 text-sm">{props.shopping.map((item) => <li key={item.key}>□ {item.name} — {item.grams} g</li>)}</ul> : <p className="mt-2 text-sm text-muted">No groceries listed.</p>}
        </Card>
      </section>

      </>}
      {activeTab === "food" && <div className="mt-3"><TextCard label="Packing list" text={trip.packingList} /></div>}
      </div>
      <p className="mt-8 text-center text-xs text-muted">Live view · updates automatically when the trip plan changes</p>
        </div>
      </div>
      <TripSectionBar sections={sections} hasRoute={hasRoute} value={phoneView} onChange={(key) => key === "map" ? setPhoneView("map") : pickTab(key)} />
    </main>
  );
}

function Metric({ label, value: metric }: { label: string; value: string }) {
  return <div><div className="text-[13px] font-semibold text-muted">{label}</div><div className="readout mt-1 text-sm">{metric}</div></div>;
}

function TextCard({ label, text }: { label: string; text: string | null }) {
  return <Card className="p-4"><h2 className="text-[17px] font-bold">{label}</h2><div className="mt-1 max-w-2xl whitespace-pre-wrap text-sm">{value(text)}</div></Card>;
}
