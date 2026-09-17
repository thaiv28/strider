"use client";

import { useEffect, useState } from "react";
import { Card, Button, Input, Badge } from "@/components/ui";
import { searchNutrition, createIngredient, updateIngredient, type IngredientInput } from "@/app/food/actions";
import type { FoodHit } from "@/lib/food-search";
import type { IngredientRow } from "@/lib/pantry";
import { kcalPerOz } from "@/lib/food";

export function IngredientDialog({ editing, onClose }: { editing: IngredientRow | null; onClose: () => void }) {
  const [form, setForm] = useState<IngredientInput>({
    name: editing?.name ?? "",
    kcalPer100g: editing?.kcalPer100g ?? null,
    densityGMl: editing?.densityGMl ?? null,
    defaultServingG: editing?.defaultServingG ?? null,
    category: editing?.category ?? null,
    source: editing?.source ?? "manual",
    sourceId: null,
    notes: editing?.notes ?? null,
  });
  const set = (patch: Partial<IngredientInput>) => setForm((f) => ({ ...f, ...patch }));

  const [q, setQ] = useState("");
  const [hits, setHits] = useState<FoodHit[]>([]);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    if (editing) return; // search only when adding new
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
  }, [q, editing]);

  const pick = (h: FoodHit) =>
    set({
      name: h.name,
      kcalPer100g: h.kcalPer100g,
      densityGMl: h.densityGMl,
      defaultServingG: h.defaultServingG,
      source: h.source,
      sourceId: h.sourceId,
    });

  const save = async () => {
    if (editing) await updateIngredient(editing.id, form);
    else await createIngredient(form);
    onClose();
  };

  const numOrNull = (v: string) => (v.trim() === "" ? null : Number(v));

  return (
    <div className="fixed inset-0 z-[1100] flex items-start justify-center overflow-y-auto bg-ink/40 p-4 py-10 backdrop-blur-sm" onClick={onClose}>
      <Card className="w-full max-w-lg p-5" onClick={(e) => e.stopPropagation()}>
        <div className="eyebrow">◇ {editing ? "Edit ingredient" : "New ingredient"}</div>
        <h2 className="font-display mt-1 text-lg font-bold">{editing ? editing.name : "Add ingredient"}</h2>

        {!editing && (
          <div className="mt-4">
            <span className="eyebrow mb-1 block">Search USDA + Open Food Facts</span>
            <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="e.g. peanut butter, ramen, Clif bar" autoFocus />
            {(searching || hits.length > 0) && (
              <div className="mt-2 max-h-56 overflow-y-auto rounded-lg border">
                {searching && <div className="px-3 py-2 text-sm text-muted">Searching…</div>}
                {hits.map((h) => (
                  <button
                    key={`${h.source}-${h.sourceId}`}
                    onClick={() => pick(h)}
                    className="flex w-full items-center justify-between gap-3 border-b px-3 py-2 text-left last:border-0 hover:bg-panel2/60"
                  >
                    <span className="min-w-0">
                      <span className="block truncate text-sm font-medium">{h.name}</span>
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
        )}

        <div className="mt-4 space-y-3">
          <Field label="Name">
            <Input value={form.name} onChange={(e) => set({ name: e.target.value })} />
          </Field>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <Field label="kcal / 100g">
              <Input type="number" step="1" value={form.kcalPer100g ?? ""} onChange={(e) => set({ kcalPer100g: numOrNull(e.target.value) })} />
            </Field>
            <Field label="Density (g/mL)">
              <Input type="number" step="0.01" value={form.densityGMl ?? ""} onChange={(e) => set({ densityGMl: numOrNull(e.target.value) })} />
            </Field>
            <Field label="Serving (g)">
              <Input type="number" step="1" value={form.defaultServingG ?? ""} onChange={(e) => set({ defaultServingG: numOrNull(e.target.value) })} />
            </Field>
          </div>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Field label="Category (optional)">
              <Input value={form.category ?? ""} onChange={(e) => set({ category: e.target.value || null })} />
            </Field>
            <div className="flex items-end">
              <span className="readout text-xs text-muted">
                {kcalPerOz(form.kcalPer100g) != null ? `${kcalPerOz(form.kcalPer100g)!.toFixed(0)} cal/oz` : "cal/oz —"}
                {form.densityGMl == null && " · no density → volume disabled"}
              </span>
            </div>
          </div>
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button onClick={save} disabled={!form.name.trim()}>{editing ? "Save" : "Add"}</Button>
        </div>
      </Card>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="eyebrow mb-1 block">{label}</span>
      {children}
    </label>
  );
}
