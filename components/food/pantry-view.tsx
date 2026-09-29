"use client";

import { useEffect, useState, useTransition } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Card, Button, Badge, Input, PageTabs } from "@/components/ui";
import { gToOz } from "@/lib/util";
import { kcalPerOz } from "@/lib/food";
import type { IngredientRow, MealRow } from "@/lib/pantry";
import type { FoodHit } from "@/lib/food-search";
import { createMeal, deleteIngredient, searchNutrition, addIngredientFromHit } from "@/app/food/actions";
import { IngredientDialog } from "./ingredient-dialog";
import { MealEditor } from "./meal-editor";

type Tab = "meals" | "ingredients";

export function PantryView({ ingredients, meals }: { ingredients: IngredientRow[]; meals: MealRow[] }) {
  const router = useRouter();
  const params = useSearchParams();
  // Deep-link: /food?meal=<id>&back=<href> opens a meal and returns to `back`
  // (e.g. the trip that linked here) via the back button or a click-outside.
  const back = params.get("back");
  const [tab, setTab] = useState<Tab>("meals");
  const [editing, setEditing] = useState<IngredientRow | null>(null);
  const [adding, setAdding] = useState(false);
  const [mealId, setMealId] = useState<number | null>(() => Number(params.get("meal")) || null);
  // Id of a meal we just created via "+ Meal" — it's provisional until the user
  // actually edits it, so MealEditor discards it on exit if left untouched.
  const [createdId, setCreatedId] = useState<number | null>(null);
  const [, start] = useTransition();

  const openMeal = meals.find((m) => m.id === mealId) ?? null;
  // Prefer history.back() when we arrived from another page (e.g. a trip) so
  // the App Router restores that page's scroll position; router.push would
  // reset it to the top. Fall back to push if there's no history to pop.
  const closeMeal = () => {
    setCreatedId(null);
    if (!back) return setMealId(null);
    if (window.history.length > 1) router.back();
    else router.push(back);
  };

  return (
    <div className="mx-auto max-w-3xl px-4 py-6 sm:px-8 sm:py-10">
      <div className="flex items-end justify-between">
        <div>
          <div className="eyebrow">◇ Pantry</div>
          <h1 className="font-display mt-1 text-3xl font-bold tracking-tight">Food</h1>
        </div>
        {tab === "ingredients" ? (
          <Button onClick={() => setAdding(true)}>+ Ingredient</Button>
        ) : (
          <Button onClick={() => start(async () => { const id = await createMeal("New meal"); setCreatedId(id); setMealId(id); })}>+ Meal</Button>
        )}
      </div>

      {!openMeal && (
        <PageTabs
          label="Food sections"
          tabs={[{ value: "meals", label: "Meals" }, { value: "ingredients", label: "Ingredients" }]}
          value={tab}
          onChange={setTab}
          className="mt-6"
        />
      )}

      <div className="mt-4">
        {openMeal ? (
          <MealEditor meal={openMeal} ingredients={ingredients} onBack={closeMeal} isNew={openMeal.id === createdId} />
        ) : tab === "meals" ? (
          <MealList meals={meals} onOpen={setMealId} />
        ) : (
          <div className="space-y-6">
            <PublicSearch />
            <div>
              <div className="eyebrow mb-2">Your ingredients</div>
              <IngredientList ingredients={ingredients} onEdit={setEditing} onDelete={(id) => start(() => deleteIngredient(id))} />
            </div>
          </div>
        )}
      </div>

      {(adding || editing) && <IngredientDialog editing={editing} onClose={() => { setAdding(false); setEditing(null); }} />}
    </div>
  );
}

function MealList({ meals, onOpen }: { meals: MealRow[]; onOpen: (id: number) => void }) {
  if (!meals.length) return <p className="text-sm text-muted">No meals yet.</p>;
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {meals.map((m) => {
        const perServing = Math.round(m.totalKcal / m.baseServings);
        const calOz = kcalPerOz(m.totalG ? (m.totalKcal / m.totalG) * 100 : null);
        return (
          <button key={m.id} onClick={() => onOpen(m.id)} className="cursor-pointer text-left">
            <Card className="h-full p-4 transition hover:border-accent/50">
              <div className="flex items-start justify-between gap-2">
                <span className="font-display font-semibold">{m.name}</span>
                {m.mealType && <Badge>{m.mealType}</Badge>}
              </div>
              <div className="readout mt-2 flex flex-wrap gap-x-4 gap-y-1 text-sm text-muted">
                <span>{perServing.toLocaleString()} kcal/serving</span>
                <span>{gToOz(Math.round(m.totalG / m.baseServings)).toFixed(1)} oz</span>
                {calOz != null && <span>{calOz.toFixed(0)} cal/oz</span>}
              </div>
              <div className="eyebrow mt-2">{m.items.length} ingredient{m.items.length === 1 ? "" : "s"} · yields {m.baseServings}</div>
            </Card>
          </button>
        );
      })}
    </div>
  );
}

// Search USDA + Open Food Facts and add a result to the pantry on click.
function PublicSearch() {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<FoodHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [, start] = useTransition();

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

  const add = (h: FoodHit) => {
    start(() => addIngredientFromHit(h));
    setQ("");
  };

  return (
    <div>
      <div className="eyebrow mb-2">Add a food — search USDA + Open Food Facts</div>
      <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="e.g. peanut butter, ramen, Clif bar" />
      {(searching || hits.length > 0) && (
        <Card className="mt-2 max-h-72 overflow-y-auto p-0">
          {searching && <div className="px-4 py-2 text-sm text-muted">Searching…</div>}
          {hits.map((h) => (
            <button key={`${h.source}-${h.sourceId}`} onClick={() => add(h)} className="flex w-full items-center justify-between gap-3 border-b px-4 py-2 text-left last:border-0 hover:bg-panel2/60">
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium">{h.name}</span>
                {h.detail && <span className="block truncate text-xs text-muted">{h.detail}</span>}
              </span>
              <span className="readout flex shrink-0 items-center gap-3 text-xs text-muted">
                {h.kcalPer100g != null && <span>{h.kcalPer100g} kcal/100g</span>}
                {kcalPerOz(h.kcalPer100g) != null && <span>{kcalPerOz(h.kcalPer100g)!.toFixed(0)} cal/oz</span>}
                <Badge tone={h.source === "usda" ? "base" : "wishlist"}>{h.source}</Badge>
              </span>
            </button>
          ))}
        </Card>
      )}
    </div>
  );
}

function IngredientList({ ingredients, onEdit, onDelete }: { ingredients: IngredientRow[]; onEdit: (i: IngredientRow) => void; onDelete: (id: number) => void }) {
  if (!ingredients.length) return <p className="text-sm text-muted">No saved ingredients.</p>;
  return (
    <>
      <div className="space-y-2 sm:hidden">
        {ingredients.map((i) => (
          <Card key={i.id} className="p-4">
            <div className="flex items-start justify-between gap-3">
              <button onClick={() => onEdit(i)} className="min-w-0 flex-1 text-left">
                <span className="block font-medium leading-snug break-words">{i.name}</span>
                {i.category && <span className="mt-0.5 block text-xs text-muted">{i.category}</span>}
              </button>
              <button
                onClick={() => confirm(`Delete "${i.name}"?`) && onDelete(i.id)}
                className="grid h-11 w-11 shrink-0 place-items-center rounded text-muted hover:bg-accent/10 hover:text-accent"
                aria-label={`Delete ${i.name}`}
              >
                ✕
              </button>
            </div>
            <div className="readout mt-3 grid grid-cols-3 gap-2 border-t pt-3 text-center text-sm">
              <MobileNutrition label="kcal/100g" value={i.kcalPer100g != null ? Math.round(i.kcalPer100g) : "—"} />
              <MobileNutrition label="cal/oz" value={kcalPerOz(i.kcalPer100g)?.toFixed(0) ?? "—"} />
              <MobileNutrition label="g/mL" value={i.densityGMl ?? "—"} />
            </div>
          </Card>
        ))}
      </div>

      <Card className="hidden overflow-hidden sm:block">
        <div className="grid grid-cols-[minmax(0,1fr)_5rem_5rem_4rem_2rem] items-center gap-x-2 border-b bg-panel2/60 px-4 py-2">
          {["Ingredient", "kcal/100g", "cal/oz", "g/mL", ""].map((h, i) => (
            <span key={h || i} className={`eyebrow ${i > 0 && i < 4 ? "text-right" : ""}`}>{h}</span>
          ))}
        </div>
        {ingredients.map((i) => (
          <div key={i.id} className="grid grid-cols-[minmax(0,1fr)_5rem_5rem_4rem_2rem] items-center gap-x-2 border-b px-4 py-2 last:border-0 hover:bg-panel2/40">
            <button onClick={() => onEdit(i)} className="min-w-0 text-left">
              <span className="block truncate text-sm font-medium">{i.name}</span>
              {i.category && <span className="block truncate text-xs text-muted">{i.category}</span>}
            </button>
            <span className="readout text-right text-sm text-muted">{i.kcalPer100g != null ? Math.round(i.kcalPer100g) : "—"}</span>
            <span className="readout text-right text-sm text-muted">{kcalPerOz(i.kcalPer100g)?.toFixed(0) ?? "—"}</span>
            <span className="readout text-right text-sm text-muted">{i.densityGMl ?? "—"}</span>
            <button onClick={() => confirm(`Delete "${i.name}"?`) && onDelete(i.id)} className="text-muted hover:text-accent" title="Delete">✕</button>
          </div>
        ))}
      </Card>
    </>
  );
}

function MobileNutrition({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <span>
      <span className="block font-sans text-[0.6rem] uppercase tracking-wide text-muted">{label}</span>
      <span className="mt-0.5 block">{value}</span>
    </span>
  );
}
