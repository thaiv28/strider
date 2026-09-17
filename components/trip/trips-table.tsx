"use client";

import Link from "next/link";
import { useState } from "react";
import { Badge } from "@/components/ui";
import { fmtLbs } from "@/lib/util";
import type { TripSummary } from "@/lib/trip";

const tone = (s: string) => (s === "completed" ? "current" : s === "planned" ? "wishlist" : "neutral");
const dash = (v: number | null) => (v == null || v === 0 ? "—" : `${v}`);
const elev = (ft: number | null) => (ft == null || ft === 0 ? "—" : `${(ft / 1000).toFixed(1)}k`);
const wt = (g: number) => (g === 0 ? "—" : fmtLbs(g).replace(" lbs", ""));
const fmtDate = (d: string | null) => {
  if (!d) return "—";
  const [y, m] = d.split("-");
  return `${Number(m)}/${y.slice(2)}`;
};

const GRID = "grid grid-cols-[minmax(0,1fr)_4rem_5.5rem_2.75rem_3rem_3.25rem_3.5rem_3.5rem] items-center gap-x-2";

type Key = "name" | "startDate" | "status" | "nights" | "distanceMi" | "elevationFt" | "baseG" | "skinOutG";
const COLS: { key: Key; label: string; num?: boolean; right?: boolean }[] = [
  { key: "name", label: "Trip" },
  { key: "startDate", label: "Date" },
  { key: "status", label: "Status" },
  { key: "nights", label: "Nts", num: true, right: true },
  { key: "distanceMi", label: "Mi", num: true, right: true },
  { key: "elevationFt", label: "Elev", num: true, right: true },
  { key: "baseG", label: "Base", num: true, right: true },
  { key: "skinOutG", label: "Skin", num: true, right: true },
];

export function TripsTable({ trips }: { trips: TripSummary[] }) {
  const [sort, setSort] = useState<{ key: Key; dir: 1 | -1 }>({ key: "startDate", dir: 1 });

  const sorted = [...trips].sort((a, b) => {
    const av = a[sort.key];
    const bv = b[sort.key];
    let cmp: number;
    if (typeof av === "number" || typeof bv === "number") {
      cmp = ((av as number) ?? -Infinity) - ((bv as number) ?? -Infinity);
    } else {
      cmp = String(av ?? "").localeCompare(String(bv ?? ""));
    }
    return cmp * sort.dir;
  });

  const onSort = (key: Key) =>
    setSort((s) => (s.key === key ? { key, dir: (s.dir * -1) as 1 | -1 } : { key, dir: 1 }));

  return (
    <div className="card overflow-hidden">
      <div className={`${GRID} border-b bg-panel2/60 px-4 py-2`}>
        {COLS.map((c) => (
          <button
            key={c.key}
            onClick={() => onSort(c.key)}
            className={`eyebrow flex items-center gap-1 hover:text-ink ${c.right ? "justify-end" : ""}`}
          >
            {c.label}
            <span className="text-accent">{sort.key === c.key ? (sort.dir === 1 ? "▲" : "▼") : ""}</span>
          </button>
        ))}
      </div>
      {sorted.map((t) => (
        <Link key={t.id} href={`/trips/${t.id}`} className={`${GRID} border-b px-4 py-2.5 transition last:border-0 hover:bg-panel2/40`}>
          <span className="min-w-0">
            <span className="block truncate font-medium">{t.name}</span>
            {t.region && <span className="block truncate text-xs text-muted">{t.region}</span>}
          </span>
          <span className="readout text-sm text-muted">{fmtDate(t.startDate)}</span>
          <span><Badge tone={tone(t.status)}>{t.status}</Badge></span>
          <span className="readout text-right text-sm">{dash(t.nights)}</span>
          <span className="readout text-right text-sm">{dash(t.distanceMi)}</span>
          <span className="readout text-right text-sm">{elev(t.elevationFt)}</span>
          <span className="readout text-right text-sm">{wt(t.baseG)}</span>
          <span className="readout text-right text-sm">{wt(t.skinOutG)}</span>
        </Link>
      ))}
    </div>
  );
}
