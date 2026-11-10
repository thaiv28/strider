"use client";

import { useRef, useState, useTransition } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Card, Button, Input, PageTabs, Field, IconButton } from "@/components/ui";
import { PageHeader, PageBody } from "@/components/page-header";
import { Trash2 } from "lucide-react";
import { gToOz } from "@/lib/util";
import { kcalPerOz, VOLUME_ML } from "@/lib/food";
import type { IngredientRow, MealRow } from "@/lib/pantry";
import type { FoodHit } from "@/lib/food-search";
import { createMeal, deleteIngredient, addIngredientFromHit } from "@/app/food/actions";
import { IngredientDialog } from "./ingredient-dialog";
import { useFoodSearch, FoodSearchResults, FOOD_SEARCH_PLACEHOLDER, foodSearchKeys } from "./food-search";
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

  const exitRef = useRef<((next: () => void) => void) | null>(null);
  const navigate = (next: () => void) => exitRef.current ? exitRef.current(next) : next();
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

  const tabs = <PageTabs tone="header" label="Food sections" tabs={[{ value: "meals", label: "Meals" }, { value: "ingredients", label: "Ingredients" }]} value={tab} onChange={(value) => navigate(() => { setMealId(null); setTab(value); })} />;
  const actions = <Button variant="header" data-food-navigation onClick={() => navigate(() => {
    if (tab === "ingredients") setAdding(true);
    else start(async () => { const id = await createMeal("New meal"); setCreatedId(id); setMealId(id); });
  })}>{tab === "ingredients" ? "Add ingredient" : "Add meal"}</Button>;

  return (
    <>
      <div data-food-navigation className={openMeal ? "hidden sm:block" : ""}>
        <PageHeader title="Food" actions={actions}>{tabs}</PageHeader>
      </div>
      {openMeal && <div className="sm:hidden [&_h1]:min-w-0 [&_h1]:break-words" data-food-navigation><PageHeader title={openMeal.name}>
        <button className="min-h-11 text-sm font-semibold text-headerink" onClick={() => navigate(closeMeal)}>← Back to meals</button>
      </PageHeader></div>}
      <PageBody>
        {tab === "meals" || openMeal ? (
          <div className={openMeal ? "grid items-start gap-4 desk:grid-cols-[minmax(0,1fr)_460px]" : ""}>
            <div className={openMeal ? "hidden min-w-0 desk:block" : "min-w-0"} data-food-navigation>
              <MealList meals={meals} selectedId={openMeal?.id} onOpen={(id) => { if (id !== mealId) navigate(() => { setCreatedId(null); setMealId(id); }); }} />
            </div>
            {openMeal && <MealEditor key={openMeal.id} meal={openMeal} ingredients={ingredients} onBack={closeMeal} exitRef={exitRef} isNew={openMeal.id === createdId} />}
          </div>
        ) : (
          <div className="space-y-3">
            <PublicSearch />
            <IngredientList ingredients={ingredients} onEdit={setEditing} onDelete={(id) => start(() => deleteIngredient(id))} />
          </div>
        )}
      </PageBody>
      {(adding || editing) && <IngredientDialog editing={editing} onClose={() => { setAdding(false); setEditing(null); }} />}
    </>
  );
}

function MealList({ meals, selectedId, onOpen }: { meals: MealRow[]; selectedId?: number; onOpen: (id: number) => void }) {
  if (!meals.length) return <p className="text-sm text-muted">No meals yet.</p>;
  return (
    <div className={`grid gap-3 ${selectedId ? "" : "sm:grid-cols-2"}`}>
      {meals.map((m) => {
        const perServing = Math.round(m.totalKcal / m.baseServings);
        const calOz = kcalPerOz(m.totalG ? (m.totalKcal / m.totalG) * 100 : null);
        return (
          <button key={m.id} onClick={() => onOpen(m.id)} aria-pressed={selectedId === m.id} className="min-w-0 cursor-pointer rounded-2xl text-left focus-visible:outline-2 focus-visible:outline-accent">
            <Card className={`h-full p-4 transition hover:border-accent/50 ${selectedId === m.id ? "border-accent ring-1 ring-accent" : ""}`}>
              <div className="flex items-start justify-between gap-2">
                <span className="min-w-0 font-display text-[19px] font-bold leading-tight break-words">{m.name}</span>
                {m.mealType && <span className="shrink-0 text-[13px] text-muted">{m.mealType}</span>}
              </div>
              <div className="readout mt-2 flex flex-wrap gap-x-1 text-sm">
                <span>{perServing.toLocaleString()} kcal/serving</span>
                <span>· {gToOz(Math.round(m.totalG / m.baseServings)).toFixed(1)} oz</span>
                {calOz != null && <span>· {calOz.toFixed(0)} cal/oz</span>}
              </div>
              <div className="mt-1 text-[13px] text-muted">{m.items.length} ingredient{m.items.length === 1 ? "" : "s"} · yields {m.baseServings}</div>
            </Card>
          </button>
        );
      })}
    </div>
  );
}

// Add a reference food, with packaged search available on request.
function PublicSearch() {
  const [q, setQ] = useState("");
  const search = useFoodSearch(q);
  const [, start] = useTransition();

  const add = (h: FoodHit) => {
    start(() => addIngredientFromHit(h));
    setQ("");
  };

  return (
    <Card className="p-4">
      <h2 className="text-[17px] font-bold">Add a food</h2>
      <p className="mb-3 text-[13px] text-muted">USDA reference foods; packaged products on request</p>
      <Field label="Food search"><Input type="search" value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={(e) => foodSearchKeys(e, () => setQ(""))} placeholder={FOOD_SEARCH_PLACEHOLDER} /></Field>
      <FoodSearchResults query={q} search={search} onPick={add} />
    </Card>
  );
}

function IngredientList({ ingredients, onEdit, onDelete }: { ingredients: IngredientRow[]; onEdit: (i: IngredientRow) => void; onDelete: (id: number) => void }) {
  if (!ingredients.length) return <p className="text-sm text-muted">No saved ingredients.</p>;
  return (
    <>
      <Card className="overflow-hidden" aria-label="Your ingredients">
        <div className="grid grid-cols-[minmax(0,1fr)_3.5rem_2.75rem_3rem_2.75rem] sm:grid-cols-[minmax(0,1fr)_5rem_5rem_4rem_2.75rem] items-center gap-x-1 border-b border-divider bg-panel2/60 px-2 py-2 sm:gap-x-2 sm:px-4">
          {["Ingredient", "kcal/100g", "cal/oz", "g/tbsp", ""].map((h, i) => (
            <span key={h || i} className={`text-[11px] font-semibold text-muted sm:text-[13px] ${i > 0 && i < 4 ? "text-right" : ""}`}>{h}</span>
          ))}
        </div>
        {ingredients.map((i) => (
          <div key={i.id} className="grid grid-cols-[minmax(0,1fr)_3.5rem_2.75rem_3rem_2.75rem] sm:grid-cols-[minmax(0,1fr)_5rem_5rem_4rem_2.75rem] items-center gap-x-1 border-b border-divider px-2 py-2 sm:gap-x-2 sm:px-4 last:border-0 hover:bg-panel2/40">
            <button data-food-source={i.source} data-food-source-id={i.sourceId} onClick={() => onEdit(i)} className="min-w-0 text-left">
              <span className="block break-words text-sm font-medium">{i.name}</span>
              {i.category && <span className="block break-words text-xs text-muted">{i.category}</span>}
            </button>
            <span className="readout text-right text-sm text-muted">{i.kcalPer100g != null ? Math.round(i.kcalPer100g) : "—"}</span>
            <span className="readout text-right text-sm text-muted">{kcalPerOz(i.kcalPer100g)?.toFixed(0) ?? "—"}</span>
            <span className="readout text-right text-sm text-muted">{i.densityGMl == null ? "—" : (i.densityGMl * VOLUME_ML.tbsp).toFixed(1)}</span>
            <IconButton onClick={() => confirm(`Delete "${i.name}"?`) && onDelete(i.id)} aria-label={`Delete ${i.name}`} className="text-signal"><Trash2 size={16} /></IconButton>
          </div>
        ))}
      </Card>
    </>
  );
}
