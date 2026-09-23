"use client";

import { useEffect, useRef, useState, useTransition } from "react";
import dynamic from "next/dynamic";
import { Button } from "@/components/ui";
import { addCampsite, moveCampsite, removeCampsite, clearCampsites } from "@/app/trips/actions";

// Reserve the map's height while the client-only Leaflet chunk loads so the
// content below doesn't jump once it mounts.
const RouteMap = dynamic(() => import("./route-map"), {
  ssr: false,
  loading: () => <div className="h-80 w-full animate-pulse bg-panel2/50" />,
});

export type RoutePoint = { d: number; lat: number; lon: number; e: number };
export type Campsite = { id: number; night: number; distanceMi: number; lat: number | null; lon: number | null; eleFt: number | null };

export function RoutePanel({
  tripId,
  points,
  minFt,
  maxFt,
  campsites,
  pushUndo,
}: {
  tripId: number;
  points: RoutePoint[];
  minFt: number | null;
  maxFt: number | null;
  campsites: Campsite[];
  pushUndo: (label: string, run: () => Promise<void>) => void;
}) {
  const [, start] = useTransition();
  const [local, setLocal] = useState<Campsite[]>(campsites);
  const [hover, setHover] = useState<RoutePoint | null>(null);
  const [drag, setDrag] = useState<{ id: number; prev: Campsite } | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const suppressClick = useRef(false);
  const touchStart = useRef<{ x: number; y: number } | null>(null);
  const touchClick = useRef<{ x: number; y: number; at: number } | null>(null);

  const sig = campsites.map((c) => `${c.id}:${c.distanceMi}`).join("|");
  useEffect(() => setLocal(campsites), [sig]);

  if (!points.length) return null;
  const maxD = points[points.length - 1].d || 1;
  const minE = minFt ?? Math.min(...points.map((p) => p.e));
  const maxE = maxFt ?? Math.max(...points.map((p) => p.e));
  const span = maxE - minE || 1;
  const H = 30;

  const leftPct = (d: number) => (d / maxD) * 100;
  const topPct = (e: number) => ((H - 1 - ((e - minE) / span) * (H - 2)) / H) * 100;
  const nearest = (d: number) => points.reduce((a, b) => (Math.abs(b.d - d) < Math.abs(a.d - d) ? b : a), points[0]);
  const fracFromEvent = (e: { clientX: number }) => {
    const r = boxRef.current!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  };

  // Sort by distance so night numbers stay in trail order (updates live on drag).
  const shown = [...local].sort((a, b) => a.distanceMi - b.distanceMi).map((c, i) => ({ ...c, night: i + 1 }));
  const selectedSite = shown.find((c) => c.id === selectedId);

  const onMove = (e: React.PointerEvent) => {
    const p = nearest(fracFromEvent(e) * maxD);
    setHover(p);
    if (drag) setLocal((cs) => cs.map((c) => (c.id === drag.id ? { ...c, distanceMi: p.d, lat: p.lat, lon: p.lon, eleFt: p.e } : c)));
  };
  const addAt = (p: RoutePoint) => {
    setSelectedId(null);
    start(async () => {
      const id = await addCampsite(tripId, { distanceMi: p.d, lat: p.lat, lon: p.lon, eleFt: p.e });
      pushUndo("Added campsite", () => removeCampsite(id, tripId));
    });
  };
  const removeSite = (site: Campsite) => {
    setSelectedId(null);
    start(async () => await removeCampsite(site.id, tripId));
    pushUndo("Removed campsite", async () => {
      await addCampsite(tripId, { distanceMi: site.distanceMi, lat: site.lat, lon: site.lon, eleFt: site.eleFt });
    });
  };
  const onUp = (e: React.PointerEvent) => {
    if (!drag) {
      // Touch devices do not reliably fire a pointermove before the synthetic
      // click. Place the site using the actual tap, and ignore that click.
      if (e.pointerType === "touch" && touchStart.current &&
          Math.hypot(e.clientX - touchStart.current.x, e.clientY - touchStart.current.y) < 10) {
        addAt(nearest(fracFromEvent(e) * maxD));
        touchClick.current = { x: e.clientX, y: e.clientY, at: Date.now() };
      }
      touchStart.current = null;
      return;
    }
    const p = nearest(fracFromEvent(e) * maxD);
    const { id, prev } = drag;
    setDrag(null);
    touchStart.current = null;
    if (p.d === prev.distanceMi) return;
    suppressClick.current = true;
    start(async () => await moveCampsite(id, tripId, { distanceMi: p.d, lat: p.lat, lon: p.lon, eleFt: p.e }));
    pushUndo("Moved campsite", () =>
      moveCampsite(id, tripId, { distanceMi: prev.distanceMi, lat: prev.lat, lon: prev.lon, eleFt: prev.eleFt }),
    );
  };
  const onClick = (e: React.MouseEvent) => {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    const tapped = touchClick.current;
    touchClick.current = null;
    if (tapped && Date.now() - tapped.at < 750 &&
        Math.hypot(e.clientX - tapped.x, e.clientY - tapped.y) < 10) return;
    addAt(nearest(fracFromEvent(e) * maxD));
  };

  const area = `0,${H} ${points.map((p) => `${leftPct(p.d).toFixed(2)},${((H - 1 - ((p.e - minE) / span) * (H - 2))).toFixed(2)}`).join(" ")} 100,${H}`;
  const line = points.map((p) => `${leftPct(p.d).toFixed(2)},${((H - 1 - ((p.e - minE) / span) * (H - 2))).toFixed(2)}`).join(" ");

  return (
    <div>
      <RouteMap track={points.map((p) => [p.lat, p.lon])} campsites={shown} preview={hover ? { lat: hover.lat, lon: hover.lon } : null} />

      <div className="space-y-3 px-4 pt-3 pb-4">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted">Tap or click the profile to drop a campsite · tap a marker to remove · drag to move</span>
        {shown.length > 0 && (
          <button
            onClick={() => confirm("Clear all campsites?") && start(async () => await clearCampsites(tripId))}
            className="eyebrow hover:text-accent"
          >
            clear campsites
          </button>
        )}
      </div>

      <div className="flex gap-3">
        <div className="readout flex h-40 w-12 shrink-0 flex-col justify-between py-0.5 text-right text-xs text-muted">
          <span>{maxE.toLocaleString()}</span>
          <span>{minE.toLocaleString()}</span>
        </div>
        <div className="min-w-0 flex-1">
          <div
            ref={boxRef}
            data-testid="elevation-profile"
            className="relative h-40 w-full cursor-crosshair touch-none select-none"
            onPointerDown={(e) => {
              if (e.pointerType === "touch") touchStart.current = { x: e.clientX, y: e.clientY };
            }}
            onPointerMove={onMove}
            onPointerLeave={() => !drag && setHover(null)}
            onPointerUp={onUp}
            onPointerCancel={() => { touchStart.current = null; setDrag(null); }}
            onClick={onClick}
          >
            <svg viewBox={`0 0 100 ${H}`} preserveAspectRatio="none" className="absolute inset-0 h-full w-full">
              <polygon points={area} fill="var(--accent)" opacity={0.15} />
              <polyline points={line} fill="none" stroke="var(--accent)" strokeWidth={0.8} vectorEffect="non-scaling-stroke" />
            </svg>
            {hover && (
              <>
                <div className="pointer-events-none absolute top-0 bottom-0 w-px bg-ink/30" style={{ left: `${leftPct(hover.d)}%` }} />
                {!drag && (
                  <div
                    className="pointer-events-none absolute h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-accent bg-panel"
                    style={{ left: `${leftPct(hover.d)}%`, top: `${topPct(hover.e)}%` }}
                  />
                )}
                <div className="readout pointer-events-none absolute -top-1 -translate-x-1/2 rounded bg-panel px-1 text-[0.65rem] text-muted" style={{ left: `${leftPct(hover.d)}%` }}>
                  {hover.d.toFixed(1)}mi · {hover.e.toLocaleString()}ft
                </div>
              </>
            )}
            {shown.map((c) => (
              <button
                key={c.id}
                onPointerDown={(e) => {
                  if (e.button !== 0) return;
                  e.stopPropagation();
                  if (e.pointerType === "touch") {
                    setSelectedId(c.id);
                    return;
                  }
                  setDrag({ id: c.id, prev: c });
                  setHover({ d: c.distanceMi, lat: c.lat ?? 0, lon: c.lon ?? 0, e: c.eleFt ?? 0 });
                }}
                onPointerUp={(e) => {
                  if (e.pointerType === "touch") e.stopPropagation();
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  touchClick.current = null;
                  if (suppressClick.current) {
                    suppressClick.current = false;
                    return;
                  }
                  setSelectedId(c.id);
                }}
                onContextMenu={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  removeSite(c);
                }}
                aria-label={`Night ${c.night} campsite`}
                aria-pressed={selectedId === c.id}
                className="absolute flex h-6 w-6 -translate-x-1/2 -translate-y-1/2 cursor-grab items-center justify-center rounded-full border-2 border-ink bg-accent text-xs font-bold text-accentink active:cursor-grabbing"
                style={{ left: `${leftPct(c.distanceMi)}%`, top: `${topPct(c.eleFt ?? minE)}%` }}
              >
                {c.night}
              </button>
            ))}
          </div>
          <div className="readout mt-1.5 flex justify-between text-xs text-muted">
            <span>0 mi</span>
            <span>{maxD.toFixed(1)} mi</span>
          </div>
          {selectedSite && (
            <div className="mt-3 flex items-center justify-between gap-3 rounded border border-ink/15 bg-panel2 px-3 py-2 text-sm">
              <span>Night {selectedSite.night} campsite · {selectedSite.distanceMi.toFixed(1)} mi</span>
              <button type="button" className="shrink-0 font-semibold text-accent hover:underline" onClick={() => removeSite(selectedSite)}>
                Remove campsite
              </button>
            </div>
          )}
        </div>
      </div>
      </div>
    </div>
  );
}
