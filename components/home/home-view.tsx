import Link from "next/link";
import { Badge } from "@/components/ui";
import { Contours } from "@/components/contours";
import { BaseBar, type CatSlice } from "@/components/gear/base-bar";
import { RouteMapThumb } from "@/components/trip/route-map-thumb";

export type TripCard = {
  id: number;
  name: string;
  status: string;
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

const tone = (s: string) => (s === "completed" ? "current" : s === "planned" ? "wishlist" : "neutral");

function ReadyChip({ ok, children }: { ok?: boolean; children: React.ReactNode }) {
  return (
    <span className={ok ? "text-accent" : "text-muted"}>
      {ok ? "✓" : "✗"} {children}
    </span>
  );
}

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

  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-8 py-10">
      <div className="relative overflow-hidden rounded-[var(--radius)] border bg-panel px-6 py-7">
        <Contours className="pointer-events-none absolute -top-24 -right-16 h-80 w-80 text-cworn/20" />
        <div className="eyebrow">◇ Season 2026</div>
        <h1 className="font-display mt-1 text-3xl font-bold tracking-tight">Basecamp</h1>
        <div className="relative mt-5 grid grid-cols-2 gap-x-10 gap-y-4 sm:grid-cols-4">
          {stats.map((st) => (
            <div key={st.label}>
              <div className="eyebrow">{st.label}</div>
              <div className="readout mt-1 text-3xl font-bold leading-none">
                {st.value}
                <span className="ml-1 text-base font-normal text-muted">{st.unit}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-5 space-y-5">
        <BaseBar baseG={baseG} byCategory={byCategory} />

        {next && (
          <Link href={`/trips/${next.id}`} className="block">
            <div className="card p-5 transition hover:border-accent/50">
              <div className="flex items-center justify-between">
                <span className="eyebrow">◇ Next objective</span>
                <Badge tone="wishlist">planned</Badge>
              </div>
              <div className="mt-2 font-display text-2xl font-bold">{next.name}</div>
              <div className="readout mt-1 text-sm text-muted">
                {[next.region, next.areaType, next.startDate ?? "date TBD"].filter(Boolean).join(" · ")}
              </div>
              <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
                <ReadyChip ok={nextReady?.gear}>gear</ReadyChip>
                <ReadyChip ok={nextReady?.meals}>meals</ReadyChip>
                <ReadyChip ok={nextReady?.gpx}>gpx</ReadyChip>
              </div>
            </div>
          </Link>
        )}
      </div>

      <div className="mt-9 flex items-baseline justify-between">
        <h2 className="font-display text-lg font-bold">Trips</h2>
        <span className="eyebrow">{trips.length} logged</span>
      </div>
      <div className="mt-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {trips.map((t) => (
          <Link key={t.id} href={`/trips/${t.id}`}>
            <div className="card h-full overflow-hidden p-4 transition hover:border-accent/50">
              <div className="flex items-start justify-between gap-2">
                <span className="font-display font-semibold">{t.name}</span>
                <Badge tone={tone(t.status)}>{t.status}</Badge>
              </div>
              <div className="eyebrow mt-1">{t.region ?? "—"}</div>
              {t.track && t.track.length > 1 && <RouteMapThumb track={t.track} className="mt-3 h-28 w-full" />}
              <div className="readout mt-3 flex gap-4 text-sm text-muted">
                <span>{t.distanceMi || "—"} mi</span>
                <span>{t.nights ?? "—"} nt</span>
                <span>{t.elevationFt ? `${(t.elevationFt / 1000).toFixed(1)}k ft` : "—"}</span>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
