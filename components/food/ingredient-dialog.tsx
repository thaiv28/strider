"use client";

import { useState } from "react";
import { Button, Input, Select, Field } from "@/components/ui";
import { Sheet } from "@/components/sheet";
import { createIngredient, updateIngredient, type IngredientInput } from "@/app/food/actions";
import type { FoodHit } from "@/lib/food-search";
import type { IngredientRow } from "@/lib/pantry";
import { useFoodSearch, FoodSearchResults, FOOD_SEARCH_PLACEHOLDER, foodSearchKeys } from "./food-search";
import { kcalPerOz, VOLUME_ML, type VolumeUnit } from "@/lib/food";

export function IngredientDialog({ editing, onClose }: { editing: IngredientRow | null; onClose: () => void }) {
  const [form, setForm] = useState<IngredientInput>({
    name: editing?.name ?? "",
    kcalPer100g: editing?.kcalPer100g ?? null,
    densityGMl: editing?.densityGMl ?? null,
    defaultServingG: editing?.defaultServingG ?? null,
    category: editing?.category ?? null,
    source: editing?.source ?? "manual",
    sourceId: editing?.sourceId ?? null,
    densitySource: editing?.densitySource ?? null,
    referenceName: editing?.referenceName ?? null,
    notes: editing?.notes ?? null,
  });
  const set = (patch: Partial<IngredientInput>) => setForm((f) => ({ ...f, ...patch }));

  const [measureUnit, setMeasureUnit] = useState<VolumeUnit>("tbsp");
  const [measureText, setMeasureText] = useState<string | null>(null);

  const [q, setQ] = useState("");
  const search = useFoodSearch(q);

  const pick = (h: FoodHit) => {
    setMeasureText(null);
    set({
      name: h.name,
      kcalPer100g: h.kcalPer100g,
      densityGMl: h.densityGMl,
      defaultServingG: h.defaultServingG,
      source: h.source,
      sourceId: h.sourceId,
      densitySource: h.reference ? "usda" : h.densityGMl != null ? "label" : null,
      referenceName: h.referenceName ?? null,
    });
  };

  const save = async () => {
    if (editing) await updateIngredient(editing.id, form);
    else await createIngredient(form);
    onClose();
  };

  const numOrNull = (v: string) => (v.trim() === "" ? null : Number(v));

  return (
    <Sheet open onClose={onClose} title={editing ? `Edit ${editing.name}` : "Add ingredient"}>
        {!editing && (
          <div className="mt-4">
            <Field label="Search foods">
              <Input type="search" value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={(e) => foodSearchKeys(e, () => setQ(""))} placeholder={FOOD_SEARCH_PLACEHOLDER} autoFocus />
            </Field>
            <FoodSearchResults query={q} search={search} onPick={(h) => { pick(h); setQ(""); }} />
          </div>
        )}

        <div className="mt-4 space-y-3">
          <Field label="Name">
            <Input value={form.name} onChange={(e) => set({ name: e.target.value })} />
          </Field>
          {form.referenceName && <p className="-mt-2 text-xs text-muted">USDA: {form.referenceName}</p>}
          <div className="grid grid-cols-2 gap-3">
            <Field label="kcal / 100g">
              <Input type="number" step="1" value={form.kcalPer100g ?? ""} onChange={(e) => set({ kcalPer100g: numOrNull(e.target.value) })} />
            </Field>
            <Field label="Serving (g)">
              <Input type="number" step="1" value={form.defaultServingG ?? ""} onChange={(e) => set({ defaultServingG: numOrNull(e.target.value) })} />
            </Field>
          </div>
          <Field label="Weight by volume">
            <span className="flex items-center gap-2">
              <span>1</span>
              <Select aria-label="Volume unit" value={measureUnit} onChange={(e) => { setMeasureUnit(e.target.value as VolumeUnit); setMeasureText(null); }}>
                {["tsp", "tbsp", "floz", "cup"].map((u) => <option key={u} value={u}>{u}</option>)}
              </Select>
              <span>=</span>
              <Input aria-label="Weight by volume grams" className="min-w-0 flex-1" type="number" min="0" step="0.1"
                value={measureText ?? (form.densityGMl == null ? "" : (form.densityGMl * VOLUME_ML[measureUnit]).toFixed(1))}
                onChange={(e) => { const text = e.target.value; setMeasureText(text); set({ densitySource: text.trim() === "" ? null : "manual", referenceName: null, densityGMl: text.trim() === "" ? null : +(Number(text) / VOLUME_ML[measureUnit]).toFixed(3) }); }} />
              <span>g</span>
            </span>
          </Field>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Field label="Category (optional)">
              <Input value={form.category ?? ""} onChange={(e) => set({ category: e.target.value || null })} />
            </Field>
            <div className="flex items-end">
              <span className="readout text-xs text-muted">
                {kcalPerOz(form.kcalPer100g) != null ? `${kcalPerOz(form.kcalPer100g)!.toFixed(0)} cal/oz` : "cal/oz —"}
                 {" · "}{form.densityGMl != null ? `≈ ${form.densityGMl} g/mL` : "Add a weight to measure by volume"}
              </span>
            </div>
          </div>
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button onClick={save} disabled={!form.name.trim()}>{editing ? "Save" : "Add"}</Button>
        </div>
    </Sheet>
  );
}
