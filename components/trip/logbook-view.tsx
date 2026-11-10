"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { Badge, Chip, SegmentedControl } from "@/components/ui";
import { fmtLbs, fmtDateRange } from "@/lib/util";
import { TripsTable } from "./trips-table";
import { RouteMapThumb } from "./route-map-thumb";
import type { TripSummary } from "@/lib/trip";

type Mode = "list" | "card";
type Filter = "all" | "planned" | "idea" | "completed" | "shared";
const filters = [["all", "All"], ["planned", "Planned"], ["idea", "Ideas"], ["completed", "Completed"], ["shared", "Shared with me"]] as const;
const elevation = (ft: number | null) => ft == null ? "—" : ft.toLocaleString();

function Thumbnail({ trip, className }: { trip: TripSummary; className: string }) {
  return <div className={`shrink-0 overflow-hidden bg-map ${className}`}>
    {trip.status !== "idea" && trip.track && trip.track.length > 1
      ? <RouteMapThumb track={trip.track} className="h-full w-full rounded-none border-0" />
      : <div className="flex h-full items-center justify-center p-2 text-center text-xs text-muted">No route yet</div>}
  </div>;
}

export function LogbookView({ trips, shared, initialSection }: { trips: TripSummary[]; shared: (TripSummary & { permission: "view" | "edit"; href: string })[]; initialSection: "mine" | "shared" }) {
  const [filter, setFilter] = useState<Filter>(initialSection === "shared" ? "shared" : "all");
  const [mode, setMode] = useState<Mode>("card");
  useEffect(() => {
    const saved = localStorage.getItem("bp_logbookMode");
    if (saved === "card" || saved === "list") setMode(saved);
  }, []);
  const pick = (value: Mode) => { setMode(value); localStorage.setItem("bp_logbookMode", value); };
  const today = new Date();
  const dateKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  const next = trips.filter((t) => t.status === "planned" && (!t.startDate || t.startDate >= dateKey))
    .sort((a, b) => a.startDate ? b.startDate ? a.startDate.localeCompare(b.startDate) : -1 : b.startDate ? 1 : 0)[0];
  const countdown = next?.startDate ? Math.round((Date.parse(next.startDate) - Date.parse(dateKey)) / 86400000) : null;
  const visible = trips.filter((t) => t.id !== next?.id && (filter === "all" || t.status === filter));
  return <div className="space-y-5">
    {next && filter !== "shared" && <Link href={`/trips/${next.id}`} className="card flex flex-col overflow-hidden transition hover:border-accent/50 sm:flex-row">
      <div className="h-[150px] shrink-0 bg-map sm:h-auto sm:w-[34%]">
        {next.track && next.track.length > 1 && <RouteMapThumb track={next.track} className="h-full min-h-[150px] w-full rounded-none border-0" />}
      </div>
      <div className="min-w-0 flex-1 p-5 sm:p-6">
        <p className="text-sm font-semibold text-signal">Next up{countdown != null ? ` · in ${countdown} days` : ""}</p>
        <h2 className="mt-2 break-words font-display text-[28px] font-bold leading-tight sm:text-[32px]">{next.name}</h2>
        <p className="mt-1 text-sm text-muted">{[next.region, fmtDateRange(next.startDate, next.nights) ?? "No dates"].filter(Boolean).join(" · ")}</p>
        <div className="mt-5 grid grid-cols-4 gap-2">
          {[["Miles", next.distanceMi ?? "—"], ["Feet gain", elevation(next.elevationFt)], ["Nights", next.nights ?? "—"], ["Base lb", (next.baseG / 453.59237).toFixed(1)]].map(([label, value]) => <div key={label}><div className="font-display text-[23px] font-bold">{value}</div><div className="text-xs text-muted">{label}</div></div>)}
        </div>
      </div>
    </Link>}
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div className="flex flex-wrap gap-2">{filters.map(([value, label]) => <Chip key={value} selected={filter === value} onClick={() => setFilter(value)}>{label}{value === "shared" ? ` (${shared.length})` : ""}</Chip>)}</div>
      {filter !== "shared" && <SegmentedControl label="Trip layout" options={[{ value: "card", label: "Cards" }, { value: "list", label: "List" }]} value={mode} onChange={pick} className="hidden w-40 sm:grid" />}
    </div>
    {filter === "shared" ? <div className="grid gap-3 sm:grid-cols-2">
      {shared.length === 0 && <p className="text-sm text-muted">Open a trip link while signed in to find it here.</p>}
      {shared.map((t) => <Link key={t.id} href={t.href} className="card block p-4 transition hover:border-accent/50">
        <div className="flex items-start justify-between gap-3"><span className="font-display font-semibold">{t.name}</span><Badge tone={t.permission === "edit" ? "current" : "neutral"}>{t.permission === "edit" ? "Can edit" : "View only"}</Badge></div>
        <div className="readout mt-2 text-sm text-muted">{[t.region, fmtDateRange(t.startDate, t.nights) ?? "No dates", t.distanceMi == null ? null : `${t.distanceMi} mi`].filter(Boolean).join(" · ")}</div>
      </Link>)}
    </div> : <>
      {visible.length === 0 && <p className="py-4 text-sm text-muted">No trips in this section yet.</p>}
      {mode === "list" && <div className="hidden sm:block"><TripsTable trips={visible} /></div>}
      <div className={`grid gap-3 sm:grid-cols-[repeat(auto-fill,minmax(260px,1fr))] ${mode === "list" ? "sm:hidden" : ""}`}>
        {visible.map((t) => <Link key={t.id} href={`/trips/${t.id}`} className="card flex min-w-0 overflow-hidden transition hover:border-accent/50 sm:block">
          <Thumbnail trip={t} className="h-auto min-h-[88px] w-[68px] sm:h-24 sm:min-h-0 sm:w-full" />
          <div className="min-w-0 p-3">
            <h2 className="break-words font-display text-base font-bold">{t.name}</h2>
            <p className="mt-0.5 text-xs text-muted">{[t.region, fmtDateRange(t.startDate, t.nights) ?? "No dates"].filter(Boolean).join(" · ")}</p>
            <p className="readout mt-1 text-xs">{t.status === "idea" && t.distanceMi == null && t.elevationFt == null && t.nights == null ? "Idea · no route yet" : [t.distanceMi == null ? null : `${t.distanceMi} mi`, t.elevationFt == null ? null : `${elevation(t.elevationFt)} ft`, t.nights == null ? null : `${t.nights} nights`].filter(Boolean).join(" · ")}</p>
            <span className="sr-only">Base {fmtLbs(t.baseG)}</span>
          </div>
        </Link>)}
      </div>
    </>}
  </div>;
}
