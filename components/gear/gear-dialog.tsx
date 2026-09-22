"use client";

import { useState } from "react";
import { Card, Button, Input, Select } from "@/components/ui";
import { displayOz, ozToG } from "@/lib/util";
import type { GearRow, CategoryOption } from "@/lib/gear";

type CompDraft = { name: string; oz: string; quantity: number; notes: string };

const toDraft = (r: GearRow | null): CompDraft[] =>
  (r?.components ?? []).map((c) => ({
    name: c.name,
    oz: c.weightG ? displayOz(c.weightG).toFixed(2) : "",
    quantity: c.quantity,
    notes: c.notes ?? "",
  }));

export function GearDialog({
  editing,
  categoryOptions,
  onClose,
  onSubmit,
}: {
  editing: GearRow | null;
  categoryOptions: CategoryOption[];
  onClose: () => void;
  onSubmit: (fd: FormData) => void;
}) {
  const [isKit, setIsKit] = useState(editing?.isKit ?? false);
  const [comps, setComps] = useState<CompDraft[]>(toDraft(editing));

  const kitTotalG = comps.reduce((s, c) => s + (c.oz.trim() ? ozToG(Number(c.oz) || 0) : 0) * c.quantity, 0);
  const setComp = (i: number, patch: Partial<CompDraft>) => setComps((cs) => cs.map((c, j) => (j === i ? { ...c, ...patch } : c)));
  const componentsJson = JSON.stringify(
    comps
      .filter((c) => c.name.trim())
      .map((c) => ({ name: c.name.trim(), weightG: c.oz.trim() ? ozToG(Number(c.oz) || 0) : 0, quantity: c.quantity, notes: c.notes.trim() || null })),
  );

  return (
    <div
      className="fixed inset-0 z-[1100] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm"
      onClick={onClose}
    >
      <Card className="flex max-h-[88vh] w-full max-w-lg flex-col overflow-hidden p-5" onClick={(e) => e.stopPropagation()}>
        <div className="eyebrow">◇ {editing ? "Edit entry" : "New entry"}</div>
        <h2 className="font-display mt-1 text-lg font-bold">{editing ? editing.name : "Add gear"}</h2>
        <form
          className="mt-4 flex min-h-0 flex-col"
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit(new FormData(e.currentTarget));
          }}
        >
          <input type="hidden" name="isKit" value={String(isKit)} />
          <input type="hidden" name="components" value={componentsJson} />
          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
          <Field label="Name">
            <Input name="name" defaultValue={editing?.name ?? ""} required autoFocus />
          </Field>

          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={isKit} onChange={(e) => setIsKit(e.target.checked)} className="h-4 w-4 accent-[var(--accent)]" />
            This is a kit (holds a list of items; weight is the sum of its contents)
          </label>

          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <Field label={isKit ? "Total (oz)" : "Unit (oz)"}>
              {isKit ? (
                <div className="readout rounded-[calc(var(--radius)*0.6)] border bg-panel2/40 px-3 py-1.5 text-sm text-muted">
                  {displayOz(kitTotalG).toFixed(2)}
                </div>
              ) : (
                <Input
                  name="weightOz"
                  type="number"
                  step="0.01"
                  defaultValue={editing?.weightG != null ? displayOz(editing.weightG).toFixed(2) : ""}
                />
              )}
            </Field>
            <Field label="Qty">
              <Input name="quantity" type="number" min="1" step="1" defaultValue={editing?.quantity ?? 1} />
            </Field>
            <Field label="Class">
              <Select name="defaultWeightClass" defaultValue={editing?.defaultWeightClass ?? "base"} className="w-full">
                <option value="base">base</option>
                <option value="worn">worn</option>
              </Select>
            </Field>
          </div>

          {isKit && (
            <div className="rounded-md border bg-panel2/30 p-3">
              <div className="eyebrow mb-2">Kit contents</div>
              <div className="space-y-2">
                {comps.map((c, i) => (
                  <div key={i} className="grid grid-cols-[minmax(0,1fr)_4.5rem_3rem_1.5rem] items-center gap-2">
                    <Input placeholder="Item" value={c.name} onChange={(e) => setComp(i, { name: e.target.value })} />
                    <Input placeholder="oz" type="number" step="0.01" value={c.oz} onChange={(e) => setComp(i, { oz: e.target.value })} />
                    <Input placeholder="qty" type="number" min="1" step="1" value={c.quantity} onChange={(e) => setComp(i, { quantity: Math.max(1, Number(e.target.value) || 1) })} />
                    <button
                      type="button"
                      onClick={() => setComps((cs) => cs.filter((_, j) => j !== i))}
                      aria-label="Remove component"
                      className="justify-self-end rounded p-1 text-muted transition hover:bg-accent/10 hover:text-accent"
                    >
                      ✕
                    </button>
                  </div>
                ))}
                {comps.length === 0 && <p className="text-xs text-muted">No contents yet.</p>}
              </div>
              <button
                type="button"
                onClick={() => setComps((cs) => [...cs, { name: "", oz: "", quantity: 1, notes: "" }])}
                className="mt-2 text-sm text-accent hover:underline"
              >
                + Add item
              </button>
            </div>
          )}
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <Field label="Price (USD, optional)">
              <Input
                name="priceUsd"
                type="number"
                step="0.01"
                min="0"
                defaultValue={editing?.priceCents != null ? (editing.priceCents / 100).toFixed(2) : ""}
              />
            </Field>
            <Field label="Category">
              <Select name="categoryId" defaultValue={editing?.categoryId ?? ""} className="w-full">
                <option value="">— none —</option>
                {categoryOptions.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Status">
              <Select name="status" defaultValue={editing?.status ?? "current"} className="w-full">
                <option value="current">current</option>
                <option value="retired">retired</option>
                <option value="wishlist">wishlist</option>
              </Select>
            </Field>
          </div>
          <Field label="Product URL (optional)">
            <Input name="productUrl" type="url" defaultValue={editing?.productUrl ?? ""} />
          </Field>
          <Field label="Notes (optional)">
            <Input name="notes" defaultValue={editing?.notes ?? ""} />
          </Field>
          </div>
          <div className="flex justify-end gap-2 pt-3">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit">{editing ? "Save" : "Add"}</Button>
          </div>
        </form>
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
