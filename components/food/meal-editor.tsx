"use client";

import { useEffect, useRef, useState, useTransition, type MutableRefObject } from "react";
import { Card, Button, Input, Select, Badge, InfoBadge, Field, IconButton } from "@/components/ui";
import { Sheet } from "@/components/sheet";
import { useUnsavedGuard } from "@/components/unsaved-changes";
import { X } from "lucide-react";
import { gToOz, ozToG } from "@/lib/util";
import { kcalForGrams, kcalPerOz, gramsFromVolume, volumeFromGrams, VOLUME_ML, type AmountUnit } from "@/lib/food";
import { DEFAULT_COOK_WATER_ML } from "@/lib/fuel";
import type { MealRow, IngredientRow } from "@/lib/pantry";
import type { FoodHit } from "@/lib/food-search";
import {
  updateMeal,
  deleteMeal,
  addMealIngredient,
  addMealIngredientFromHit,
  updateMealIngredientAmount,
  removeMealIngredient,
  searchNutrition,
} from "@/app/food/actions";

const MEAL_TYPES = ["breakfast", "lunch", "dinner", "snack"];
// Number field without the native up/down spinner.
const NUM = "[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none";

export function MealEditor({ meal, ingredients, onBack, exitRef, isNew = false }: { meal: MealRow; ingredients: IngredientRow[]; onBack: () => void; exitRef: MutableRefObject<((next: () => void) => void) | null>; isNew?: boolean }) {
  const [, start] = useTransition();
  const run = (fn: () => Promise<unknown>) => start(() => void fn());

  const [form, setForm] = useState({ name: meal.name, mealType: meal.mealType ?? "", baseServings: meal.baseServings, isHot: meal.isHot, waterMl: meal.waterMl });
  const [amounts, setAmounts] = useState<Record<number, number>>(() => Object.fromEntries(meal.items.map((i) => [i.id, i.amountG])));
  const [units, setUnits] = useState<Record<number, AmountUnit>>(() => Object.fromEntries(meal.items.map((i) => [i.id, i.amountUnit])));
  const [preview, setPreview] = useState(meal.baseServings);
  const [asWeight, setAsWeight] = useState(false);
  const [leaving, setLeaving] = useState(false);

  // Add/remove of ingredients happens server-side; merge in new rows while
  // keeping any in-progress local amount and unit edits for surviving rows.
  const itemSig = meal.items.map((i) => i.id).join(",");
  useEffect(() => {
    setAmounts((prev) => Object.fromEntries(meal.items.map((i) => [i.id, prev[i.id] ?? i.amountG])));
    setUnits((prev) => Object.fromEntries(meal.items.map((i) => [i.id, prev[i.id] ?? i.amountUnit])));
  }, [itemSig]);

  const base = form.baseServings || 1;
  const scale = (preview || base) / base;
  const items = meal.items.map((i) => ({ ...i, amountG: amounts[i.id] ?? i.amountG }));
  const totalG = items.reduce((s, i) => s + i.amountG, 0);
  const totalKcal = items.reduce((s, i) => s + kcalForGrams(i.kcalPer100g, i.amountG), 0);
  const perServingKcal = Math.round(totalKcal / base);
  const perServingG = Math.round(totalG / base);
  const calOz = kcalPerOz(totalG ? (totalKcal / totalG) * 100 : null);

  const formDirty =
    form.name !== meal.name ||
    (form.mealType || null) !== (meal.mealType || null) ||
    form.baseServings !== meal.baseServings ||
    form.isHot !== meal.isHot ||
    (form.waterMl ?? null) !== (meal.waterMl ?? null);
  const lineDirty = (i: MealRow["items"][number]) =>
    (amounts[i.id] ?? i.amountG) !== i.amountG || (units[i.id] ?? i.amountUnit) !== i.amountUnit;
  const dirty = formDirty || meal.items.some(lineDirty);

  const persist = async () => {
    await updateMeal(meal.id, { name: form.name, baseServings: form.baseServings, mealType: form.mealType || null, isHot: form.isHot, waterMl: form.isHot ? form.waterMl : null });
    for (const i of meal.items) {
      if (lineDirty(i)) await updateMealIngredientAmount(i.id, amounts[i.id] ?? i.amountG, units[i.id] ?? i.amountUnit);
    }
  };
  const save = () => run(persist);

  const discard = () => {
    setForm({ name: meal.name, mealType: meal.mealType ?? "", baseServings: meal.baseServings, isHot: meal.isHot, waterMl: meal.waterMl });
    setAmounts(Object.fromEntries(meal.items.map((i) => [i.id, i.amountG])));
    setUnits(Object.fromEntries(meal.items.map((i) => [i.id, i.amountUnit])));
  };

  // A just-created meal with no ingredients is still provisional; if the user
  // leaves without saving any edit, remove it rather than keep an empty stub.
  const isBlankNew = isNew && meal.items.length === 0;
  const leaveAction = useRef(onBack);
  const abandon = (next = onBack) => run(async () => { await deleteMeal(meal.id); next(); });

  const tryBack = (next = onBack) => {
    leaveAction.current = next;
    if (dirty) return setLeaving(true);
    if (isBlankNew) return abandon(next);
    next();
  };

  useEffect(() => {
    exitRef.current = tryBack;
    return () => { exitRef.current = null; };
  });

  // Leaving for another page (app navigation or closing the tab) would lose edits.
  useUnsavedGuard(`meal-${meal.id}`, dirty, persist);

  // Clicking empty space closes a clean editor. With unsaved edits nothing is
  // lost by staying, so the click is ignored rather than prompting.
  const cardRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (leaving || dirty) return;
    const h = (e: MouseEvent) => {
      const target = e.target as Element;
      if (target.closest("[data-food-navigation], a, button")) return;
      if (cardRef.current && !cardRef.current.contains(target)) tryBack();
    };
    document.addEventListener("mousedown", h);
    return () => document.removeEventListener("mousedown", h);
  });

  return (
    <div className="min-w-0">
      <div ref={cardRef}>
      <button onClick={() => tryBack()} className="mb-2 hidden min-h-11 items-center text-sm font-semibold text-accent sm:inline-flex desk:hidden">← Back to meals</button>

      <Card className="p-4 sm:p-5">
        <div className="mb-4 hidden items-start justify-between gap-2 desk:flex">
          <h2 className="min-w-0 font-display text-2xl font-bold break-words">{form.name}</h2>
          <IconButton aria-label="Close meal editor" onClick={() => tryBack()}><X size={20} /></IconButton>
        </div>
        <div className="grid grid-cols-2 items-end gap-3">
          <Field className="min-w-0 col-span-2" label="Meal name">
            <Input value={form.name} onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))} />
          </Field>
          <Field label="Type">
            <Select value={form.mealType} onChange={(e) => setForm((f) => ({ ...f, mealType: e.target.value }))}>
              <option value="">—</option>
              {MEAL_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
            </Select>
          </Field>
          <Field className="min-w-0 w-28" label="Recipe yields">
            <Input type="number" min="1" step="1" value={form.baseServings}
              onChange={(e) => setForm((f) => ({ ...f, baseServings: Math.max(1, Number(e.target.value) || 1) }))} />
          </Field>
          <label className="flex min-h-11 items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={form.isHot}
              onChange={(e) => setForm((f) => ({
                ...f,
                isHot: e.target.checked,
                waterMl: e.target.checked ? (f.waterMl ?? DEFAULT_COOK_WATER_ML) : f.waterMl,
              }))}
              className="h-4 w-4 accent-[var(--accent)]"
            />
            Hot meal
            <InfoBadge>Hot meals need a stove, so they drive the trip fuel calculation. Cold-soak, no-cook, and snacks stay off.</InfoBadge>
          </label>
          {form.isHot && (
            <Field className="min-w-0 w-28" label="Cook water (mL)">
              <Input className={NUM} type="number" min="0" step="10" placeholder={String(DEFAULT_COOK_WATER_ML)} value={form.waterMl ?? ""}
                onChange={(e) => setForm((f) => ({ ...f, waterMl: e.target.value === "" ? null : Number(e.target.value) }))} />
            </Field>
          )}
        </div>

        <div className="mt-5 grid grid-cols-2 gap-x-4 gap-y-2 rounded-lg border bg-panel2/40 px-4 py-3 sm:grid-cols-3">
          <Stat label="Recipe">{totalKcal.toLocaleString()} kcal · {gToOz(totalG).toFixed(1)} oz</Stat>
          <Stat label="Per serving">{perServingKcal.toLocaleString()} kcal · {gToOz(perServingG).toFixed(1)} oz</Stat>
          <Stat label="Efficiency">{calOz != null ? `${calOz.toFixed(0)} cal/oz` : "—"}</Stat>
        </div>

        <div className="mt-3 flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
          <div className="flex items-center gap-2 whitespace-nowrap">
            <label htmlFor={`meal-preview-${meal.id}`} className="text-[13px] font-semibold text-muted">Show amounts for</label>
            <Input id={`meal-preview-${meal.id}`} className="!w-16 shrink-0" type="number" min="1" step="1" value={preview}
              onChange={(e) => setPreview(Math.max(1, Number(e.target.value) || 1))} />
            <span className="text-sm text-muted">servings</span>
          </div>
          <label className="flex min-h-11 cursor-pointer items-center gap-2.5 whitespace-nowrap text-[13px] font-semibold text-muted">
            View as weight
            <button
              type="button"
              role="switch"
              aria-checked={asWeight}
              onClick={() => setAsWeight((v) => !v)}
              className={`relative inline-flex h-6 w-10 shrink-0 items-center rounded-full transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent ${asWeight ? "bg-accent" : "bg-fieldborder"}`}
            >
              <span className={`absolute left-0.5 h-5 w-5 rounded-full bg-panel shadow-sm transition-transform ${asWeight ? "translate-x-4" : ""}`} />
            </button>
          </label>
        </div>

        <div className="mt-4 divide-y rounded-lg border">
          {items.length === 0 && <div className="px-4 py-3 text-sm text-muted">No ingredients yet.</div>}
          {items.map((it) => (
            <div key={it.id} className="flex flex-wrap items-center gap-x-2 gap-y-1 py-1.5 pr-1 pl-3 sm:flex-nowrap sm:pl-4">
              <span className="min-w-0 flex-1 text-sm font-medium leading-snug break-words max-sm:basis-full">{it.name}</span>
              <AmountControl
                grams={Math.round(it.amountG * scale)}
                density={it.densityGMl}
                unit={units[it.id] ?? it.amountUnit}
                asWeight={asWeight}
                onUnitChange={(u) => setUnits((m) => ({ ...m, [it.id]: u }))}
                onChange={(displayG) => setAmounts((a) => ({ ...a, [it.id]: Math.max(0, Math.round(displayG / scale)) }))}
              />
              <span className="readout w-16 shrink-0 text-right text-xs text-muted max-sm:ml-auto">{kcalForGrams(it.kcalPer100g, Math.round(it.amountG * scale))} kcal</span>
              <IconButton aria-label={`Remove ${it.name}`} onClick={() => run(() => removeMealIngredient(it.id))} className="text-signal"><X size={16} /></IconButton>
            </div>
          ))}
        </div>

        <AddIngredient
          ingredients={ingredients}
          onAddCustom={(id, g) => run(() => addMealIngredient(meal.id, id, g))}
          onAddHit={(hit) => run(() => addMealIngredientFromHit(meal.id, hit))}
        />

        <div className="mt-5 flex flex-wrap items-center justify-between gap-3">
          <Button variant="danger" onClick={() => confirm(`Delete meal "${meal.name}"?`) && run(async () => { await deleteMeal(meal.id); onBack(); })}>
            Delete
          </Button>
          <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
            <Button variant="outline" disabled={!dirty} onClick={discard}>Discard</Button>
            <Button disabled={!dirty} onClick={save}>Save</Button>
          </div>
        </div>
      </Card>
      </div>

      <Sheet open={leaving} onClose={() => setLeaving(false)} title="Unsaved changes">
        <p className="mt-3 text-sm text-muted">Save your changes to “{meal.name}” before leaving?</p>
        <div className="mt-4 flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={() => setLeaving(false)}>Cancel</Button>
          <Button variant="outline" onClick={() => { setLeaving(false); if (isBlankNew) return abandon(leaveAction.current); discard(); leaveAction.current(); }}>Discard</Button>
          <Button onClick={() => run(async () => { await persist(); setLeaving(false); leaveAction.current(); })}>Save</Button>
        </div>
      </Sheet>
    </div>
  );
}

// Controlled amount editor: a spinner-free number box plus a unit picker.
// `unit` is the line's saved entry unit; `asWeight` shows grams without
// changing it. Volume units are disabled until a density is known. Emits grams.
function AmountControl({ grams, density, unit: savedUnit, asWeight, onUnitChange, onChange }: {
  grams: number;
  density: number | null;
  unit: AmountUnit;
  asWeight: boolean;
  onUnitChange: (unit: AmountUnit) => void;
  onChange: (grams: number) => void;
}) {
  const isVolume = savedUnit !== "g" && savedUnit !== "oz";
  const unit: AmountUnit = asWeight || (isVolume && density == null) ? "g" : savedUnit;
  const toDisplay = (g: number): number => {
    if (unit === "g") return g;
    if (unit === "oz") return +gToOz(g).toFixed(2);
    return +(volumeFromGrams(g, unit, density) ?? 0).toFixed(2);
  };
  const toGrams = (v: number): number => {
    if (unit === "g") return Math.round(v);
    if (unit === "oz") return ozToG(v);
    return gramsFromVolume(v, unit, density) ?? Math.round(grams);
  };
  const [val, setVal] = useState<string>(String(toDisplay(grams)));
  const emitted = useRef(grams);

  // Switching units always reformats the current amount into the new unit.
  const [lastUnit, setLastUnit] = useState(unit);
  if (unit !== lastUnit) {
    setLastUnit(unit);
    emitted.current = grams;
    setVal(String(toDisplay(grams)));
  }
  // Resync from grams only on an EXTERNAL change (e.g. serving-scale preview).
  // The echo of our own edit round-trips through integer grams and would snap
  // a typed "4" oz back to "3.99"; ignore it (tolerance covers scale rounding).
  const [lastGrams, setLastGrams] = useState(grams);
  if (grams !== lastGrams) {
    setLastGrams(grams);
    if (Math.abs(grams - emitted.current) > 1) setVal(String(toDisplay(grams)));
  }

  const onInput = (s: string) => {
    setVal(s);
    const n = Number(s);
    if (s.trim() !== "" && Number.isFinite(n) && n >= 0) {
      const g = toGrams(n);
      emitted.current = g;
      onChange(g);
    }
  };

  return (
    <span className="inline-flex items-center gap-1">
      <Input aria-label="Ingredient amount" className={`!w-20 text-right ${NUM}`} type="number" min="0" step="0.1" value={val} onChange={(e) => onInput(e.target.value)} />
      {asWeight ? (
        <span className="inline-flex min-h-11 w-[4.25rem] items-center px-2 text-sm text-muted">g</span>
      ) : (
        <Select aria-label="Amount unit" value={unit} onChange={(e) => onUnitChange(e.target.value as AmountUnit)} className="!w-[4.25rem] !px-2 !py-1.5">
          <option value="g">g</option>
          <option value="oz">oz</option>
          {Object.keys(VOLUME_ML).map((u) => (
            <option key={u} value={u} disabled={density == null}>{u}</option>
          ))}
        </Select>
      )}
    </span>
  );
}

// Searches your pantry (listed first) and the public APIs together.
function AddIngredient({
  ingredients,
  onAddCustom,
  onAddHit,
}: {
  ingredients: IngredientRow[];
  onAddCustom: (id: number, grams: number) => void;
  onAddHit: (hit: FoodHit) => void;
}) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<FoodHit[]>([]);
  const [searching, setSearching] = useState(false);

  const custom = q.trim().length ? ingredients.filter((i) => i.name.toLowerCase().includes(q.toLowerCase())).slice(0, 6) : [];

  useEffect(() => {
    if (q.trim().length < 2) {
      setHits([]);
      return;
    }
    const h = setTimeout(async () => {
      setSearching(true);
      try {
        setHits(await searchNutrition(q));
      } finally {
        setSearching(false);
      }
    }, 350);
    return () => clearTimeout(h);
  }, [q]);

  return (
    <div className="mt-4">
      <Field label="Add ingredient"><Input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder="e.g. rolled oats, peanut butter, Clif bar" /></Field>
      {(custom.length > 0 || searching || hits.length > 0) && (
        <div className="mt-2 max-h-64 overflow-y-auto rounded-lg border">
          {custom.map((i) => (
            <button key={`c${i.id}`} onClick={() => { onAddCustom(i.id, i.defaultServingG ?? 50); setQ(""); }} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
              <span className="truncate text-sm">{i.name}</span>
              <span className="readout flex max-w-[48%] shrink-0 flex-wrap justify-end gap-2 text-xs text-muted">
                {i.kcalPer100g != null ? Math.round(i.kcalPer100g) : "—"} kcal/100g
                <Badge tone="neutral">saved</Badge>
              </span>
            </button>
          ))}
          {searching && <div className="px-3 py-2 text-sm text-muted">Searching…</div>}
          {hits.map((h) => (
            <button key={`${h.source}-${h.sourceId}`} onClick={() => { onAddHit(h); setQ(""); }} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
              <span className="min-w-0">
                <span className="block truncate text-sm">{h.name}</span>
                {h.detail && <span className="block truncate text-xs text-muted">{h.detail}</span>}
              </span>
              <span className="readout flex max-w-[48%] shrink-0 flex-wrap justify-end gap-2 text-xs text-muted">
                {h.kcalPer100g != null && <span>{h.kcalPer100g} kcal/100g</span>}
                <Badge tone={h.source === "usda" ? "base" : "wishlist"}>{h.source}</Badge>
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function Stat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="text-[13px] font-semibold text-muted">{label}</div>
      <div className="readout text-sm">{children}</div>
    </div>
  );
}
