"use client";

import { useEffect, useRef, useState, useTransition } from "react";
import { Card, Button, Input, Select, Badge, InfoBadge } from "@/components/ui";
import { gToOz, ozToG } from "@/lib/util";
import { kcalForGrams, kcalPerOz, gramsFromVolume, volumeFromGrams, VOLUME_ML, type VolumeUnit } from "@/lib/food";
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

export function MealEditor({ meal, ingredients, onBack, isNew = false }: { meal: MealRow; ingredients: IngredientRow[]; onBack: () => void; isNew?: boolean }) {
  const [, start] = useTransition();
  const run = (fn: () => Promise<unknown>) => start(() => void fn());

  const [form, setForm] = useState({ name: meal.name, mealType: meal.mealType ?? "", baseServings: meal.baseServings, isHot: meal.isHot, waterMl: meal.waterMl });
  const [amounts, setAmounts] = useState<Record<number, number>>(() => Object.fromEntries(meal.items.map((i) => [i.id, i.amountG])));
  const [preview, setPreview] = useState(meal.baseServings);
  const [leaving, setLeaving] = useState(false);

  // Add/remove of ingredients happens server-side; merge in new rows while
  // keeping any in-progress local amount edits for surviving rows.
  const itemSig = meal.items.map((i) => i.id).join(",");
  useEffect(() => {
    setAmounts((prev) => Object.fromEntries(meal.items.map((i) => [i.id, prev[i.id] ?? i.amountG])));
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
  const amountsDirty = meal.items.some((i) => (amounts[i.id] ?? i.amountG) !== i.amountG);
  const dirty = formDirty || amountsDirty;

  const save = () =>
    run(async () => {
      await updateMeal(meal.id, { name: form.name, baseServings: form.baseServings, mealType: form.mealType || null, isHot: form.isHot, waterMl: form.isHot ? form.waterMl : null });
      for (const i of meal.items) {
        const a = amounts[i.id];
        if (a != null && a !== i.amountG) await updateMealIngredientAmount(i.id, a);
      }
    });

  const discard = () => {
    setForm({ name: meal.name, mealType: meal.mealType ?? "", baseServings: meal.baseServings, isHot: meal.isHot, waterMl: meal.waterMl });
    setAmounts(Object.fromEntries(meal.items.map((i) => [i.id, i.amountG])));
  };

  // A just-created meal with no ingredients is still provisional; if the user
  // leaves without saving any edit, remove it rather than keep an empty stub.
  const isBlankNew = isNew && meal.items.length === 0;
  const abandon = () => run(async () => { await deleteMeal(meal.id); onBack(); });

  const tryBack = () => {
    if (dirty) return setLeaving(true);
    if (isBlankNew) return abandon();
    onBack();
  };

  // Clicking outside the card returns to the list (with the same dirty guard).
  const cardRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (leaving) return;
    const h = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) tryBack();
    };
    document.addEventListener("mousedown", h);
    return () => document.removeEventListener("mousedown", h);
  });

  return (
    <div>
      <div ref={cardRef}>
      <button onClick={tryBack} className="eyebrow -my-2 py-2 hover:text-accent">← all meals</button>

      <Card className="mt-2 p-4 sm:p-5">
        <div className="flex flex-wrap items-end gap-3">
          <label className="flex-1">
            <span className="eyebrow mb-1 block">Meal name</span>
            <Input value={form.name} onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))} />
          </label>
          <label>
            <span className="eyebrow mb-1 block">Type</span>
            <Select value={form.mealType} onChange={(e) => setForm((f) => ({ ...f, mealType: e.target.value }))}>
              <option value="">—</option>
              {MEAL_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
            </Select>
          </label>
          <label className="w-28">
            <span className="eyebrow mb-1 block">Recipe yields</span>
            <Input type="number" min="1" step="1" value={form.baseServings}
              onChange={(e) => setForm((f) => ({ ...f, baseServings: Math.max(1, Number(e.target.value) || 1) }))} />
          </label>
          <label className="flex items-center gap-2 pb-1.5 text-sm">
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
            <label className="w-28">
              <span className="eyebrow mb-1 block">Cook water (mL)</span>
              <Input className={NUM} type="number" min="0" step="10" placeholder={String(DEFAULT_COOK_WATER_ML)} value={form.waterMl ?? ""}
                onChange={(e) => setForm((f) => ({ ...f, waterMl: e.target.value === "" ? null : Number(e.target.value) }))} />
            </label>
          )}
        </div>

        <div className="mt-5 flex flex-wrap items-center gap-x-8 gap-y-2 rounded-lg border bg-panel2/40 px-4 py-3">
          <Stat label="Recipe">{totalKcal.toLocaleString()} kcal · {gToOz(totalG).toFixed(1)} oz</Stat>
          <Stat label="Per serving">{perServingKcal.toLocaleString()} kcal · {gToOz(perServingG).toFixed(1)} oz</Stat>
          <Stat label="Efficiency">{calOz != null ? `${calOz.toFixed(0)} cal/oz` : "—"}</Stat>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <span className="eyebrow">Show amounts for</span>
          <Input className="w-16" type="number" min="1" step="1" value={preview}
            onChange={(e) => setPreview(Math.max(1, Number(e.target.value) || 1))} />
          <span className="text-sm text-muted">servings</span>
        </div>

        <div className="mt-4 divide-y rounded-lg border">
          {items.length === 0 && <div className="px-4 py-3 text-sm text-muted">No ingredients yet.</div>}
          {items.map((it) => (
            <div key={it.id} className="flex flex-wrap items-center gap-x-3 gap-y-2 px-3 py-3 sm:flex-nowrap sm:px-4 sm:py-2.5">
              <span className="min-w-0 basis-full text-sm font-medium leading-snug break-words sm:basis-auto sm:flex-1 sm:truncate">{it.name}</span>
              <span className="readout w-20 text-right text-xs text-muted">{kcalForGrams(it.kcalPer100g, Math.round(it.amountG * scale))} kcal</span>
              <AmountControl
                grams={Math.round(it.amountG * scale)}
                density={it.densityGMl}
                onChange={(displayG) => setAmounts((a) => ({ ...a, [it.id]: Math.max(0, Math.round(displayG / scale)) }))}
              />
              <button onClick={() => run(() => removeMealIngredient(it.id))} className="ml-auto grid h-11 w-11 place-items-center rounded text-muted hover:bg-accent/10 hover:text-accent sm:ml-0 sm:h-auto sm:w-auto" title="Remove">✕</button>
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
            Delete meal
          </Button>
          <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
            <Button variant="outline" disabled={!dirty} onClick={discard}>Discard changes</Button>
            <Button disabled={!dirty} onClick={save}>Save</Button>
          </div>
        </div>
      </Card>
      </div>

      {leaving && (
        <div className="fixed inset-0 z-[1100] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm" onClick={() => setLeaving(false)}>
          <Card className="w-full max-w-sm p-5" onClick={(e) => e.stopPropagation()}>
            <div className="font-display text-lg font-bold">Unsaved changes</div>
            <p className="mt-1 text-sm text-muted">Save your changes to “{meal.name}” before leaving?</p>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setLeaving(false)}>Cancel</Button>
              <Button variant="outline" onClick={() => { setLeaving(false); if (isBlankNew) return abandon(); discard(); onBack(); }}>Discard</Button>
              <Button onClick={() => { save(); setLeaving(false); onBack(); }}>Save</Button>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}

// Controlled amount editor: a spinner-free number box plus a unit toggle.
// Volume units are disabled until a density is known. Emits grams on change.
function AmountControl({ grams, density, onChange }: { grams: number; density: number | null; onChange: (grams: number) => void }) {
  const [unit, setUnit] = useState<"g" | "oz" | VolumeUnit>("g");
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
      <Input className={`w-20 text-right ${NUM}`} type="number" min="0" step="0.1" value={val} onChange={(e) => onInput(e.target.value)} />
      <Select value={unit} onChange={(e) => setUnit(e.target.value as typeof unit)} className="!px-2 !py-1.5">
        <option value="g">g</option>
        <option value="oz">oz</option>
        {Object.keys(VOLUME_ML).map((u) => (
          <option key={u} value={u} disabled={density == null}>{u}</option>
        ))}
      </Select>
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
      <span className="eyebrow mb-1 block">Add ingredient</span>
      <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="e.g. rolled oats, peanut butter, Clif bar" />
      {(custom.length > 0 || searching || hits.length > 0) && (
        <div className="mt-2 max-h-64 overflow-y-auto rounded-lg border">
          {custom.map((i) => (
            <button key={`c${i.id}`} onClick={() => { onAddCustom(i.id, i.defaultServingG ?? 50); setQ(""); }} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
              <span className="truncate text-sm">{i.name}</span>
              <span className="readout flex shrink-0 items-center gap-2 text-xs text-muted">
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
              <span className="readout flex shrink-0 items-center gap-2 text-xs text-muted">
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
      <div className="eyebrow">{label}</div>
      <div className="readout text-sm">{children}</div>
    </div>
  );
}
