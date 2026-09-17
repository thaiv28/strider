"use client";

import { useState, useTransition } from "react";
import { Card, Button, Input, Select } from "@/components/ui";
import { mifflinBmr, type EnergyParams } from "@/lib/energy";
import { updateEnergyParams } from "@/app/settings/actions";
import { useUnsavedGuard } from "@/components/unsaved-changes";

const numOrNull = (v: string) => (v.trim() === "" ? null : Number(v));

export function EnergyModel({ params }: { params: EnergyParams }) {
  const [, start] = useTransition();
  const [saved, setSaved] = useState(false);
  const [f, setF] = useState(params);
  const set = <K extends keyof EnergyParams>(k: K, v: EnergyParams[K]) => {
    setF((p) => ({ ...p, [k]: v }));
    setSaved(false);
  };

  const ft = f.heightIn != null ? Math.floor(f.heightIn / 12) : null;
  const inches = f.heightIn != null ? Math.round(f.heightIn % 12) : null;
  const setHeight = (feet: number | null, inch: number | null) => {
    if (feet == null && inch == null) return set("heightIn", null);
    set("heightIn", (feet ?? 0) * 12 + (inch ?? 0));
  };

  const calc = mifflinBmr(f.sex, f.weightLb, f.heightIn, f.ageYears);
  const dirty = JSON.stringify(f) !== JSON.stringify(params);

  // Example day to show the model's effect.
  const exMove = Math.round((8 + 2000 / f.ftPerMile) * f.calPerMile);

  const save = () =>
    start(async () => {
      await updateEnergyParams(f);
      setSaved(true);
    });

  useUnsavedGuard("energy", dirty, async () => {
    await updateEnergyParams(f);
    setSaved(true);
  });

  return (
    <Card className="p-5">
      <div className="eyebrow">Energy model</div>
      <p className="mt-2 text-sm text-muted">
        Each day&apos;s calorie target is your resting burn plus the effort of moving:
      </p>
      <p className="readout mt-2 rounded-md border bg-panel2/40 px-3 py-2 text-sm">
        target = <b>{f.bmr.toLocaleString()}</b> + ( miles + gain_ft / <b>{f.ftPerMile}</b> ) × <b>{f.calPerMile}</b>
      </p>

      <div className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3">
        <Field label="BMR (resting kcal/day)">
          <Input type="number" value={f.bmr} onChange={(e) => set("bmr", Number(e.target.value) || 0)} />
        </Field>
        <Field label="kcal per energy-mile">
          <Input type="number" value={f.calPerMile} onChange={(e) => set("calPerMile", Number(e.target.value) || 0)} />
        </Field>
        <Field label="ft of gain = 1 mile">
          <Input type="number" value={f.ftPerMile} onChange={(e) => set("ftPerMile", Number(e.target.value) || 1)} />
        </Field>
      </div>
      <p className="mt-2 text-xs text-muted">
        Example — an 8 mi day with 2,000 ft gain burns {f.bmr.toLocaleString()} + {exMove.toLocaleString()} = <b>{(f.bmr + exMove).toLocaleString()}</b> kcal.
      </p>

      <div className="mt-5 rounded-lg border bg-panel2/30 p-4">
        <div className="eyebrow">BMR calculator (Mifflin–St Jeor)</div>
        <div className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Field label="Sex">
            <Select value={f.sex ?? ""} onChange={(e) => set("sex", e.target.value || null)} className="w-full">
              <option value="">—</option>
              <option value="male">male</option>
              <option value="female">female</option>
            </Select>
          </Field>
          <Field label="Weight (lb)">
            <Input type="number" value={f.weightLb ?? ""} onChange={(e) => set("weightLb", numOrNull(e.target.value))} />
          </Field>
          <Field label="Height">
            <div className="flex items-center gap-1">
              <Input type="number" placeholder="ft" value={ft ?? ""} onChange={(e) => setHeight(numOrNull(e.target.value), inches)} className="w-full" />
              <Input type="number" placeholder="in" value={inches ?? ""} onChange={(e) => setHeight(ft, numOrNull(e.target.value))} className="w-full" />
            </div>
          </Field>
          <Field label="Age">
            <Input type="number" value={f.ageYears ?? ""} onChange={(e) => set("ageYears", numOrNull(e.target.value) as number | null)} />
          </Field>
        </div>
        <div className="mt-3 flex items-center gap-3">
          <span className="text-sm text-muted">
            Calculated BMR: <b className="text-ink">{calc != null ? `${calc.toLocaleString()} kcal` : "—"}</b>
          </span>
          <Button variant="outline" disabled={calc == null || calc === f.bmr} onClick={() => set("bmr", calc!)}>
            Use as BMR
          </Button>
        </div>
      </div>

      <div className="mt-5 flex items-center justify-end gap-3">
        {saved && <span className="text-sm text-muted">Saved.</span>}
        <Button disabled={!dirty} onClick={save}>Save</Button>
      </div>
    </Card>
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
