"use client";

import { useEffect, useRef, useState, useTransition, type ReactNode } from "react";
import dynamic from "next/dynamic";
import { Chip } from "@/components/ui";
import { addCampsite, moveCampsite, removeCampsite } from "@/app/trips/actions";

// Reserve the map's height while the client-only Leaflet chunk loads so the
// content below doesn't jump once it mounts.
const RouteMap = dynamic(() => import("./route-map"), {
  ssr: false,
  loading: () => <div className="h-full w-full animate-pulse bg-map" />,
});

export type RoutePoint = { d: number; lat: number; lon: number; e: number };
export type Campsite = { id: number; night: number; distanceMi: number; lat: number | null; lon: number | null; eleFt: number | null };

export function RoutePanel({
  readOnly = false,
  tripId,
  points,
  minFt,
  maxFt,
  campsites,
  pushUndo,
  mapActions,
}: {
  mapActions?: ReactNode;
  readOnly?: boolean;
  tripId?: number;
  points: RoutePoint[];
  minFt: number | null;
  maxFt: number | null;
  campsites: Campsite[];
  pushUndo?: (label: string, run: () => Promise<void>) => void;
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
    if (readOnly || tripId == null) return;
    setSelectedId(null);
    start(async () => {
      const id = await addCampsite(tripId, { distanceMi: p.d, lat: p.lat, lon: p.lon, eleFt: p.e });
      pushUndo?.("Added campsite", () => removeCampsite(id, tripId));
    });
  };
  const removeSite = (site: Campsite) => {
    if (readOnly || tripId == null) return;
    setSelectedId(null);
    start(async () => await removeCampsite(site.id, tripId));
    pushUndo?.("Removed campsite", async () => {
      await addCampsite(tripId, { distanceMi: site.distanceMi, lat: site.lat, lon: site.lon, eleFt: site.eleFt });
    });
  };
  const onUp = (e: React.PointerEvent) => {
    if (readOnly || tripId == null) return;
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
    pushUndo?.("Moved campsite", () =>
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
    <div className="flex h-full min-h-0 w-full flex-col">
      <div className={`relative isolate z-0 min-h-0 flex-1 bg-map sm:h-[380px] sm:flex-none ${readOnly ? "sm:[&_.leaflet-top.leaflet-right]:!right-[104px]" : "[&_.leaflet-top.leaflet-right]:!right-[52px] sm:[&_.leaflet-top.leaflet-right]:!right-[156px]"} [&_.leaflet-top.leaflet-right]:!top-3 [&_.leaflet-top.leaflet-right]:mr-3 [&_.leaflet-control-layers]:!m-0 [&_.leaflet-control-layers]:!rounded-xl [&_.leaflet-control-layers-toggle]:!h-11 [&_.leaflet-control-layers-toggle]:!w-11`}>
        <RouteMap fill className="h-full" track={points.map((p) => [p.lat, p.lon])} campsites={shown} preview={hover ? { lat: hover.lat, lon: hover.lon } : null} />
        <div className="absolute right-3 top-3 z-[1000] flex gap-2">{mapActions}</div>
      </div>
      <section aria-label="Elevation" className="flex-none border-t bg-panel px-4 pt-2.5 pb-3">
        <div className="flex items-baseline justify-between gap-3 text-[13px]">
          <div className="min-w-0 whitespace-nowrap"><strong className="text-sm">Elevation</strong><span className="ml-2.5 tabular-nums text-muted">{hover ? `${hover.d.toFixed(1)} mi · ${hover.e.toLocaleString()} ft` : `${minE.toLocaleString()} to ${maxE.toLocaleString()} ft`}</span></div>
          {!readOnly && <div className="text-right text-muted">{selectedSite ? <><span className="tabular-nums">Night {selectedSite.night} · {selectedSite.distanceMi.toFixed(1)} mi · {selectedSite.eleFt?.toLocaleString() ?? "—"} ft</span><button type="button" className="ml-3 font-semibold text-signal hover:underline" onClick={() => removeSite(selectedSite)}>Remove campsite</button></> : <><span className="whitespace-nowrap sm:hidden">Tap to add a camp</span><span className="hidden sm:inline">Click the profile to add a campsite</span></>}</div>}
        </div>
        <div
            ref={boxRef}
            data-testid="elevation-profile"
            className={`relative mt-1.5 h-[88px] sm:h-[92px] w-full ${readOnly ? "" : "cursor-crosshair touch-none"} select-none`}
            onPointerDown={(e) => {
              if (readOnly) return;
              if (e.pointerType === "touch") touchStart.current = { x: e.clientX, y: e.clientY };
            }}
            onPointerMove={onMove}
            onPointerLeave={() => !drag && setHover(null)}
            onPointerUp={onUp}
            onPointerCancel={() => { touchStart.current = null; setDrag(null); }}
            onClick={onClick}
          >
            <svg viewBox={`0 0 100 ${H}`} preserveAspectRatio="none" className="absolute inset-0 h-full w-full">
              <polygon points={area} fill="var(--elevation)" />
              <polyline points={line} fill="none" stroke="var(--route)" strokeWidth={2} vectorEffect="non-scaling-stroke" />
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
              </>
            )}
            {shown.map((c) => (
              <button
                key={c.id}
                onPointerDown={(e) => {
                  if (readOnly) return;
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
                  if (readOnly) return;
                  e.preventDefault();
                  e.stopPropagation();
                  removeSite(c);
                }}
                disabled={readOnly}
                aria-label={`Night ${c.night} campsite`}
                aria-pressed={selectedId === c.id}
                className={`${selectedId === c.id ? "ring-2 ring-route ring-offset-2 ring-offset-panel" : ""} absolute flex h-6 w-6 -translate-x-1/2 -translate-y-1/2 disabled:cursor-default cursor-grab items-center justify-center rounded-full border-2 border-panel bg-accent text-xs font-bold text-accentink active:cursor-grabbing`}
                style={{ left: `${leftPct(c.distanceMi)}%`, top: `${topPct(c.eleFt ?? minE)}%` }}
              >
                {c.night}
              </button>
            ))}
          </div>
      </section>
    </div>
  );
}

// Owner and shared plans use the same responsive map/section geometry.
export const tripLayout = {
  frame: "flex h-[100dvh] min-w-0 flex-col overflow-hidden bg-canvas pb-[calc(4.25rem+env(safe-area-inset-bottom))] sm:h-auto sm:overflow-visible sm:pb-0",
  split: "flex min-h-0 flex-1 flex-col sm:flex-none",
  map: (phoneView: string) => `relative min-h-0 flex-1 ${phoneView === "map" ? "flex" : "hidden"} flex-col sm:mx-auto sm:-mt-16 sm:flex sm:w-full sm:max-w-[1136px] sm:flex-none sm:px-6`,
  content: (hasRoute: boolean, phoneView: string) => `mx-auto min-h-0 w-full min-w-0 max-w-[1136px] flex-1 overflow-y-auto px-4 pt-4 pb-24 sm:mt-[18px] sm:flex-none sm:overflow-visible sm:px-6 sm:pt-0 sm:pb-8 ${phoneView === "map" && hasRoute ? "hidden sm:block" : ""}`,
};

export function TripSectionBar<T extends string>({ sections, hasRoute, value, onChange }: { sections: readonly (readonly [T, string])[]; hasRoute: boolean; value: T | "map"; onChange: (key: T | "map") => void }) {
  return <nav aria-label="Trip sections" className="fixed inset-x-0 bottom-0 z-[2000] flex gap-1.5 overflow-x-auto border-t bg-panel px-3 pt-2 pb-[calc(0.5rem+env(safe-area-inset-bottom))] sm:hidden">
    {hasRoute && <Chip selected={value === "map"} onClick={() => onChange("map")}>Map</Chip>}
    {sections.map(([key, label]) => <Chip key={key} selected={value === key} onClick={() => onChange(key)}>{label}</Chip>)}
  </nav>;
}

export function ElevationSparkline({ points }: { points: { d: number; e: number }[] }) {
  const low = Math.min(...points.map((p) => p.e));
  const span = Math.max(...points.map((p) => p.e)) - low || 1;
  const distance = points[points.length - 1]?.d || 1;
  return <svg viewBox="0 0 100 32" aria-label="Elevation summary" role="img" className="h-10 w-24"><polyline points={points.map((p) => `${p.d / distance * 100},${30 - (p.e - low) / span * 28}`).join(" ")} fill="none" stroke="var(--route)" strokeWidth="1.5" /></svg>;
}
