import Link from "next/link";
import { PageHeader, PageBody } from "@/components/page-header";

import { BaseBar, type CatSlice } from "@/components/gear/base-bar";
import { RouteMapThumb } from "@/components/trip/route-map-thumb";

export type TripCard = {
  id: number;
  name: string;
  status: string;
  updatedAt: Date | string;
  startDate: string | null;
  nights: number | null;
  region: string | null;
  areaType: string | null;
  distanceMi: number;
  elevationFt: number | null;
  track: [number, number][] | null;
};

export type NextReady = { gear: boolean; meals: boolean; gpx: boolean };
type Props = { trips: TripCard[]; baseG: number; byCategory: CatSlice[]; nextReady?: NextReady };

function season(trips: TripCard[]) {
  const done = trips.filter((t) => t.status === "completed");
  return {
    trips: done.length,
    miles: done.reduce((s, t) => s + t.distanceMi, 0),
    elevation: done.reduce((s, t) => s + (t.elevationFt ?? 0), 0),
    nights: done.reduce((s, t) => s + (t.nights ?? 0), 0),
  };
}

export function HomeView({ trips, baseG, byCategory, nextReady }: Props) {
  const s = season(trips);
  const next = trips.find((t) => t.status === "planned");
  const stats = [
    { label: "Trips", value: s.trips.toString(), unit: "" },
    { label: "Miles", value: s.miles.toLocaleString(), unit: "mi" },
    { label: "Elevation", value: (s.elevation / 1000).toFixed(1), unit: "k ft" },
    { label: "Nights out", value: s.nights.toString(), unit: "" },
  ];

  const recent = [...trips].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime() || b.id - a.id).slice(0, 3);
  const days = next?.startDate ? Math.max(0, Math.ceil((new Date(`${next.startDate}T00:00:00`).getTime() - Date.now()) / 86400000)) : null;
  const where = (t: TripCard) => [t.region, t.startDate ?? "Date TBD"].filter(Boolean).join(" · ");
  return (
    <>
      <PageHeader title="Basecamp">
        <section aria-label="All-time stats" className="grid grid-cols-4 gap-2 pt-3">
          {stats.map((st) => <div key={st.label}>
            <div className="font-display text-[27px] font-bold leading-tight tabular-nums sm:text-[38px]">{st.value}<span className="ml-1 text-xs font-normal sm:text-[15px]">{st.unit}</span></div>
            <div className="text-xs text-headermuted sm:text-sm">{st.label}</div>
          </div>)}
        </section>
      </PageHeader>
      <PageBody className="-mt-10 space-y-5 sm:-mt-12">
        {next && <Link href={`/trips/${next.id}`} className="card flex flex-col overflow-hidden transition hover:border-accent/50 sm:flex-row">
          {next.track && next.track.length > 1 && <RouteMapThumb track={next.track} className="h-36 w-full shrink-0 sm:h-auto sm:min-h-52 sm:w-2/5" />}
          <div className="min-w-0 flex-1 p-4 sm:p-5">
            <div className="text-sm font-bold text-signal">Next up{days !== null && ` · in ${days} days`}</div>
            <h2 className="mt-1 font-display text-[26px] font-bold leading-tight">{next.name}</h2>
            <div className="mt-1 text-sm text-muted">{where(next)}</div>
            <div className="mt-4 grid grid-cols-3 gap-2 text-sm">
              {([{label: "Gear", ok: nextReady?.gear, needs: "Needs gear"}, {label: "Meals", ok: nextReady?.meals, needs: "Needs meals"}, {label: "Route", ok: nextReady?.gpx, needs: "Needs GPX"}]).map((r) => <div key={r.label}><div className="font-bold">{r.label}</div><div className={r.ok ? "text-muted" : "font-bold text-signal"}>{r.ok ? "Ready" : r.needs}</div></div>)}
            </div>
          </div>
        </Link>}
        <BaseBar baseG={baseG} byCategory={byCategory} />
        <section aria-label="Recent trips">
          <div className="flex items-center justify-between gap-3">
            <h2 className="font-display text-[22px] font-bold">Recent trips</h2>
            <Link href="/trips" className="inline-flex min-h-11 items-center font-semibold text-signal">All trips</Link>
          </div>
          <div className="mt-2 grid grid-cols-1 gap-3 sm:grid-cols-[repeat(auto-fill,minmax(200px,1fr))]">
            {recent.map((t) => <a key={t.id} href={`/trips/${t.id}`} className="card overflow-hidden transition hover:border-accent/50">
              {t.track && t.track.length > 1 ? <RouteMapThumb track={t.track} className="h-28 w-full" /> : <div className="h-28 bg-map" aria-hidden="true" />}
              <div className="p-3"><div className="font-bold">{t.name}</div><div className="mt-1 text-xs text-muted">{where(t)}</div>
                <div className="mt-2 text-sm tabular-nums">{t.distanceMi || "—"} mi · {t.elevationFt ? `${(t.elevationFt / 1000).toFixed(1)}k ft` : "—"} · {t.nights ?? "—"} nights</div>
              </div>
            </a>)}
          </div>
        </section>
      </PageBody>
    </>
  );
}
