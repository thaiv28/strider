"use client";

import { useEffect, useState, useTransition } from "react";
import Link from "next/link";
import { Copy } from "lucide-react";
import { Card, Input, Badge, Button, InfoBadge } from "@/components/ui";
import { gToOz } from "@/lib/util";
import { searchNutrition } from "@/app/food/actions";
import { addTripMeal, setTripMealQty, removeTripMeal, copyTripMealToAll } from "@/app/trips/actions";
import type { FoodHit } from "@/lib/food-search";
import type { TripFood, ShoppingItem } from "@/lib/trip";

export type MealOpt = { id: number; name: string; mealType: string | null; baseServings: number; totalKcal: number; totalG: number };
export type IngOpt = { id: number; name: string; kcalPer100g: number | null; defaultServingG: number | null };

export function TripFoodPanel({
  tripId,
  food,
  meals,
  ingredients,
}: {
  tripId: number;
  food: TripFood;
  meals: MealOpt[];
  ingredients: IngOpt[];
}) {
  const [, start] = useTransition();
  const run = (fn: () => Promise<unknown>) => start(() => void fn());

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-x-8 gap-y-2 rounded-lg border bg-panel2/40 px-4 py-3">
        <Stat label="Trip food">{food.totalKcal.toLocaleString()} kcal · {gToOz(food.totalG).toFixed(1)} oz</Stat>
        <Stat label="Avg / day">{food.avgKcal.toLocaleString()} kcal</Stat>
        <span className="ml-auto flex items-center gap-1.5 text-xs text-muted">
          <InfoBadge>
            Each day's calorie target is BMR ({food.params.bmr.toLocaleString()}) plus moving
            calories from that day's distance and elevation gain.
          </InfoBadge>
          <Link href="/settings" className="text-accent hover:underline">adjust model</Link>
        </span>
      </div>

      {food.days.map((d) => {
        return (
          <Card key={d.dayNumber} className="p-4">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
              <span className="font-display font-semibold">Day {d.dayNumber}</span>
              <span className="readout text-xs text-muted">
                {d.distanceMi != null ? `${d.distanceMi} mi` : "—"} · {d.gainFt != null ? `${d.gainFt.toLocaleString()} ft` : "—"}
              </span>
              <span className="readout ml-auto flex items-center gap-1 text-sm">
                {d.plannedKcal.toLocaleString()} <span className="text-muted">/ {d.targetKcal.toLocaleString()} kcal</span>
                <InfoBadge>
                  Target = {food.params.bmr.toLocaleString()} BMR + {(d.targetKcal - food.params.bmr).toLocaleString()} moving calories.
                </InfoBadge>
              </span>
              <span className="readout text-xs text-muted">{gToOz(d.plannedG).toFixed(1)} oz</span>
            </div>

            <div className="mt-3 divide-y rounded-lg border">
              {d.entries.length === 0 && <div className="px-3 py-2 text-sm text-muted">Nothing planned.</div>}
              {d.entries.map((e) => (
                <div key={e.id} className="flex flex-wrap items-center gap-x-3 gap-y-2 px-3 py-2.5 sm:flex-nowrap sm:py-2">
                  {e.isMeal && e.mealId ? (
                    <Link
                      href={`/food?meal=${e.mealId}&back=/trips/${tripId}`}
                      className="min-w-0 basis-[calc(100%-3rem)] text-sm leading-snug break-words transition hover:text-accent sm:basis-auto sm:truncate"
                      title="Edit this meal"
                    >
                      {e.name}
                    </Link>
                  ) : (
                    <span className="min-w-0 basis-[calc(100%-3rem)] text-sm leading-snug break-words sm:basis-auto sm:flex-1 sm:truncate">{e.name}</span>
                  )}
                  <QtyInput isMeal={e.isMeal} value={e.isMeal ? e.servings : e.weightG} onCommit={(q) => run(() => setTripMealQty(e.id, tripId, q))} />
                  <span className="readout w-16 text-right text-xs text-muted">{e.kcal.toLocaleString()} kcal</span>
                  <span className="readout w-14 text-right text-xs text-muted">{gToOz(e.weightG).toFixed(1)} oz</span>
                  <button onClick={() => run(() => copyTripMealToAll(e.id, tripId))} className="grid h-11 w-11 place-items-center rounded text-muted hover:bg-panel2 hover:text-accent sm:h-auto sm:w-auto" title="Copy to all days">
                    <Copy size={14} />
                  </button>
                  <button onClick={() => run(() => removeTripMeal(e.id, tripId))} className="grid h-11 w-11 place-items-center rounded text-muted hover:bg-accent/10 hover:text-accent sm:h-auto sm:w-auto" title="Remove">✕</button>
                </div>
              ))}
            </div>

            <AddFood
              meals={meals}
              ingredients={ingredients}
              onAdd={(entry) => run(() => addTripMeal(tripId, d.dayNumber, entry))}
            />
          </Card>
        );
      })}
    </div>
  );
}

// Consolidated grocery list — total grams to buy per ingredient, scaled by the
// people count (defaults to party size, overridable here only). Items removed
// here drop to a "Removed" section and are left out of the copy + report.
export function ShoppingList({
  items,
  partySize,
  people,
  removed,
  tripName,
  onPeople,
  onRemoved,
}: {
  items: ShoppingItem[];
  partySize: number;
  people: number;
  removed: string[];
  tripName: string;
  onPeople: (n: number | null) => void;
  onRemoved: (keys: string[]) => void;
}) {
  const [copied, setCopied] = useState(false);

  const [pplStr, setPplStr] = useState(String(people));
  const [lastPeople, setLastPeople] = useState(people);
  if (people !== lastPeople) { setLastPeople(people); setPplStr(String(people)); }
  const ppl = Math.max(1, Math.round(Number(pplStr) || 1));

  // Optimistic removal set so the X / restore feel instant; the saved value
  // (via revalidation) resyncs it.
  const [rem, setRem] = useState<Set<string>>(() => new Set(removed));
  const remSig = removed.join(",");
  const [lastSig, setLastSig] = useState(remSig);
  if (remSig !== lastSig) { setLastSig(remSig); setRem(new Set(removed)); }

  const scale = (g: number) => Math.round(g * ppl);
  const active = items.filter((i) => !rem.has(i.key));
  const gone = items.filter((i) => rem.has(i.key));

  const commitPeople = () => onPeople(ppl === partySize ? null : ppl);
  const remove = (key: string) => { const s = new Set(rem); s.add(key); setRem(s); onRemoved([...s]); };
  const restore = (key: string) => { const s = new Set(rem); s.delete(key); setRem(s); onRemoved([...s]); };

  const asText = () =>
    [`Grocery list — ${tripName}`, "", ...active.map((it) => `- [ ] ${it.name} — ${scale(it.grams)} g (${gToOz(scale(it.grams)).toFixed(1)} oz)`)].join("\n");

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(asText());
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      /* clipboard unavailable */
    }
  };

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-1.5">
          <span className="eyebrow">Grocery list</span>
          <InfoBadge>
            Every planned meal's ingredients (scaled to the servings you planned) plus any
            standalone foods, summed per ingredient and multiplied by the people count. Copy pastes
            a checkable list you can tick off in a doc or notes app.
          </InfoBadge>
        </div>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-1.5 text-sm text-muted">
            People
            <Input
              type="number"
              min="1"
              step="1"
              value={pplStr}
              onChange={(e) => setPplStr(e.target.value)}
              onBlur={commitPeople}
              onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
              className="w-16 text-center [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
            />
          </label>
          {active.length > 0 && <Button variant="outline" onClick={copy}>{copied ? "Copied!" : "Copy list"}</Button>}
        </div>
      </div>
      <Card className="mt-2 p-4">
        {items.length === 0 ? (
          <p className="text-sm text-muted">Plan some meals and the ingredients to buy show up here.</p>
        ) : (
          <>
            <div className="divide-y rounded-lg border">
              {active.length === 0 && <div className="px-3 py-2 text-sm text-muted">Everything's been removed.</div>}
              {active.map((it) => (
                <div key={it.key} className="flex items-center gap-3 px-3 py-2">
                  <span className="min-w-0 flex-1 truncate text-sm">{it.name}</span>
                  <span className="readout w-16 text-right text-xs text-muted">{scale(it.grams)} g</span>
                  <span className="readout w-14 text-right text-xs text-muted">{gToOz(scale(it.grams)).toFixed(1)} oz</span>
                  <button onClick={() => remove(it.key)} className="text-muted hover:text-accent" title="Remove from list">✕</button>
                </div>
              ))}
            </div>
            {gone.length > 0 && (
              <div className="mt-4">
                <div className="eyebrow mb-1">Removed</div>
                <div className="divide-y rounded-lg border">
                  {gone.map((it) => (
                    <div key={it.key} className="flex items-center gap-3 px-3 py-2">
                      <span className="min-w-0 flex-1 truncate text-sm text-muted line-through">{it.name}</span>
                      <button onClick={() => restore(it.key)} className="text-xs text-accent hover:underline" title="Add back to list">add back</button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </Card>
    </div>
  );
}

function QtyInput({ isMeal, value, onCommit }: { isMeal: boolean; value: number; onCommit: (q: number) => void }) {
  const [v, setV] = useState(String(value));
  const [k, setK] = useState(value);
  if (k !== value) { setK(value); setV(String(value)); }
  const commit = () => {
    const n = Number(v);
    if (Number.isFinite(n) && n > 0 && n !== value) onCommit(isMeal ? Math.round(n) : Math.round(n));
  };
  return (
    <span className="inline-flex items-center gap-1">
      <Input
        type="number"
        min={isMeal ? "1" : "0"}
        step={isMeal ? "1" : "5"}
        value={v}
        onChange={(e) => setV(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        className="w-16 text-right [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
      />
      <span className="w-8 text-xs text-muted">{isMeal ? "srv" : "g"}</span>
    </span>
  );
}

type AddEntry =
  | { kind: "meal"; mealId: number; servings: number }
  | { kind: "ingredient"; ingredientId: number; grams: number }
  | { kind: "hit"; hit: FoodHit; grams: number };

function AddFood({
  meals,
  ingredients,
  onAdd,
}: {
  meals: MealOpt[];
  ingredients: IngOpt[];
  onAdd: (entry: AddEntry) => void;
}) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<FoodHit[]>([]);
  const [searching, setSearching] = useState(false);

  const mealMatches = q.trim().length ? meals.filter((m) => m.name.toLowerCase().includes(q.toLowerCase())).slice(0, 5) : [];
  const ingMatches = q.trim().length ? ingredients.filter((i) => i.name.toLowerCase().includes(q.toLowerCase())).slice(0, 5) : [];

  useEffect(() => {
    if (q.trim().length < 2) {
      setHits([]);
      return;
    }
    let cancelled = false;
    const h = setTimeout(async () => {
      setSearching(true);
      try {
        const r = await searchNutrition(q);
        if (!cancelled) setHits(r);
      } finally {
        if (!cancelled) setSearching(false);
      }
    }, 350);
    // Cancel a debounced/in-flight search so its results can't repopulate the
    // dropdown after the query was cleared (e.g. right after picking a result).
    return () => {
      cancelled = true;
      clearTimeout(h);
    };
  }, [q]);

  const pick = (entry: AddEntry) => {
    onAdd(entry);
    setQ("");
    setHits([]);
  };

  return (
    <div className="mt-3">
      <div className="min-w-0 flex-1">
        <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="add a meal or food" />
        {(mealMatches.length > 0 || ingMatches.length > 0 || searching || hits.length > 0) && (
          <div className="mt-2 max-h-64 overflow-y-auto rounded-lg border">
            {mealMatches.map((m) => (
              <button key={`m${m.id}`} onClick={() => pick({ kind: "meal", mealId: m.id, servings: 1 })} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
                <span className="truncate text-sm">{m.name}</span>
                <span className="readout flex shrink-0 items-center gap-2 text-xs text-muted">
                  {Math.round(m.totalKcal / m.baseServings)} kcal/srv
                  <Badge tone="wishlist">meal</Badge>
                </span>
              </button>
            ))}
            {ingMatches.map((i) => (
              <button key={`i${i.id}`} onClick={() => pick({ kind: "ingredient", ingredientId: i.id, grams: i.defaultServingG ?? 50 })} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
                <span className="truncate text-sm">{i.name}</span>
                <span className="readout flex shrink-0 items-center gap-2 text-xs text-muted">
                  {i.kcalPer100g != null ? Math.round(i.kcalPer100g) : "—"} kcal/100g
                  <Badge tone="neutral">saved</Badge>
                </span>
              </button>
            ))}
            {searching && <div className="px-3 py-2 text-sm text-muted">Searching…</div>}
            {hits.map((h) => (
              <button key={`${h.source}-${h.sourceId}`} onClick={() => pick({ kind: "hit", hit: h, grams: h.defaultServingG ?? 50 })} className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60">
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
