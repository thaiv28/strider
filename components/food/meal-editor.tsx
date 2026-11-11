"use client";

import { useEffect, useRef, useState, useTransition, type MutableRefObject } from "react";
import { Card, Button, Input, Select, Badge, InfoBadge, Field, IconButton } from "@/components/ui";
import { Sheet } from "@/components/sheet";
import { useUnsavedGuard } from "@/components/unsaved-changes";
import { X } from "lucide-react";
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
  suggestReferenceFoods,
  linkIngredientDensity,
  setIngredientMeasure,
} from "@/app/food/actions";

import { useFoodSearch, FoodSearchResults, FOOD_SEARCH_PLACEHOLDER, foodSearchKeys } from "./food-search";

const MEAL_TYPES = ["breakfast", "lunch", "dinner", "snack"];
type EnteredAmount = { amount: number; unit: string };
const storedEntered = (i: MealRow["items"][number]): EnteredAmount | null => i.enteredAmount != null && i.enteredUnit != null ? { amount: i.enteredAmount, unit: i.enteredUnit } : null;
// Re-express a gram amount in a line's entered unit, so editing in the weight
// view keeps the original unit. null when a volume has no density to convert with.
const enteredForGrams = (grams: number, unit: string, density: number | null): EnteredAmount | null => {
  if (unit === "g") return { amount: Math.round(grams), unit };
  if (unit === "oz") return { amount: +gToOz(grams).toFixed(2), unit };
  const v = volumeFromGrams(grams, unit as VolumeUnit, density);
  return v == null ? null : { amount: +v.toFixed(2), unit };
};

// Number field without the native up/down spinner.
const NUM = "[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none";

export function MealEditor({ meal, ingredients, onBack, exitRef, isNew = false }: { meal: MealRow; ingredients: IngredientRow[]; onBack: () => void; exitRef: MutableRefObject<((next: () => void) => void) | null>; isNew?: boolean }) {
  const [, start] = useTransition();
  const run = (fn: () => Promise<unknown>) => start(() => void fn());

  const [form, setForm] = useState({ name: meal.name, mealType: meal.mealType ?? "", baseServings: meal.baseServings, isHot: meal.isHot, waterMl: meal.waterMl });
  const [amounts, setAmounts] = useState<Record<number, number>>(() => Object.fromEntries(meal.items.map((i) => [i.id, i.amountG])));
  const [entered, setEntered] = useState<Record<number, EnteredAmount | null>>(() => Object.fromEntries(meal.items.map((i) => [i.id, storedEntered(i)])));
  const [resetKey, setResetKey] = useState(0);
  const [preview, setPreview] = useState(meal.baseServings);
  const [asWeight, setAsWeight] = useState(false);
  const [leaving, setLeaving] = useState(false);

  // Add/remove of ingredients happens server-side; merge in new rows while
  // keeping any in-progress local amount edits for surviving rows.
  const previousIds = useRef(new Set(meal.items.map((i) => i.id)));
  const itemSig = meal.items.map((i) => i.id).join(",");
  useEffect(() => {
    const added = meal.items.find((i) => !previousIds.current.has(i.id));
    previousIds.current = new Set(meal.items.map((i) => i.id));
    if (added) requestAnimationFrame(() => { const input = document.querySelector<HTMLInputElement>(`[data-meal-line="${added.id}"] input[aria-label="Ingredient amount"]`); input?.focus(); input?.select(); });
    setAmounts((prev) => Object.fromEntries(meal.items.map((i) => [i.id, prev[i.id] ?? i.amountG])));
    setEntered((prev) => Object.fromEntries(meal.items.map((i) => [i.id, Object.prototype.hasOwnProperty.call(prev, i.id) ? prev[i.id] : storedEntered(i)])));
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
  const enteredDirty = (i: MealRow["items"][number]) => JSON.stringify(entered[i.id] ?? null) !== JSON.stringify(storedEntered(i));
  const dirty = formDirty || amountsDirty || meal.items.some(enteredDirty);

  const persist = async () => {
    await updateMeal(meal.id, { name: form.name, baseServings: form.baseServings, mealType: form.mealType || null, isHot: form.isHot, waterMl: form.isHot ? form.waterMl : null });
    for (const i of meal.items) {
      const a = amounts[i.id];
      if (a != null && (a !== i.amountG || enteredDirty(i))) await updateMealIngredientAmount(i.id, a, entered[i.id]);
    }
  };
  const save = () => run(persist);

  const discard = () => {
    setForm({ name: meal.name, mealType: meal.mealType ?? "", baseServings: meal.baseServings, isHot: meal.isHot, waterMl: meal.waterMl });
    setAmounts(Object.fromEntries(meal.items.map((i) => [i.id, i.amountG])));
    setEntered(Object.fromEntries(meal.items.map((i) => [i.id, storedEntered(i)])));
    setResetKey((k) => k + 1);
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
              className="h-4 w-4 accent-accent"
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
            <div key={it.id} data-meal-line={it.id} className="flex flex-wrap items-center gap-x-2 gap-y-1 py-1.5 pr-1 pl-3 sm:pl-4">
              <span className="min-w-0 flex-1 text-sm font-medium leading-snug break-words max-sm:basis-full sm:basis-0">{it.name}</span>
              {asWeight ? (
                <WeightAmount
                  key={`${it.id}-${resetKey}-weight`}
                  grams={Math.round(it.amountG * scale)}
                  onChange={(displayG) => {
                    const baseG = displayG / scale;
                    setAmounts((a) => ({ ...a, [it.id]: Math.max(0, Math.round(baseG)) }));
                    setEntered((e) => {
                      const cur = e[it.id];
                      return cur ? { ...e, [it.id]: enteredForGrams(baseG, cur.unit, it.densityGMl) } : e;
                    });
                  }}
                />
              ) : <AmountControl
                key={`${it.id}-${resetKey}`}
                grams={Math.round(it.amountG * scale)}
                density={it.densityGMl}
                ingredientName={it.name}
                onLinkReference={(fdcId) => { if (it.ingredientId != null) run(() => linkIngredientDensity(it.ingredientId!, fdcId)); }}
                canSetDensity={it.ingredientId != null}
                initialUnit={entered[it.id]?.unit ?? "g"}
                displayAmount={entered[it.id] ? +(entered[it.id]!.amount * scale).toFixed(3) : null}
                onSetMeasure={(unit, grams) => { if (it.ingredientId != null) run(() => setIngredientMeasure(it.ingredientId!, unit, grams)); }}
                onChange={(displayG, value) => {
                  setAmounts((a) => ({ ...a, [it.id]: Math.max(0, Math.round(displayG / scale)) }));
                  setEntered((e) => ({ ...e, [it.id]: { amount: value.amount / scale, unit: value.unit } }));
                }}
              />}
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

// Gram-only amount field for the "View as weight" mode. Emits displayed grams.
function WeightAmount({ grams, onChange }: { grams: number; onChange: (grams: number) => void }) {
  const [val, setVal] = useState(String(grams));
  const emitted = useRef(grams);
  // Resync only on an external change (the serving preview), not the echo of typing.
  const [lastGrams, setLastGrams] = useState(grams);
  if (grams !== lastGrams) {
    setLastGrams(grams);
    if (Math.abs(grams - emitted.current) > 1) { emitted.current = grams; setVal(String(grams)); }
  }
  const onInput = (text: string) => {
    setVal(text);
    const g = Number(text);
    if (text.trim() === "" || !Number.isFinite(g) || g < 0) return;
    emitted.current = g;
    onChange(g);
  };
  return (
    <span className="inline-flex items-center gap-1">
      <Input aria-label="Ingredient amount" className={`!w-20 text-right ${NUM}`} type="number" min="0" step="1" value={val} onChange={(e) => onInput(e.target.value)} />
      <span className="inline-flex min-h-11 w-[4.25rem] items-center px-2 text-sm text-muted">g</span>
    </span>
  );
}

// Canonical grams and the entered quantity travel together, at the displayed scale.
function AmountControl({ grams, density, ingredientName, onLinkReference, canSetDensity, initialUnit, displayAmount, onChange, onSetMeasure }: {
  grams: number; density: number | null; ingredientName: string; onLinkReference: (fdcId: string) => void; canSetDensity: boolean;
  initialUnit: string; displayAmount: number | null;
  onChange: (grams: number, entered: EnteredAmount) => void;
  onSetMeasure: (unit: VolumeUnit, grams: number) => void;
}) {
  const [unit, setUnit] = useState(initialUnit);
  const isVolume = unit !== "g" && unit !== "oz";
  const toDisplay = (g: number, u = unit): number => {
    if (u === "g") return g;
    if (u === "oz") return +gToOz(g).toFixed(2);
    return +(volumeFromGrams(g, u as VolumeUnit, density) ?? 0).toFixed(2);
  };
  const [val, setVal] = useState(String(displayAmount ?? toDisplay(grams)));
  const typed = useRef(false);
  const hasNumber = () => val.trim() !== "" && Number.isFinite(Number(val)) && Number(val) >= 0;
  const pendingTyped = useRef(false);
  const [suggestions, setSuggestions] = useState<FoodHit[]>([]);
  useEffect(() => {
    if (!isVolume || density != null || !canSetDensity) return;
    let active = true;
    void suggestReferenceFoods(ingredientName).then((rows) => { if (active) setSuggestions(rows.slice(0, 3)); }).catch(() => {});
    return () => { active = false; };
  }, [isVolume, density, canSetDensity, ingredientName]);
  const [measure, setMeasure] = useState("");
  const emitted = useRef({ grams, amount: displayAmount });
  const previous = useRef({ grams, displayAmount, density });

  useEffect(() => {
    const prev = previous.current;
    previous.current = { grams, displayAmount, density };
    if (prev.density == null && density != null && isVolume) {
      const keep = (pendingTyped.current || typed.current) && hasNumber();
      const amount = keep ? Number(val) : toDisplay(grams);
      const nextGrams = keep ? gramsFromVolume(amount, unit as VolumeUnit, density)! : grams;
      pendingTyped.current = false;
      setVal(String(amount));
      emitted.current = { grams: nextGrams, amount };
      onChange(nextGrams, { amount, unit });
    } else if (grams !== prev.grams || displayAmount !== prev.displayAmount) {
      // Only an external change (the serving preview) reformats the field; the
      // echo of the user's own typing must not snap it to a rounded value.
      const moved = displayAmount == null || emitted.current.amount == null
        ? grams !== emitted.current.grams
        : Math.abs(displayAmount - emitted.current.amount) > 0.002;
      if (moved) {
        setVal(String(displayAmount ?? toDisplay(grams)));
        emitted.current = { grams, amount: displayAmount };
      }
    }
  });

  const changeUnit = (next: string) => {
    const preserve = typed.current && hasNumber();
    const amount = preserve ? Number(val) : toDisplay(grams, next);
    const nextGrams = preserve ? next === "g" ? Math.round(amount) : next === "oz" ? ozToG(amount) : gramsFromVolume(amount, next as VolumeUnit, density) ?? grams : grams;
    typed.current = false;
    pendingTyped.current = density == null && next !== "g" && next !== "oz" && preserve;
    setUnit(next);
    setVal(String(amount));
    if (density == null && next !== "g" && next !== "oz") return;
    emitted.current = { grams: nextGrams, amount };
    onChange(nextGrams, { amount, unit: next });
  };
  const onInput = (text: string) => {
    typed.current = true;
    setVal(text);
    const amount = Number(text);
    if (text.trim() === "" || !Number.isFinite(amount) || amount < 0) return;
    // Without a density these grams would be wrong; hold the number until one arrives.
    if (isVolume && density == null) return;
    const g = unit === "g" ? Math.round(amount) : unit === "oz" ? ozToG(amount) : gramsFromVolume(amount, unit as VolumeUnit, density) ?? grams;
    emitted.current = { grams: g, amount };
    onChange(g, { amount, unit });
  };

  return (
    <span className="contents">
      <span className="inline-flex items-center gap-1">
        <Input aria-label="Ingredient amount" className={`!w-20 text-right ${NUM}`} type="number" min="0" step="0.1" value={val} onChange={(e) => onInput(e.target.value)} />
        <Select aria-label="Ingredient amount unit" value={unit} onChange={(e) => changeUnit(e.target.value)} className="!px-2 !py-1.5">
          <option value="g">g</option><option value="oz">oz</option>
          {(Object.keys(VOLUME_ML) as VolumeUnit[]).map((u) => <option key={u} value={u} disabled={density == null && !canSetDensity}>{u}</option>)}
        </Select>
      </span>
      {isVolume && density == null && canSetDensity && (
        <span className="order-last flex basis-full flex-col gap-2 rounded border bg-panel2/40 p-2 text-xs text-muted">
          {suggestions.length > 0 && <span>Pick the closest USDA food:</span>}
          {suggestions.map((h) => <button type="button" key={h.sourceId} onClick={() => onLinkReference(h.sourceId)} className="min-h-11 rounded border bg-panel px-2 py-1.5 text-left text-ink break-words hover:border-accent/50 hover:text-accent sm:min-h-0">{h.referenceName} · 1 {unit} = {(h.densityGMl! * VOLUME_ML[unit as VolumeUnit]).toFixed(1)} g</button>)}
          <span className="flex flex-wrap items-center gap-1"><span>or 1 {unit} =</span>
          <Input aria-label="Measure weight grams" className="!w-20 min-w-0" type="number" min="0" step="0.1" value={measure} onChange={(e) => setMeasure(e.target.value)} />
          <span>g</span>
          <Button variant="outline" disabled={!Number.isFinite(Number(measure)) || Number(measure) <= 0} onClick={() => onSetMeasure(unit as VolumeUnit, Number(measure))}>Set</Button></span>
        </span>
      )}
    </span>
  );
}

// Saved pantry foods precede bundled reference results.
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
  const search = useFoodSearch(q);
  const custom = q.trim().length ? ingredients.filter((i) => i.name.toLowerCase().includes(q.toLowerCase())).slice(0, 6) : [];

  return (
    <div className="mt-4">
      <Field label="Add ingredient"><Input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={FOOD_SEARCH_PLACEHOLDER} onKeyDown={(e) => foodSearchKeys(e, () => setQ(""))} /></Field>
      <FoodSearchResults query={q} search={search} onPick={(h) => { onAddHit(h); setQ(""); }}>
          {custom.map((i) => (
            <button key={`c${i.id}`} onClick={() => { onAddCustom(i.id, i.defaultServingG ?? 50); setQ(""); }} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
              <span className="truncate text-sm">{i.name}</span>
              <span className="readout flex max-w-[48%] shrink-0 flex-wrap justify-end gap-2 text-xs text-muted">
                {i.kcalPer100g != null ? Math.round(i.kcalPer100g) : "—"} kcal/100g
                <Badge tone="neutral">saved</Badge>
              </span>
            </button>
          ))}
      </FoodSearchResults>
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
