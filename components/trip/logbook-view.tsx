"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { Badge } from "@/components/ui";
import { fmtLbs } from "@/lib/util";
import { TripsTable } from "./trips-table";
import { RouteMapThumb } from "./route-map-thumb";
import type { TripSummary } from "@/lib/trip";

type Mode = "list" | "card";
const tone = (s: string) => (s === "completed" ? "current" : s === "planned" ? "wishlist" : "neutral");
const elev = (ft: number | null) => (ft == null || ft === 0 ? "—" : `${(ft / 1000).toFixed(1)}k ft`);
const wt = (g: number) => (g === 0 ? "—" : fmtLbs(g));

export function LogbookView({ trips, shared, initialSection }: { trips: TripSummary[]; shared: (TripSummary & { permission: "view" | "edit"; href: string })[]; initialSection: "mine" | "shared" }) {
  const [section, setSection] = useState<"mine" | "shared">(initialSection);
  const [mode, setMode] = useState<Mode>("list");
  useEffect(() => {
    const saved = localStorage.getItem("bp_logbookMode");
    if (saved === "card" || saved === "list") setMode(saved);
  }, []);
  const pick = (m: Mode) => {
    setMode(m);
    localStorage.setItem("bp_logbookMode", m);
  };

  return (
    <div>
      <div role="tablist" aria-label="Trip ownership" className="mb-5 inline-flex rounded-md border p-1">
        <button type="button" role="tab" aria-selected={section === "mine"} onClick={() => setSection("mine")}
          className={`rounded px-4 py-2 text-sm ${section === "mine" ? "bg-accent text-accentink" : "text-muted"}`}>My trips</button>
        <button type="button" role="tab" aria-selected={section === "shared"} onClick={() => setSection("shared")}
          className={`rounded px-4 py-2 text-sm ${section === "shared" ? "bg-accent text-accentink" : "text-muted"}`}>Shared with me ({shared.length})</button>
      </div>
      {section === "shared" ? (
        <div className="grid gap-3 sm:grid-cols-2">
          {shared.length === 0 && <p className="text-sm text-muted">Open a trip link while signed in to find it here.</p>}
          {shared.map((t) => <Link key={t.id} href={t.href} className="card block p-4 transition hover:border-accent/50">
            <div className="flex items-start justify-between gap-3"><span className="font-display font-semibold">{t.name}</span>
              <Badge tone={t.permission === "edit" ? "current" : "neutral"}>{t.permission === "edit" ? "Can edit" : "View only"}</Badge></div>
            <div className="readout mt-2 text-sm text-muted">{[t.region, t.startDate, t.distanceMi == null ? null : `${t.distanceMi} mi`].filter(Boolean).join(" · ")}</div>
          </Link>)}
        </div>
      ) : <>
      <div className="mb-3 hidden justify-end sm:flex">
        <div className="inline-flex overflow-hidden rounded-md border">
          {(["list", "card"] as Mode[]).map((m) => (
            <button
              key={m}
              onClick={() => pick(m)}
              className={`px-3 py-1 text-sm capitalize transition ${mode === m ? "bg-accent text-accentink" : "hover:bg-panel2/60"}`}
            >
              {m}
            </button>
          ))}
        </div>
      </div>

      {mode === "list" ? (
        <TripsTable trips={trips} />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {trips.map((t) => (
            <Link key={t.id} href={`/trips/${t.id}`}>
              <div className="card h-full overflow-hidden p-4 transition hover:border-accent/50">
                <div className="flex items-start justify-between gap-2">
                  <span className="font-display font-semibold">{t.name}</span>
                  <Badge tone={tone(t.status)}>{t.status}</Badge>
                </div>
                {t.region && <div className="eyebrow mt-1">{t.region}</div>}
                {t.track && t.track.length > 1 && <RouteMapThumb track={t.track} className="mt-3 h-32 w-full" />}
                <div className="readout mt-3 flex flex-wrap gap-x-4 gap-y-1 text-sm text-muted">
                  <span>{t.distanceMi ?? "—"} mi</span>
                  <span>{t.nights ?? "—"} nt</span>
                  <span>{elev(t.elevationFt)}</span>
                  <span>base {wt(t.baseG)}</span>
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
      </>}
    </div>
  );
}
