"use client";

import { useEffect, useState } from "react";
import { Snowflake } from "lucide-react";
import { Card } from "@/components/ui";
import { prettyDate } from "@/lib/weather";
import { getSnowDepths } from "@/app/weather/actions";
import type { TripDateSnow } from "@/lib/snow";

export type SnowDay = { key: string; label: string; note?: string; lat: number | null; lon: number | null; date: string };
type SnowData = { now: number | null; days: (TripDateSnow | null)[] };

const located = (lat: number | null, lon: number | null): boolean =>
  lat != null && lon != null && Number.isFinite(lat) && Number.isFinite(lon);

// One server call covers today's depth at the trip start plus each day's depth.
// null means snow is unavailable (no coordinates, or a signed-out share viewer).
function useSnow(lat: number | null, lon: number | null, days: SnowDay[]): SnowData | "loading" | null {
  const key = [lat, lon, ...days.map((d) => `${d.lat},${d.lon},${d.date}`)].join("|");
  const [snow, setSnow] = useState<SnowData | "loading" | null>(located(lat, lon) ? "loading" : null);

  useEffect(() => {
    if (!located(lat, lon)) {
      setSnow(null);
      return;
    }
    let live = true;
    setSnow("loading");
    const points = days.map((d) => (located(d.lat, d.lon) ? d : null));
    const queries = [{ lat: lat!, lon: lon!, date: null }, ...points.flatMap((d) => (d ? [{ lat: d.lat!, lon: d.lon!, date: d.date }] : []))];
    getSnowDepths(queries)
      .then(([now, ...rest]) => {
        if (!live) return;
        let k = 0;
        setSnow({ now: now.nowIn, days: points.map((d) => (d ? rest[k++].trip : null)) });
      })
      .catch(() => live && setSnow(null));
    return () => {
      live = false;
    };
    // `key` captures every coordinate and date in the request.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  return snow;
}

// Same basis vocabulary as the weather rows above it.
const BASIS: Record<TripDateSnow["kind"], string> = { observed: "historical", forecast: "forecast", typical: "typical" };

export function SnowPanel({ lat, lon, startDate, days }: { lat: number | null; lon: number | null; startDate: string | null; days: SnowDay[] }) {
  const snow = useSnow(lat, lon, days);
  // Signed-out share viewers and trips without coordinates get no section at all.
  if (snow === null) return null;
  const loading = snow === "loading";

  return (
    <div id="snow" className="scroll-mt-24">
      <div className="eyebrow">Snow depth</div>
      <Card className="mt-2 min-w-0 p-3 sm:p-4">
        <div className="min-w-0 divide-y overflow-hidden rounded-lg border">
          <div className="flex min-w-0 items-center gap-3 bg-panel2/40 px-3 py-3 sm:gap-4 sm:px-4">
            <div className="flex min-w-0 flex-1 items-center gap-3">
              <Snowflake aria-hidden className="h-6 w-6 shrink-0 text-ccons" strokeWidth={1.5} />
              <div className="min-w-0">
                <div className="text-sm font-medium">On the ground now</div>
                <div className="text-xs text-muted">At the trip start</div>
              </div>
            </div>
            <div className="eyebrow hidden w-20 shrink-0 text-right sm:block">current</div>
            <Depth inches={loading ? undefined : snow.now} className="w-16 shrink-0 text-xl" />
          </div>

          {days.length === 0 ? (
            <p className="px-3 py-3 text-sm text-muted sm:px-4">
              {startDate ? "Add a route or coordinates to see snow depth for each day." : "Set a start date to see snow depth for each day."}
            </p>
          ) : (
            days.map((d, i) => {
              const day = loading ? undefined : snow.days[i];
              return (
                <div key={d.key} className="flex min-w-0 items-center gap-3 px-3 py-2.5 sm:gap-4 sm:px-4">
                  <div className="min-w-0 flex-1 sm:flex sm:items-baseline sm:gap-4">
                    <div className="font-display font-semibold sm:w-24 sm:shrink-0">
                      {d.label}
                      {d.note && <span className="ml-1 text-xs font-normal text-muted">· {d.note}</span>}
                    </div>
                    <div className="readout text-xs text-muted">{prettyDate(d.date)}</div>
                  </div>
                  <div className="w-20 shrink-0 text-right">
                    {day && <div className="eyebrow">{BASIS[day.kind]}</div>}
                    {day?.kind === "typical" && <div className="readout text-[0.65rem] text-muted">{day.years}-yr avg</div>}
                  </div>
                  <Depth inches={day === undefined ? undefined : day?.inches ?? null} className="w-16 shrink-0 text-lg" />
                </div>
              );
            })
          )}
        </div>

        <p className="mt-3 px-1 text-xs leading-relaxed text-muted sm:px-2">
          Modeled by Open-Meteo at each point, so treat it as a planning signal rather than a trail report.
        </p>
      </Card>
    </div>
  );
}

// undefined = loading (skeleton), null = unavailable. Zero stays muted so real
// snow stands out down the column.
function Depth({ inches, className = "" }: { inches: number | null | undefined; className?: string }) {
  return (
    <div className={`readout text-right font-bold leading-tight ${className}`}>
      {inches === undefined ? (
        <span aria-label="Loading snow depth" className="inline-block h-[0.8em] w-[2.5em] animate-pulse rounded-sm bg-panel2 align-middle" />
      ) : inches == null ? (
        <span className="font-normal text-muted">—</span>
      ) : (
        <span className={inches === 0 ? "text-muted" : "text-ink"}>
          {inches}
          <span className="text-[0.6em] font-normal text-muted"> in</span>
        </span>
      )}
    </div>
  );
}
