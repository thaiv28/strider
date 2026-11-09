"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, useTransition } from "react";
import { Card, Button, Input, Select, InfoBadge, Chip, Field, Textarea } from "@/components/ui";
import { fmtWeight, fmtLbs, ozToG, fmtDateRange } from "@/lib/util";
import { TripGearList } from "./trip-gear-list";
import { ContourBand } from "@/components/contour-band";
import { Sheet } from "@/components/sheet";
import { RoutePanel, tripLayout, TripSectionBar, ElevationSparkline } from "./route-panel";
import { RouteMapThumb } from "./route-map-thumb";
import { RouteActions, type Gpx } from "./route-actions";
import { TripBand } from "./trip-band";
import { OptionsMenu } from "./options-menu";
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

const sections = [["logistics", "Logistics"], ["planning", "Planning"], ["gear", "Gear"], ["food", "Food"], ["weather", "Weather"], ["report", "Report"]] as const;

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
  const [, start] = useTransition();
  const [startDate, setStartDate] = useState(trip.startDate ?? "");
  const dateSaveQueue = useRef(Promise.resolve());
  const saveDate = (value: string | null) => {
    dateSaveQueue.current = dateSaveQueue.current.catch(() => {}).then(() => updateTrip(trip.id, { startDate: value }));
    start(async () => await dateSaveQueue.current);
  };
  const del = () => {
    if (!confirm(`Delete "${trip.name}"? This removes its gear, food, route and campsites and can't be undone.`)) return;
    start(async () => {
      await deleteTrip(trip.id);
      router.push(props.backHref);
    });
  };
  const [picker, setPicker] = useState(false);
  type Tab = "logistics" | "planning" | "gear" | "food" | "weather" | "report";
  const hasRoute = !!props.gpx?.points.length;
  const [phoneView, setPhoneView] = useState<"map" | Tab>(hasRoute ? "map" : "logistics");
  const [isPhone, setIsPhone] = useState(false);
  const [minimized, setMinimized] = useState(false);
  useEffect(() => {
    const media = window.matchMedia("(max-width: 639px)");
    const sync = () => setIsPhone(media.matches);
    sync(); media.addEventListener("change", sync);
    setMinimized(localStorage.getItem("strider-route-minimized") === "true");
    return () => media.removeEventListener("change", sync);
  }, []);
  const toggleRoute = (value: boolean) => { setMinimized(value); localStorage.setItem("strider-route-minimized", String(value)); };
  const [tab, setTab] = useState<Tab>("logistics");
  const [generateReportRequest, setGenerateReportRequest] = useState(0);
  const tabKey = `bp_tripTab_${trip.id}`;
  useEffect(() => {
    const saved = localStorage.getItem(tabKey);
    if (saved && ["logistics", "planning", "gear", "food", "weather", "report"].includes(saved)) setTab(saved as Tab);
  }, [tabKey]);
  const pickTab = (k: Tab) => {
    setTab(k);
    setPhoneView(k);
    localStorage.setItem(tabKey, k);
  };
  const activeTab = isPhone ? (phoneView === "map" ? (hasRoute ? tab : "logistics") : phoneView) : tab;
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
    <div className={tripLayout.frame}>
      <header className={`${phoneView === "logistics" ? "hidden" : "block"} relative z-[1100] min-h-[72px] shrink-0 bg-header text-headerink sm:hidden`}>
        <div className="pointer-events-none absolute inset-0 overflow-hidden"><ContourBand /></div>
        <div className="relative flex min-h-[72px] items-center gap-2 px-3">
          <Link href={props.backHref} aria-label="Back to Trips" className="grid h-11 w-11 shrink-0 place-items-center"><ArrowLeft size={21} /></Link>
          <div className="min-w-0 flex-1"><div className="truncate font-display text-[21px] font-bold">{trip.name}</div><div className="truncate text-xs text-headermuted">{props.effectiveDistance ?? "—"} mi · {props.effectiveElevation?.toLocaleString() ?? "—"} ft · {trip.nights ?? "—"} nights · {fmtDateRange(trip.startDate, trip.nights) ?? "No dates"}</div></div>
          {props.isOwner && <div className="shrink-0 text-ink"><ShareMenu icon tripId={trip.id} initialToken={trip.shareToken} initialEditToken={trip.editToken} onGenerateReport={generateReport} /></div>}
        </div>
      </header>
      <TripBand name={trip.name} onName={(name) => save({ name })} region={props.region} startDate={trip.startDate} nights={trip.nights} status={trip.status} distance={props.effectiveDistance} gain={props.effectiveElevation} packG={weights.packG} skinOutG={weights.skinOutG} backHref={props.backHref} backLabel={props.backLabel} phoneVisible={phoneView === "logistics"} actions={<>
        <Select aria-label="Trip status" value={trip.status} onChange={(e) => save({ status: e.target.value as Trip["status"] })} className="!border-headerborder !bg-transparent !text-headerink [&>option]:bg-panel [&>option]:text-ink">
          <option value="idea">Idea</option><option value="planned">Planned</option><option value="completed">Completed</option>
        </Select>
        {props.isOwner && <><ShareMenu tripId={trip.id} initialToken={trip.shareToken} initialEditToken={trip.editToken} onGenerateReport={generateReport} /><OptionsMenu label="More trip options" header><button role="menuitem" onClick={del}>Delete trip</button></OptionsMenu></>}
      </>} />
      <div className={tripLayout.split}>
        {hasRoute && props.gpx && <aside aria-label="Route map and elevation" className={tripLayout.map(phoneView)}>
          <div className={`${minimized && !isPhone ? "hidden" : "flex"} relative h-full min-h-0 flex-1 flex-col overflow-hidden bg-panel sm:rounded-[18px] sm:shadow-[0_8px_28px_rgba(23,56,45,0.16)]`}>
            <RoutePanel tripId={trip.id} points={props.gpx.points} minFt={props.gpx.minEleFt} maxFt={props.gpx.maxEleFt} campsites={props.campsites} pushUndo={pushUndo} mapActions={<>
              <Button className="w-24 !bg-white px-0 !text-[#18261f] shadow max-sm:!hidden" onClick={() => toggleRoute(true)}>Hide map</Button>
              <RouteActions tripId={trip.id} tripName={trip.name} gpx={props.gpx} pushUndo={pushUndo} current={{ distanceMi: trip.distanceMi, elevationGainFt: trip.elevationGainFt, lat: trip.lat, lon: trip.lon }} campsites={props.campsites} />
            </>} />
          </div>
          {minimized && !isPhone && <Card className="flex items-center gap-3 p-3"><RouteMapThumb track={props.gpx.track} className="h-14 w-20 shrink-0" /><ElevationSparkline points={props.gpx.points} /><span className="text-sm text-muted">{(props.gpx.minEleFt ?? Math.min(...props.gpx.points.map((p) => p.e))).toLocaleString()} to {(props.gpx.maxEleFt ?? Math.max(...props.gpx.points.map((p) => p.e))).toLocaleString()} ft</span><Button variant="outline" className="ml-auto" onClick={() => toggleRoute(false)}>Show map and elevation</Button></Card>}
        </aside>}
        {!hasRoute && <div className={`${phoneView === "logistics" ? "block" : "hidden"} relative mx-auto w-full max-w-[1136px] px-4 pt-4 sm:-mt-16 sm:block sm:px-6 sm:pt-0`}><RouteActions tripId={trip.id} tripName={trip.name} gpx={props.gpx} pushUndo={pushUndo} current={{ distanceMi: trip.distanceMi, elevationGainFt: trip.elevationGainFt, lat: trip.lat, lon: trip.lon }} campsites={props.campsites} /></div>}
        <div className={tripLayout.content(hasRoute, phoneView)}>
          <div className="hidden flex-wrap gap-1.5 sm:flex">{sections.map(([key, label]) => <Chip key={key} selected={activeTab === key} onClick={() => pickTab(key)}>{label}</Chip>)}</div>
          <div className="relative mt-3 min-w-0">
        {activeTab === "logistics" && (
          <div className="grid items-start gap-3.5 desk:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
            <Card id="overview" className="min-w-0 p-4">
              <h2 className="text-[17px] font-bold">Details</h2>
              <div className="mt-2.5 grid min-w-0 grid-cols-2 gap-x-3 gap-y-2.5">
              <div className="relative min-w-0 max-w-full">
                <label htmlFor="trip-start-date" className="mb-1 block text-[13px] font-semibold text-muted">Start date</label>
                <div className="flex min-w-0 items-center gap-2">
                  <Input
                    id="trip-start-date"
                    type="date"
                    value={startDate}
                    onChange={(e) => {
                      setStartDate(e.target.value);
                      saveDate(e.target.value || null);
                    }}
                    className="block min-w-0 w-full max-w-full px-2 text-base sm:px-3 sm:text-sm"
                  />
                  {startDate && (
                    <button
                      type="button"
                      onClick={() => {
                        setStartDate("");
                        saveDate(null);
                      }}
                      aria-label="Clear start date"
                      className="absolute right-0 top-0 text-[13px] text-muted underline underline-offset-2 hover:text-ink"
                    >
                      Clear
                    </button>
                  )}
                </div>
              </div>
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
              </div>
            </Card>

            <Card id="by-day" className="scroll-mt-24 p-4">
              <div className="flex items-center justify-between">
                <h2 className="text-[17px] font-bold">By day</h2>
                <button className="min-h-9 px-2.5 text-sm font-semibold text-signal" onClick={() => start(async () => await addTripDay(trip.id))}>Add day</button>
              </div>
              <div className="mt-2 overflow-hidden">
                {dayRows.length === 0 && <div className="px-4 py-3 text-sm text-muted">No days yet. Add a day, or drop campsites on the route.</div>}
                {dayRows.map((d) => {
                  const auto = autoMap.get(d.dayNumber);
                  const dist = d.distanceMi ?? "";
                  const gain = d.elevationGainFt ?? "";
                  return (
                    <div key={d.id} className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 border-b py-2 last:border-0">
                      <span className="readout col-start-1 row-start-1 text-sm text-muted">Day {d.dayNumber}</span>
                      <div className="col-start-2 row-start-1 grid grid-cols-2 gap-2">
                        <Input key={`d${d.id}-${dist}`} aria-label={`Day ${d.dayNumber} distance in miles`} type="number" step="0.1" placeholder="mi" defaultValue={dist} onBlur={(e) => start(async () => await upsertTripDay(trip.id, d.dayNumber, { distanceMi: e.target.value || null }))} />
                        <Input key={`e${d.id}-${gain}`} aria-label={`Day ${d.dayNumber} elevation gain in feet`} type="number" placeholder="ft gain" defaultValue={gain} onBlur={(e) => start(async () => await upsertTripDay(trip.id, d.dayNumber, { elevationGainFt: e.target.value === "" ? null : Number(e.target.value) }))} />
                      </div>
                      <span className="readout col-start-2 row-start-2 truncate text-right text-xs text-muted">
                        {auto?.lat != null && auto?.lon != null ? `${auto.lat.toFixed(3)}, ${auto.lon.toFixed(3)}` : ""}
                      </span>
                      <button onClick={() => start(async () => await deleteTripDay(trip.id, d.id))} aria-label={`Delete day ${d.dayNumber}`} className="col-start-3 row-start-1 grid h-11 w-11 place-items-center justify-self-end rounded text-muted hover:bg-accent/10 hover:text-accent" title="Delete day">✕</button>
                    </div>
                  );
                })}
              </div>
            </Card>
          </div>
        )}

        {activeTab === "planning" && (
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

        {activeTab === "gear" && (
          <div>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="readout text-sm text-muted">{props.packedCount}/{props.gearCount} packed · Base {fmtLbs(weights.baseG)} · Consumable {fmtLbs(weights.consumableG)} · Worn {fmtLbs(weights.wornG)}</span>
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

        {activeTab === "food" && (
          <div className="space-y-6">
            <div id="water-fuel" className="scroll-mt-24">
              <Card className="grid grid-cols-1 gap-4 p-4 sm:grid-cols-2">
                <h2 className="text-[17px] font-bold sm:col-span-2">Water &amp; fuel</h2>
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

        {activeTab === "weather" && (
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
        )}

        {activeTab === "report" && (
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

        </div>
      </div>
      <TripSectionBar sections={sections} hasRoute={hasRoute} value={phoneView} onChange={(key) => key === "map" ? setPhoneView("map") : pickTab(key)} />
      {toast && (
        <div className="fixed bottom-24 sm:bottom-4 left-1/2 z-[2100] flex -translate-x-1/2 items-center gap-3 rounded-full border bg-panel px-4 py-2 text-sm shadow-lg">
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
        <h2 className="text-[17px] font-bold">Fuel calculator</h2>
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
    <Sheet open onClose={onClose} title="Add gear">
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
    </Sheet>
  );
}

function LineBox({ label, defaultValue, onSave }: { label: string; defaultValue: string | null; onSave: (v: string | null) => void }) {
  return (
    <Card className="p-4">
      <h2 className="text-[17px] font-bold">{label}</h2>
      <Textarea
        defaultValue={defaultValue ?? ""}
        onBlur={(e) => onSave(e.target.value.trim() || null)}
        rows={3}
        aria-label={label}
        className="mt-2 w-full max-w-2xl resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25"
      />
    </Card>
  );
}

function NoteCard({ label, defaultValue, onSave, rows = 6 }: { label: string; defaultValue: string | null; onSave: (v: string | null) => void; rows?: number }) {
  return (
    <Card className="p-4">
      <h2 className="text-[17px] font-bold">{label}</h2>
      <Textarea
        defaultValue={defaultValue ?? ""}
        onBlur={(e) => onSave(e.target.value || null)}
        rows={rows}
        aria-label={label}
        className="mt-2 w-full max-w-2xl resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25"
      />
    </Card>
  );
}
