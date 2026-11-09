"use client";

import { useState, useTransition } from "react";
import { OptionsMenu } from "./options-menu";
import { Card, Button } from "@/components/ui";
import { uploadGpx, removeGpx, restoreGpx, reverseGpx, updateTrip, clearCampsites } from "@/app/trips/actions";
import type { Campsite, RoutePoint } from "./route-panel";
import { buildGpx, gpxFilename } from "@/lib/gpx-export";

export type Gpx = {
  filename: string | null;
  startLat: number | null;
  startLon: number | null;
  distanceMi: number | null;
  elevationGainFt: number | null;
  minEleFt: number | null;
  maxEleFt: number | null;
  track: [number, number][];
  profile: { d: number; e: number }[];
  points: RoutePoint[];
};

export function RouteActions({
  tripId,
  tripName,
  gpx,
  pushUndo,
  current,
  campsites,
}: {
  tripId: number;
  tripName: string;
  gpx: Gpx | null;
  pushUndo: (label: string, run: () => Promise<void>) => void;
  current: { distanceMi: string | null; elevationGainFt: number | null; lat: string | null; lon: string | null };
  campsites: Campsite[];
}) {
  const [pending, start] = useTransition();
  const [error, setError] = useState<string | null>(null);

  const downloadGpx = () => {
    if (!gpx) return;
    const xml = buildGpx(tripName, gpx.points, campsites);
    const url = URL.createObjectURL(new Blob([xml], { type: "application/gpx+xml" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = gpxFilename(tripName);
    a.click();
    URL.revokeObjectURL(url);
  };

  if (!gpx?.points.length) {
    return (
      <Card className="p-4">
          <h2 className="text-[17px] font-bold">Add a route</h2>
          <p className="mt-1 mb-3 text-sm text-muted">Upload a GPX file to see the map and elevation profile.</p>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              const fd = new FormData(e.currentTarget);
              start(async () => setError(await uploadGpx(tripId, fd)));
            }}
            className="flex flex-wrap items-center gap-3"
          >
            <input
              type="file"
              name="gpx"
              accept=".gpx,application/gpx+xml"
              className="min-w-0 max-w-full text-sm file:mr-3 file:rounded-md file:border file:bg-panel2 file:px-3 file:py-1.5 file:text-sm"
            />
            <Button type="submit" disabled={pending}>
              {pending ? "Reading…" : "Upload"}
            </Button>
            {error && <span className="text-sm text-accent">{error}</span>}
          </form>
        </Card>
    );
  }

  const differs = (current.distanceMi == null ? null : Number(current.distanceMi)) !== gpx.distanceMi || current.elevationGainFt !== gpx.elevationGainFt;
  return (
    <OptionsMenu label="Route options">
      <button role="menuitem" onClick={downloadGpx}>Download GPX</button>
      <button role="menuitem" disabled={pending || gpx.points.length < 2} onClick={() => {
        start(async () => await reverseGpx(tripId));
        pushUndo("Reversed route", () => reverseGpx(tripId));
      }}>Reverse route</button>
      {differs && <button role="menuitem" disabled={pending} onClick={() => {
        const prev = { distanceMi: current.distanceMi, elevationGainFt: current.elevationGainFt, lat: current.lat, lon: current.lon };
        const patch: Parameters<typeof updateTrip>[1] = {
          distanceMi: gpx.distanceMi != null ? String(gpx.distanceMi) : null,
          elevationGainFt: gpx.elevationGainFt,
        };
        if (current.lat == null && gpx.startLat != null) patch.lat = String(gpx.startLat);
        if (current.lon == null && gpx.startLon != null) patch.lon = String(gpx.startLon);
        start(async () => await updateTrip(tripId, patch));
        pushUndo("Applied GPX values", () => updateTrip(tripId, prev));
      }}>Use route distance and gain</button>}
      {campsites.length > 0 && <button role="menuitem" onClick={() => confirm("Clear all campsites?") && start(async () => await clearCampsites(tripId))}>Clear campsites</button>}
      <button role="menuitem" onClick={() => {
        const previous = { ...gpx };
        start(async () => await removeGpx(tripId));
        pushUndo("Removed route", () => restoreGpx(tripId, previous));
      }}>Remove route</button>
    </OptionsMenu>
  );
}
