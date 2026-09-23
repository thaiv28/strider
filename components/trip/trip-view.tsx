"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, useTransition } from "react";
import { Card, Badge, Button, Input, Select, InfoBadge } from "@/components/ui";
import { fmtWeight, fmtOz, fmtLbs, ozToG } from "@/lib/util";
import { TripGearList } from "./trip-gear-list";
import { GpxPanel, type Gpx } from "./gpx-panel";
import { WeatherPanel } from "./weather-panel";
import { TripFoodPanel, ShoppingList, type MealOpt, type IngOpt } from "./trip-food-panel";
import { fuelPlan, GAS_PER_LITER_G, CANISTERS } from "@/lib/fuel";
import { ReportPanel } from "./report-panel";
import { PermitCard, type PermitMeta } from "./permit-card";
import { ShareMenu } from "./share-menu";
import type { TripFood, ShoppingItem } from "@/lib/trip";
import type { ReportData } from "@/lib/report";
import {
  seedTripFromLoadout,
  addTripGear,
  removeTripGear,
  setTripGearClass,
  setTripGearQty,
  setTripGearPacked,
  clearTripGear,
  moveTripGear,
  updateTrip,
  upsertTripDay,
  addTripDay,
  deleteTripDay,
  restoreTripGear,
  deleteTrip,
} from "@/app/trips/actions";
import type { LoadoutLite } from "@/lib/gear";
import type { TripGearRow } from "@/lib/trip";

type Trip = {
  id: number;
  name: string;
  status: "idea" | "planned" | "completed";
  startDate: string | null;
  nights: number | null;
  partySize: number;
  lat: string | null;
  lon: string | null;
  distanceMi: string | null;
  elevationGainFt: number | null;
  foodGPerDay: number;
  waterGPerDay: number;
  fuelGPerDay: number;
  trailhead: string | null;
  permitRequired: boolean | null;
  permitNotes: string | null;
  drivingNotes: string | null;
  waterSources: string | null;
  planningNotes: string | null;
  tripReport: string | null;
  shareToken: string | null;
  editToken: string | null;
};
type LibItem = { id: number; name: string; category: string | null; weightG: number | null; quantity: number; defaultWeightClass: "base" | "worn" | "consumable" };
type Weights = { baseG: number; wornG: number; foodG: number; waterG: number; fuelG: number; consumableG: number; packG: number; skinOutG: number };

const tone = (s: string) => (s === "completed" ? "current" : s === "planned" ? "wishlist" : "neutral");

export function TripView(props: {
  isOwner: boolean;
  trip: Trip;
  region: string | null;
  areaType: string | null;
  trailName: string | null;
  effectiveDistance: number | null;
  effectiveElevation: number | null;
  lat: number | null;
  lon: number | null;
  days: number;
  cookWaterMlPerPerson: number;
  weights: Weights;
  groups: { category: string; rows: TripGearRow[] }[];
  gearCount: number;
  packedCount: number;
  dayBreakdown: { id: number; dayNumber: number; distanceMi: string | null; elevationGainFt: number | null }[];
  gpx: Gpx | null;
  campsites: { id: number; night: number; distanceMi: number; lat: number | null; lon: number | null; eleFt: number | null }[];
  perDayAuto: { day: number; distanceMi: number; elevationGainFt: number; lat: number | null; lon: number | null }[];
  loadouts: LoadoutLite[];
  library: LibItem[];
  food: TripFood;
  shopping: ShoppingItem[];
  groceryPeople: number;
  groceryRemoved: string[];
  meals: MealOpt[];
  ingredients: IngOpt[];
  report: ReportData;
  reportTemplate: string;
  tripPackingList: string;
  permit: PermitMeta;
  backHref: string;
  backLabel: string;
}) {
  const { trip, weights, groups, days } = props;
  const router = useRouter();
  const [pending, start] = useTransition();
  const del = () => {
    if (!confirm(`Delete "${trip.name}"? This removes its gear, food, route and campsites and can't be undone.`)) return;
    start(async () => {
      await deleteTrip(trip.id);
      router.push(props.backHref);
    });
  };
  const [picker, setPicker] = useState(false);
  type Tab = "logistics" | "planning" | "gear" | "food" | "report";
  const [tab, setTab] = useState<Tab>("logistics");
  const [generateReportRequest, setGenerateReportRequest] = useState(0);
  const tabKey = `bp_tripTab_${trip.id}`;
  useEffect(() => {
    const saved = localStorage.getItem(tabKey);
    if (saved && ["logistics", "planning", "gear", "food", "report"].includes(saved)) setTab(saved as Tab);
  }, [tabKey]);
  const pickTab = (k: Tab) => {
    setTab(k);
    localStorage.setItem(tabKey, k);
  };
  const generateReport = () => {
    pickTab("report");
    setGenerateReportRequest((n) => n + 1);
    requestAnimationFrame(() => document.getElementById("report")?.scrollIntoView({ behavior: "smooth", block: "start" }));
  };
  const save = (patch: Parameters<typeof updateTrip>[1]) => start(async () => await updateTrip(trip.id, patch));
  const oz = (g: number) => (g / 28.3495).toFixed(1);

  const inTrip = useMemo(
    () => new Set(groups.flatMap((g) => g.rows.map((r) => r.gearItemId))),
    [groups],
  );
  const autoMap = useMemo(() => new Map(props.perDayAuto.map((a) => [a.day, a])), [props.perDayAuto]);
  const dayRows = useMemo(() => [...props.dayBreakdown].sort((a, b) => a.dayNumber - b.dayNumber), [props.dayBreakdown]);

  // Undo stack for removed gear (Ctrl/⌘+Z or the toast button).
  type UndoEntry = { label: string; run: () => Promise<void> };
  const [undo, setUndo] = useState<UndoEntry[]>([]);
  const [toast, setToast] = useState<string | null>(null);
  const undoRef = useRef<UndoEntry[]>([]);
  useEffect(() => {
    undoRef.current = undo;
  }, [undo]);
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(null), 4000);
    return () => clearTimeout(id);
  }, [toast]);

  const pushUndo = useCallback((label: string, run: () => Promise<void>) => {
    setUndo((u) => [...u, { label, run }]);
    setToast(label);
  }, []);

  const doRemove = (id: number) => {
    const row = groups.flatMap((g) => g.rows).find((r) => r.id === id);
    start(async () => await removeTripGear(id, trip.id));
    if (row)
      pushUndo(`Removed ${row.name}`, () =>
        restoreTripGear(trip.id, {
          gearItemId: row.gearItemId,
          name: row.name,
          weightG: row.weightG,
          category: row.category,
          weightClass: row.weightClass,
          quantity: row.quantity,
          packed: row.packed,
        }),
      );
  };

  const doUndo = useCallback(() => {
    const u = undoRef.current;
    if (!u.length) return;
    const last = u[u.length - 1];
    setUndo(u.slice(0, -1));
    setToast(`Undone: ${last.label}`);
    start(async () => await last.run());
  }, []);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "z") return;
      const t = e.target as HTMLElement | null;
      const tag = t?.tagName?.toLowerCase();
      if (tag === "input" || tag === "textarea" || t?.isContentEditable) return;
      e.preventDefault();
      doUndo();
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [doUndo]);

  return (
    <div className="mx-auto max-w-4xl px-4 py-6 sm:px-8 sm:py-8">
      <Link href={props.backHref} className="eyebrow -my-2 inline-block py-2 hover:text-accent">
        ← {props.backLabel}
      </Link>

      <div className="mt-3 flex flex-wrap items-center gap-3">
        <input
          aria-label="Trip name"
          defaultValue={trip.name}
          onBlur={(e) => e.target.value.trim() && save({ name: e.target.value.trim() })}
          className="font-display min-w-0 flex-1 bg-transparent text-3xl font-bold tracking-tight outline-none"
        />
        <Select
          value={trip.status}
          onChange={(e) => save({ status: e.target.value as Trip["status"] })}
        >
          <option value="idea">idea</option>
          <option value="planned">planned</option>
          <option value="completed">completed</option>
        </Select>
        {props.isOwner && <ShareMenu tripId={trip.id} initialToken={trip.shareToken} initialEditToken={trip.editToken} onGenerateReport={generateReport} />}
        {props.isOwner && <Button variant="danger" onClick={del} aria-label="Delete trip" title="Delete trip">
          <Trash2 size={16} />
        </Button>}
      </div>
      <div className="readout mt-1 text-sm text-muted">
        {[props.region, props.areaType, trip.startDate ?? "date TBD"].filter(Boolean).join(" · ")}
        {pending && <span className="ml-2">· saving…</span>}
      </div>

      <div className="mt-5">
        <StrataBar weights={weights} />
      </div>

      <div className="mt-5 flex gap-1 overflow-x-auto overflow-y-hidden border-b">
        {([
          ["logistics", "Logistics"],
          ["planning", "Planning"],
          ["gear", "Gear"],
          ["food", "Food"],
          ["report", "Report"],
        ] as const).map(([k, label]) => (
          <button
            key={k}
            onClick={() => pickTab(k)}
            className={`-mb-px shrink-0 whitespace-nowrap border-b-2 px-4 py-2 text-sm font-medium transition ${
              tab === k ? "border-accent text-ink" : "border-transparent text-muted hover:text-ink"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="relative mt-4">
        {/* Sticky TOC lives in the left gutter so it never narrows the content. */}
        <div className="absolute inset-y-0 right-full mr-6 hidden xl:block">
          <SectionNav tab={tab} groups={groups} />
        </div>
        {tab === "logistics" && (
          <div className="space-y-6">
            <Card id="overview" className="scroll-mt-24 grid grid-cols-1 gap-4 p-4 sm:grid-cols-2">
              <Field label="Start date">
                <Input type="date" defaultValue={trip.startDate ?? ""} onBlur={(e) => save({ startDate: e.target.value || null })} />
              </Field>
              <Field label="Nights">
                <div className="rounded-[calc(var(--radius)*0.6)] border bg-panel2/40 px-3 py-1.5 text-sm text-muted">
                  {trip.nights ?? "—"}
                </div>
              </Field>
              <Field label="Region">
                <Input defaultValue={props.region ?? ""} onBlur={(e) => save({ region: e.target.value || null })} />
              </Field>
              <Field label="Area type">
                <Input defaultValue={props.areaType ?? ""} onBlur={(e) => save({ areaType: e.target.value || null })} />
              </Field>
              <Field label="Distance (mi)">
                <Input key={`d-${props.effectiveDistance ?? ""}`} type="number" step="0.1" defaultValue={props.effectiveDistance ?? ""} onBlur={(e) => save({ distanceMi: e.target.value || null })} />
              </Field>
              <Field label="Elevation gain (ft)">
                <Input key={`e-${props.effectiveElevation ?? ""}`} type="number" defaultValue={props.effectiveElevation ?? ""} onBlur={(e) => save({ elevationGainFt: e.target.value === "" ? null : Number(e.target.value) })} />
              </Field>
              <Field label="Party size (people)">
                <Input
                  key={`party-${trip.partySize}`}
                  type="number"
                  min="1"
                  step="1"
                  defaultValue={trip.partySize}
                  onBlur={(e) => {
                    const v = Math.max(1, Math.round(Number(e.target.value) || 1));
                    if (v !== trip.partySize) save({ partySize: v });
                  }}
                />
              </Field>
            </Card>

            <div id="by-day" className="scroll-mt-24">
              <div className="flex items-center justify-between">
                <div className="eyebrow">By day</div>
                <Button variant="outline" onClick={() => start(async () => await addTripDay(trip.id))}>+ Add day</Button>
              </div>
              <Card className="mt-2 overflow-hidden">
                {dayRows.length === 0 && <div className="px-4 py-3 text-sm text-muted">No days yet. Add a day, or drop campsites on the route below.</div>}
                {dayRows.map((d) => {
                  const auto = autoMap.get(d.dayNumber);
                  const dist = d.distanceMi ?? "";
                  const gain = d.elevationGainFt ?? "";
                  return (
                    <div key={d.id} className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 border-b px-3 py-3 last:border-0 sm:grid-cols-[3rem_minmax(0,1fr)_minmax(0,1fr)_7rem_1.5rem] sm:gap-x-3 sm:px-4 sm:py-2">
                      <span className="readout col-start-1 row-start-1 text-sm text-muted sm:col-auto sm:row-auto">Day {d.dayNumber}</span>
                      <div className="col-span-3 row-start-2 grid grid-cols-2 gap-2 sm:contents">
                        <Input key={`d${d.id}-${dist}`} aria-label={`Day ${d.dayNumber} distance in miles`} type="number" step="0.1" placeholder="mi" defaultValue={dist} onBlur={(e) => start(async () => await upsertTripDay(trip.id, d.dayNumber, { distanceMi: e.target.value || null }))} />
                        <Input key={`e${d.id}-${gain}`} aria-label={`Day ${d.dayNumber} elevation gain in feet`} type="number" placeholder="ft gain" defaultValue={gain} onBlur={(e) => start(async () => await upsertTripDay(trip.id, d.dayNumber, { elevationGainFt: e.target.value === "" ? null : Number(e.target.value) }))} />
                      </div>
                      <span className="readout col-start-2 row-start-1 truncate text-right text-xs text-muted sm:col-auto sm:row-auto sm:text-left">
                        {auto?.lat != null && auto?.lon != null ? `${auto.lat.toFixed(3)}, ${auto.lon.toFixed(3)}` : ""}
                      </span>
                      <button onClick={() => start(async () => await deleteTripDay(trip.id, d.id))} aria-label={`Delete day ${d.dayNumber}`} className="col-start-3 row-start-1 grid h-11 w-11 place-items-center justify-self-end rounded text-muted hover:bg-accent/10 hover:text-accent sm:col-auto sm:row-auto sm:h-auto sm:w-auto" title="Delete day">✕</button>
                    </div>
                  );
                })}
              </Card>
            </div>

            <div id="route" className="scroll-mt-24">
              <GpxPanel
                tripId={trip.id}
                tripName={trip.name}
                gpx={props.gpx}
                pushUndo={pushUndo}
                current={{ distanceMi: trip.distanceMi, elevationGainFt: trip.elevationGainFt, lat: trip.lat, lon: trip.lon }}
                campsites={props.campsites}
              />
            </div>

            <div id="weather" className="scroll-mt-24">
              <WeatherPanel
                lat={props.lat}
                lon={props.lon}
                startDate={trip.startDate}
                days={days}
                nights={trip.nights ?? props.campsites.length}
                campsites={props.campsites.map((c) => ({ night: c.night, lat: c.lat, lon: c.lon }))}
                endCoord={
                  props.perDayAuto.length ? { lat: props.perDayAuto[props.perDayAuto.length - 1].lat, lon: props.perDayAuto[props.perDayAuto.length - 1].lon } : null
                }
                elevationFt={props.gpx?.maxEleFt ?? null}
              />
            </div>
          </div>
        )}

        {tab === "planning" && (
          <div className="space-y-6">
            <div id="plan-details" className="scroll-mt-24 grid grid-cols-1 gap-4 sm:grid-cols-2">
              <LineBox label="Trailhead" defaultValue={trip.trailhead} onSave={(v) => save({ trailhead: v })} />
              <LineBox label="Permits" defaultValue={trip.permitNotes} onSave={(v) => save({ permitNotes: v })} />
              <LineBox label="Water sources" defaultValue={trip.waterSources} onSave={(v) => save({ waterSources: v })} />
              <LineBox label="Driving / access" defaultValue={trip.drivingNotes} onSave={(v) => save({ drivingNotes: v })} />
            </div>
            <div id="permit" className="scroll-mt-24">
              <PermitCard tripId={trip.id} permit={props.permit} />
            </div>
            <div id="plan-notes" className="scroll-mt-24">
              <NoteCard label="Planning notes" defaultValue={trip.planningNotes} onSave={(v) => save({ planningNotes: v })} rows={8} />
            </div>
          </div>
        )}

        {tab === "gear" && (
          <div>
            <div className="flex items-center justify-between gap-2">
              <span className="readout text-sm text-muted">{props.packedCount}/{props.gearCount} packed</span>
              {props.gearCount > 0 && (
                <div className="flex items-center gap-2">
                  <Button variant="ghost" onClick={() => confirm("Remove all gear from this trip?") && start(async () => await clearTripGear(trip.id))}>
                    Clear all
                  </Button>
                  <Button onClick={() => setPicker(true)}>+ Add gear</Button>
                </div>
              )}
            </div>
            {props.gearCount === 0 ? (
              <SeedPanel tripId={trip.id} loadouts={props.loadouts} onSeed={(lid) => start(async () => await seedTripFromLoadout(trip.id, lid))} />
            ) : (
              <div className="mt-3">
                <TripGearList
                  groups={groups}
                  onPacked={(id, v) => start(async () => await setTripGearPacked(id, trip.id, v))}
                  onQty={(id, qty) => start(async () => await setTripGearQty(id, trip.id, qty))}
                  onClass={(id, cls) => start(async () => await setTripGearClass(id, trip.id, cls))}
                  onRemove={doRemove}
                  onMove={(id, toCat, ordered) => start(async () => await moveTripGear(id, trip.id, toCat, ordered))}
                />
              </div>
            )}
          </div>
        )}

        {tab === "food" && (
          <div className="space-y-6">
            <div id="water-fuel" className="scroll-mt-24">
              <div className="eyebrow">Water &amp; fuel</div>
              <Card className="mt-2 grid grid-cols-1 gap-4 p-4 sm:grid-cols-2">
                {([
                  ["Water", "waterGPerDay", trip.waterGPerDay],
                  ["Fuel", "fuelGPerDay", trip.fuelGPerDay],
                ] as const).map(([label, key, total]) => (
                  <div key={key}>
                    <div className="eyebrow">{label} — oz</div>
                    <Input
                      key={`${key}-${total}`}
                      type="number"
                      step="0.1"
                      defaultValue={total > 0 ? oz(total) : ""}
                      onBlur={(e) => {
                        const g = e.target.value === "" ? 0 : ozToG(Number(e.target.value));
                        if (g !== total) save({ [key]: g } as never);
                      }}
                      className="mt-1"
                    />
                  </div>
                ))}
              </Card>
            </div>
            <div id="meals" className="scroll-mt-24">
              <TripFoodPanel tripId={trip.id} food={props.food} meals={props.meals} ingredients={props.ingredients} />
            </div>
            <div id="fuel-calc" className="scroll-mt-24">
              <FuelCalculator
                cookWaterMlPerPerson={props.cookWaterMlPerPerson}
                partySize={trip.partySize}
                carriedFuelG={weights.fuelG}
                onApply={(g) => save({ fuelGPerDay: g })}
              />
            </div>
            <div id="shopping" className="scroll-mt-24">
              <ShoppingList
                items={props.shopping}
                partySize={trip.partySize}
                people={props.groceryPeople}
                removed={props.groceryRemoved}
                tripName={trip.name}
                onPeople={(n) => save({ groceryPeople: n })}
                onRemoved={(keys) => save({ groceryRemoved: keys })}
              />
            </div>
          </div>
        )}

        {tab === "report" && (
          <div className="space-y-6">
            <div id="report" className="scroll-mt-24">
              <ReportPanel tripId={trip.id} data={props.report} template={props.reportTemplate} packingList={props.tripPackingList} generateRequest={generateReportRequest} />
            </div>
            <div id="report-notes" className="scroll-mt-24">
              {props.isOwner && <NoteCard label="Trip report (private notes)" defaultValue={trip.tripReport} onSave={(v) => save({ tripReport: v })} rows={14} />}
            </div>
          </div>
        )}
      </div>

      {toast && (
        <div className="fixed bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center gap-3 rounded-full border bg-panel px-4 py-2 text-sm shadow-lg">
          <span>{toast}</span>
          {undo.length > 0 && (
            <button onClick={doUndo} className="font-medium text-accent">
              Undo <span className="text-muted">⌘Z</span>
            </button>
          )}
        </div>
      )}

      {picker && (
        <GearPicker
          library={props.library.filter((i) => !inTrip.has(i.id))}
          onPick={(id) => start(async () => await addTripGear(trip.id, id))}
          onClose={() => setPicker(false)}
        />
      )}
    </div>
  );
}

function FuelCalculator({
  cookWaterMlPerPerson,
  partySize,
  carriedFuelG,
  onApply,
}: {
  cookWaterMlPerPerson: number;
  partySize: number;
  carriedFuelG: number;
  onApply: (g: number) => void;
}) {
  const totalWaterMl = cookWaterMlPerPerson * partySize;
  const plan = fuelPlan(totalWaterMl);
  const applied = plan.packedWeightG === carriedFuelG;

  return (
    <div>
      <div className="flex items-center gap-1.5">
        <span className="eyebrow">Fuel calculator</span>
        <InfoBadge>
          Sums the cook water of every planned hot meal (× party size) and burns{" "}
          {GAS_PER_LITER_G} g of gas per liter on a Soto WindMaster in average
          temps/wind/altitude — cold, wind, or altitude use more. A 110 g canister
          holds {CANISTERS[0].net} g of gas; the estimate rounds up to the next size
          ({CANISTERS.map((c) => `${c.net}`).join(" / ")} g) and reports the full
          canister weight you'd pack, not just the gas.
        </InfoBadge>
      </div>
      <Card className="mt-2 space-y-4 p-4">
        {plan.neededGasG <= 0 ? (
          <p className="text-sm text-muted">
            No meals with cook water are planned, so no fuel is calculated.
          </p>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-x-6 gap-y-2 sm:grid-cols-4">
              <Metric label="Cook water" value={`${(totalWaterMl / 1000).toFixed(2)} L`} sub={`${cookWaterMlPerPerson} mL × ${partySize} ${partySize === 1 ? "person" : "people"}`} />
              <Metric label="Gas to boil it" value={`${plan.neededGasG} g`} sub={`${GAS_PER_LITER_G} g per L`} />
              <Metric label="Bring" value={plan.label} sub={`${plan.gasProvidedG} g gas`} />
              <Metric label="Canister weight" value={fmtWeight(plan.packedWeightG)} sub="full, incl. canister" />
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <Button variant="outline" disabled={applied} onClick={() => onApply(plan.packedWeightG)}>
                {applied ? "Applied to fuel" : `Use ${fmtWeight(plan.packedWeightG)}`}
              </Button>
              <span className="readout text-xs text-muted">
                Currently carried: {carriedFuelG > 0 ? fmtWeight(carriedFuelG) : "—"}
              </span>
            </div>
          </>
        )}
      </Card>
    </div>
  );
}

function Metric({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div>
      <div className="eyebrow">{label}</div>
      <div className="readout text-sm font-semibold text-ink">{value}</div>
      {sub && <div className="readout text-xs text-muted">{sub}</div>}
    </div>
  );
}

const slugify = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");

// Left-rail contents for the current tab; each entry scrolls to a section id.
// Hidden below lg. Gear entries are the trip's gear categories.
function SectionNav({ tab, groups }: { tab: string; groups: { category: string }[] }) {
  const items =
    tab === "logistics"
      ? [
          { id: "overview", label: "Overview" },
          { id: "by-day", label: "By day" },
          { id: "route", label: "Route & GPX" },
          { id: "weather", label: "Weather" },
        ]
      : tab === "planning"
      ? [
          { id: "plan-details", label: "Details" },
          { id: "permit", label: "Permit" },
          { id: "plan-notes", label: "Planning notes" },
        ]
      : tab === "gear"
      ? groups.map((g) => ({ id: `gear-${slugify(g.category)}`, label: g.category }))
      : tab === "food"
      ? [
          { id: "water-fuel", label: "Water & fuel" },
          { id: "meals", label: "Meals by day" },
          { id: "fuel-calc", label: "Fuel calculator" },
          { id: "shopping", label: "Grocery list" },
        ]
      : tab === "report"
      ? [
          { id: "report", label: "Report" },
          { id: "report-notes", label: "Private notes" },
        ]
      : [];
  if (items.length === 0) return null;
  const jump = (id: string) => document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  return (
    <nav className="w-40">
      <div className="sticky top-6 space-y-0.5">
        <div className="eyebrow mb-2">On this tab</div>
        {items.map((it) => (
          <button
            key={it.id}
            onClick={() => jump(it.id)}
            className="block w-full truncate rounded px-2 py-1 text-left text-sm text-muted transition hover:bg-panel2/60 hover:text-ink"
          >
            {it.label}
          </button>
        ))}
      </div>
    </nav>
  );
}

function StrataBar({ weights }: { weights: Weights }) {
  const strata = [
    { label: "Base", g: weights.baseG, color: "var(--cbase)" },
    { label: "Consumables", g: weights.consumableG, color: "var(--ccons)" },
    { label: "Worn", g: weights.wornG, color: "var(--cworn)" },
  ];
  const total = weights.skinOutG || 1;
  return (
    <Card className="mt-5 p-5">
      <div className="grid grid-cols-3 gap-4">
        <Readout label="Base" g={weights.baseG} />
        <Readout label="Pack" g={weights.packG} />
        <Readout label="Skin-out" g={weights.skinOutG} />
      </div>
      <div className="mt-4 flex h-3 overflow-hidden rounded-full border">
        {strata.map((s) => (s.g > 0 ? <div key={s.label} style={{ width: `${(s.g / total) * 100}%`, background: s.color }} title={`${s.label}: ${fmtWeight(s.g)}`} /> : null))}
      </div>
      <div className="mt-3 grid grid-cols-1 gap-x-6 gap-y-1.5 text-sm sm:grid-cols-3">
        {strata.map((s) => (
          <div key={s.label} className="flex items-center gap-2">
            <span className="h-2.5 w-2.5 rounded-sm" style={{ background: s.color }} />
            <span className="font-medium">{s.label}</span>
            <span className="readout ml-auto text-muted">{fmtOz(s.g)}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}

function Readout({ label, g }: { label: string; g: number }) {
  return (
    <div>
      <div className="eyebrow">{label}</div>
      <div className="readout mt-1 text-3xl font-bold leading-none">{fmtLbs(g)}</div>
    </div>
  );
}

function SeedPanel({ tripId, loadouts, onSeed }: { tripId: number; loadouts: LoadoutLite[]; onSeed: (loadoutId: number) => void }) {
  const [sel, setSel] = useState<number | "">(loadouts[0]?.id ?? "");
  return (
    <Card className="mt-3 flex flex-wrap items-center gap-3 p-5">
      <span className="text-sm text-muted">Start this trip's kit from a loadout:</span>
      <Select value={sel} onChange={(e) => setSel(Number(e.target.value))}>
        {loadouts.map((l) => (
          <option key={l.id} value={l.id}>
            {l.name}
          </option>
        ))}
      </Select>
      <Button onClick={() => sel !== "" && onSeed(sel)}>Copy in</Button>
    </Card>
  );
}

function GearPicker({ library, onPick, onClose }: { library: LibItem[]; onPick: (id: number) => void; onClose: () => void }) {
  const [q, setQ] = useState("");
  const shown = library.filter((i) => q === "" || i.name.toLowerCase().includes(q.toLowerCase()) || (i.category ?? "").toLowerCase().includes(q.toLowerCase()));
  return (
    <div className="fixed inset-0 z-[1100] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm" onClick={onClose}>
      <Card className="flex max-h-[80vh] w-full max-w-md flex-col p-5" onClick={(e) => e.stopPropagation()}>
        <div className="eyebrow">◇ Add gear</div>
        <Input autoFocus placeholder="Search library…" value={q} onChange={(e) => setQ(e.target.value)} className="mt-2" />
        <div className="mt-3 min-h-0 flex-1 overflow-y-auto">
          {shown.map((i) => (
            <button key={i.id} onClick={() => onPick(i.id)} className="flex w-full items-center gap-2 border-b py-2 text-left last:border-0 hover:text-accent">
              <span className="min-w-0 flex-1 truncate">{i.name}{i.quantity > 1 && <span className="text-muted"> ×{i.quantity}</span>}</span>
              <span className="eyebrow shrink-0">{i.category}</span>
            </button>
          ))}
          {shown.length === 0 && <div className="py-6 text-center text-sm text-muted">Nothing left to add.</div>}
        </div>
        <div className="mt-3 flex justify-end">
          <Button variant="outline" onClick={onClose}>Done</Button>
        </div>
      </Card>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block min-w-0 max-w-full">
      <span className="eyebrow mb-1 block">{label}</span>
      {children}
    </label>
  );
}

function LineBox({ label, defaultValue, onSave }: { label: string; defaultValue: string | null; onSave: (v: string | null) => void }) {
  return (
    <Card className="p-4">
      <div className="eyebrow">{label}</div>
      <textarea
        defaultValue={defaultValue ?? ""}
        onBlur={(e) => onSave(e.target.value.trim() || null)}
        rows={3}
        className="mt-2 w-full resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25"
      />
    </Card>
  );
}

function NoteCard({ label, defaultValue, onSave, rows = 6 }: { label: string; defaultValue: string | null; onSave: (v: string | null) => void; rows?: number }) {
  return (
    <Card className="p-4">
      <div className="eyebrow">{label}</div>
      <textarea
        defaultValue={defaultValue ?? ""}
        onBlur={(e) => onSave(e.target.value || null)}
        rows={rows}
        className="mt-2 w-full resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25"
      />
    </Card>
  );
}
