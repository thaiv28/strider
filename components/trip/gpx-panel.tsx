"use client";

import { useState, useTransition } from "react";
import { Card, Button } from "@/components/ui";
import { uploadGpx, removeGpx, restoreGpx, updateTrip } from "@/app/trips/actions";
import { RoutePanel, type Campsite, type RoutePoint } from "./route-panel";
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

export function GpxPanel({
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

  if (!gpx) {
    return (
      <div>
        <div className="eyebrow">Route (GPX)</div>
        <Card className="mt-2 p-4">
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
              className="text-sm file:mr-3 file:rounded-md file:border file:bg-panel2 file:px-3 file:py-1.5 file:text-sm"
            />
            <Button type="submit" disabled={pending}>
              {pending ? "Reading…" : "Upload"}
            </Button>
            {error && <span className="text-sm text-accent">{error}</span>}
          </form>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between">
        <div className="eyebrow">Route (GPX)</div>
        <div className="-my-2 flex items-center gap-1">
          <button onClick={downloadGpx} disabled={!gpx.points.length} className="eyebrow px-2 py-2 hover:text-accent disabled:opacity-40">
            download
          </button>
          <button
            onClick={() => {
              const g = { ...gpx };
              start(async () => await removeGpx(tripId));
              pushUndo("Removed route", () => restoreGpx(tripId, g));
            }}
            className="eyebrow px-2 py-2 hover:text-accent"
          >
            remove
          </button>
        </div>
      </div>
      <Card className="mt-2 overflow-hidden">
        <RoutePanel
          tripId={tripId}
          points={gpx.points}
          minFt={gpx.minEleFt}
          maxFt={gpx.maxEleFt}
          campsites={campsites}
          pushUndo={pushUndo}
        />
        <div className="grid grid-cols-2 gap-4 border-t px-4 py-3 sm:grid-cols-4">
          <Stat label="Distance" value={gpx.distanceMi != null ? `${gpx.distanceMi} mi` : "—"} />
          <Stat label="Elev gain" value={gpx.elevationGainFt != null ? `${gpx.elevationGainFt.toLocaleString()} ft` : "—"} />
          <Stat label="Low / high" value={gpx.minEleFt != null && gpx.maxEleFt != null ? `${gpx.minEleFt.toLocaleString()}–${gpx.maxEleFt.toLocaleString()} ft` : "—"} />
          <Stat
            label="Start"
            value={gpx.startLat != null && gpx.startLon != null ? `${gpx.startLat.toFixed(4)}, ${gpx.startLon.toFixed(4)}` : "—"}
          />
        </div>
        {(gpx.distanceMi != null || gpx.elevationGainFt != null) && (
          <div className="flex items-center gap-3 border-t px-4 py-2.5">
            <Button
              variant="outline"
              disabled={pending}
              onClick={() => {
                const prev = { distanceMi: current.distanceMi, elevationGainFt: current.elevationGainFt, lat: current.lat, lon: current.lon };
                const patch: Parameters<typeof updateTrip>[1] = {
                  distanceMi: gpx.distanceMi != null ? String(gpx.distanceMi) : null,
                  elevationGainFt: gpx.elevationGainFt,
                };
                if (current.lat == null && gpx.startLat != null) patch.lat = String(gpx.startLat);
                if (current.lon == null && gpx.startLon != null) patch.lon = String(gpx.startLon);
                start(async () => await updateTrip(tripId, patch));
                pushUndo("Applied GPX values", () => updateTrip(tripId, prev));
              }}
            >
              Use these for the trip
            </Button>
            <span className="text-xs text-muted">Overwrites the trip's distance & elevation gain.</span>
          </div>
        )}
      </Card>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <div className="eyebrow">{label}</div>
      <div className="readout truncate text-sm">{value}</div>
    </div>
  );
}

