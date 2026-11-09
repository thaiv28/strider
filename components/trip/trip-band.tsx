"use client";

import Link from "next/link";
import type { ReactNode } from "react";
import { ContourBand } from "@/components/contour-band";
import { fmtDateRange } from "@/lib/util";

export function TripBand({ name, onName, region, startDate, nights, status, distance, gain, packG, skinOutG, backHref, backLabel = "Trips", actions, phoneVisible, overlap = true }: {
  name: string; onName?: (name: string) => void; region: string | null; startDate: string | null; nights: number | null; status: string;
  distance: number | null; gain: number | null; packG: number; skinOutG: number; backHref?: string; backLabel?: string;
  actions?: ReactNode; phoneVisible: boolean; overlap?: boolean;
}) {
  return <header className={`relative shrink-0 overflow-hidden bg-header text-headerink ${phoneVisible ? "block" : "hidden"} sm:block ${overlap ? "pb-5 sm:pb-[84px]" : "pb-5"}`}>
    <ContourBand />
    <div className="relative mx-auto max-w-[1136px] px-4 pt-3.5 sm:px-6">
      <div className="flex flex-wrap items-end justify-between gap-x-8 gap-y-4">
        <div className="min-w-0 basis-full desk:flex-1 desk:basis-0">
          {backHref && <Link href={backHref} className="inline-flex min-h-8 items-center text-sm font-semibold text-headermuted">← {backLabel === "Logbook" ? "Trips" : backLabel}</Link>}
          {onName ? <input aria-label="Trip name" defaultValue={name} onBlur={(event) => { const next = event.target.value.trim(); if (next) onName(next); }} className="block w-full min-w-0 rounded border-0 bg-transparent font-display text-[28px] font-bold leading-[1.05] text-headerink focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-headerborder sm:text-[42px]" /> : <h1 className="break-words font-display text-[28px] font-bold leading-[1.05] sm:text-[42px]">{name}</h1>}
          <div className="mt-1.5 text-sm text-headermuted">{[region, fmtDateRange(startDate, nights) ?? "No dates", status.charAt(0).toUpperCase() + status.slice(1)].filter(Boolean).join(" · ")}</div>
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </div>
      <div className="mt-4 flex flex-wrap gap-x-11 gap-y-2 tabular-nums">
        {[[distance?.toLocaleString() ?? "—", "miles"], [gain?.toLocaleString() ?? "—", "ft gain"], [nights ?? "—", "nights"], [(packG / 453.59237).toFixed(1), "lb pack"], [(skinOutG / 453.59237).toFixed(1), "lb skin-out"]].map(([value, label]) => <div key={label}><span className="font-display text-[22px] font-bold">{value}</span><span className="ml-[5px] text-sm text-headermuted">{label}</span></div>)}
      </div>
    </div>
  </header>;
}
