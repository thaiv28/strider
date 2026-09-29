"use client";

import { useState } from "react";
import { fmtWeight, gToOz } from "@/lib/util";
import { categoryColor } from "@/lib/categories";

export type SliceItem = { name: string; g: number; quantity: number };
export type CatSlice = { name: string; g: number; items: SliceItem[] };

// Base weight only — consumables and worn gear are trip-specific and live on
// the trip page, not in this generic number. Two views of the same breakdown.
export function BaseBar({ baseG, byCategory }: { baseG: number; byCategory: CatSlice[] }) {
  const [view, setView] = useState<"bar" | "pie">("bar");
  const [hover, setHover] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const slices = byCategory.filter((c) => c.g > 0).sort((a, b) => b.g - a.g);
  const active = hover ? slices.find((s) => s.name === hover) ?? null : null;
  const open = selected ? slices.find((s) => s.name === selected) ?? null : null;

  return (
    <div className="card p-4">
      <div className="flex items-baseline justify-between gap-3">
        <span className="eyebrow">◇ Base weight</span>
        <div className="flex overflow-hidden rounded-full border text-xs">
          {(["bar", "pie"] as const).map((v) => (
            <button
              key={v}
              onClick={() => setView(v)}
              className={`px-2.5 py-0.5 capitalize transition ${
                view === v ? "bg-accent text-accentink" : "text-muted hover:text-ink"
              }`}
            >
              {v}
            </button>
          ))}
        </div>
      </div>

      <div className="mt-1.5 flex items-baseline gap-3">
        <span className="readout text-4xl font-bold leading-none">{fmtWeight(baseG)}</span>
        {active && (
          <span className="readout flex items-center gap-1.5 text-sm text-muted">
            <span className="h-2.5 w-2.5 rounded-sm" style={{ background: categoryColor(active.name) }} />
            <span className="font-medium text-ink">{active.name}</span>
            {gToOz(active.g).toFixed(1)} oz · {((active.g / baseG) * 100).toFixed(0)}%
          </span>
        )}
      </div>

      {view === "bar" ? (
        <>
          <div className="relative mt-3 flex h-8 overflow-hidden rounded-full" role="group" aria-label="Base weight by category">
            {slices.map((s) => (
              <button
                type="button"
                key={s.name}
                aria-label={`${s.name}: ${fmtWeight(s.g)}. Show items`}
                onMouseEnter={() => setHover(s.name)}
                onMouseLeave={() => setHover(null)}
                onFocus={() => setHover(s.name)}
                onBlur={() => setHover(null)}
                onClick={() => setSelected(s.name)}
                style={{ width: `${(s.g / baseG) * 100}%` }}
                className={`relative h-full min-w-0 touch-manipulation transition-opacity focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-accent ${hover && hover !== s.name ? "opacity-30" : "opacity-100"}`}
              >
                <span className="absolute inset-x-0 top-2.5 h-3" style={{ background: categoryColor(s.name) }} />
              </button>
            ))}
          </div>
          <Legend slices={slices} baseG={baseG} cols="sm:grid-cols-2 lg:grid-cols-3" hover={hover} setHover={setHover} onSelect={setSelected} />
        </>
      ) : (
        <div className="mt-3 flex flex-wrap items-center gap-4">
          <Donut slices={slices} baseG={baseG} hover={hover} setHover={setHover} active={active} onSelect={setSelected} />
          <div className="min-w-48 flex-1">
            <Legend slices={slices} baseG={baseG} cols="grid-cols-1" pct hover={hover} setHover={setHover} onSelect={setSelected} />
          </div>
        </div>
      )}

      {open && <SectionModal slice={open} baseG={baseG} onClose={() => setSelected(null)} />}
    </div>
  );
}

function SectionModal({ slice, baseG, onClose }: { slice: CatSlice; baseG: number; onClose: () => void }) {
  const items = [...slice.items].sort((a, b) => b.g - a.g);
  return (
    <div className="fixed inset-0 z-[1100] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm" onClick={onClose}>
      <div className="card flex max-h-[80vh] w-full max-w-md flex-col p-5" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-2">
            <span className="h-3 w-3 rounded-sm" style={{ background: categoryColor(slice.name) }} />
            <span className="font-display text-lg font-bold">{slice.name}</span>
          </div>
          <button onClick={onClose} aria-label="Close" className="text-muted hover:text-accent">✕</button>
        </div>
        <div className="readout mt-1 text-sm text-muted">
          {fmtWeight(slice.g)} · {((slice.g / baseG) * 100).toFixed(0)}% of base · {items.length} items
        </div>
        <div className="mt-3 min-h-0 flex-1 divide-y overflow-y-auto rounded-lg border">
          {items.map((it, i) => (
            <div key={i} className="flex items-baseline gap-2 px-3 py-2 text-sm">
              <span className="min-w-0 flex-1 truncate">
                {it.name}
                {it.quantity > 1 && <span className="text-muted"> ×{it.quantity}</span>}
              </span>
              <span className="readout shrink-0 text-muted">{gToOz(it.g).toFixed(1)} oz</span>
            </div>
          ))}
          {items.length === 0 && <div className="px-3 py-6 text-center text-sm text-muted">No items.</div>}
        </div>
      </div>
    </div>
  );
}

function Donut({
  slices,
  baseG,
  hover,
  setHover,
  active,
  onSelect,
}: {
  slices: CatSlice[];
  baseG: number;
  hover: string | null;
  setHover: (n: string | null) => void;
  active: CatSlice | null;
  onSelect: (n: string) => void;
}) {
  const r = 42;
  const C = 2 * Math.PI * r;
  let offset = 0;
  return (
    <div className="relative h-32 w-32 shrink-0">
      <svg viewBox="0 0 100 100" className="h-full w-full -rotate-90">
        {slices.map((s) => {
          const frac = s.g / baseG;
          const dash = `${Math.max(0, frac * C - 1.2)} ${C}`;
          const dim = hover && hover !== s.name;
          const el = (
            <circle
              key={s.name}
              cx="50"
              cy="50"
              r={r}
              fill="none"
              stroke={categoryColor(s.name)}
              strokeWidth={hover === s.name ? 18 : 15}
              strokeDasharray={dash}
              strokeDashoffset={-offset * C}
              opacity={dim ? 0.3 : 1}
              onMouseEnter={() => setHover(s.name)}
              onMouseLeave={() => setHover(null)}
              onClick={() => onSelect(s.name)}
              className="cursor-pointer transition-all"
            >
              <title>{`${s.name}: ${fmtWeight(s.g)} (${(frac * 100).toFixed(0)}%)`}</title>
            </circle>
          );
          offset += frac;
          return el;
        })}
      </svg>
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center px-6 text-center">
        {active ? (
          <>
            <div className="readout text-lg font-bold leading-tight">{gToOz(active.g).toFixed(1)} oz</div>
            <div className="text-xs font-medium text-muted">{active.name}</div>
          </>
        ) : (
          <div className="eyebrow text-muted">{slices.length} groups</div>
        )}
      </div>
    </div>
  );
}

function Legend({
  slices,
  baseG,
  cols,
  pct,
  hover,
  setHover,
  onSelect,
}: {
  slices: CatSlice[];
  baseG: number;
  cols: string;
  pct?: boolean;
  hover: string | null;
  setHover: (n: string | null) => void;
  onSelect: (n: string) => void;
}) {
  return (
    <div className={`mt-2 grid ${cols} gap-x-6 gap-y-0.5`}>
      {slices.map((s) => (
        <button
          type="button"
          key={s.name}
          onMouseEnter={() => setHover(s.name)}
          onMouseLeave={() => setHover(null)}
          onFocus={() => setHover(s.name)}
          onBlur={() => setHover(null)}
          onClick={() => onSelect(s.name)}
          className={`flex min-h-11 w-full touch-manipulation items-center text-left gap-2 text-sm transition-opacity sm:min-h-7 ${
            hover && hover !== s.name ? "opacity-40" : "opacity-100"
          }`}
        >
          <span className="h-2.5 w-2.5 shrink-0 rounded-sm" style={{ background: categoryColor(s.name) }} />
          <span className={hover === s.name ? "font-semibold" : "font-medium"}>{s.name}</span>
          <span className="readout ml-auto text-muted">
            {gToOz(s.g).toFixed(1)} oz{pct && ` · ${((s.g / baseG) * 100).toFixed(0)}%`}
          </span>
        </button>
      ))}
    </div>
  );
}
