"use client";

import { useState, useTransition } from "react";
import { Card, Button } from "@/components/ui";
import { updateReportSettings } from "@/app/settings/actions";
import { useUnsavedGuard } from "@/components/unsaved-changes";

const SCALAR_TOKENS = [
  "name", "region", "area_type", "start_date", "end_date", "nights", "days",
  "distance", "gain", "trailhead", "permits", "water_sources", "driving",
  "planning_notes", "generated_on",
];
const BLOCK_HELP = [
  ["route_map", "terrain map image: route + campsites"],
  ["itinerary", "day-by-day table"],
  ["route", "campsites at each night"],
  ["weather", "per-day forecast"],
  ["food_plan", "meals by day"],
  ["shopping_list", "consolidated grocery list (checkable)"],
  ["packing_list", "the trip's packing list"],
] as const;

const ta =
  "mt-2 w-full resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25";

export function ReportTemplate({ template, packingDefault }: { template: string; packingDefault: string }) {
  const [, start] = useTransition();
  const [saved, setSaved] = useState(false);
  const [tpl, setTpl] = useState(template);
  const [pack, setPack] = useState(packingDefault);
  const [showTokens, setShowTokens] = useState(false);

  const dirty = tpl !== template || pack !== packingDefault;
  const save = () =>
    start(async () => {
      await updateReportSettings({ template: tpl, packingDefault: pack });
      setSaved(true);
    });

  useUnsavedGuard("report-template", dirty, async () => {
    await updateReportSettings({ template: tpl, packingDefault: pack });
    setSaved(true);
  });

  return (
    <Card className="p-5">
      <div className="eyebrow">Trip report template</div>
      <p className="mt-2 text-sm text-muted">
        The Markdown document behind each trip&apos;s shareable report. Use <code className="readout">{"{{tokens}}"}</code> to
        drop in trip data — empty ones remove their line.
      </p>

      <button onClick={() => setShowTokens((s) => !s)} className="eyebrow mt-3 hover:text-accent">
        {showTokens ? "hide" : "show"} available tokens
      </button>
      {showTokens && (
        <div className="mt-2 rounded-lg border bg-panel2/30 p-3 text-xs">
          <div className="eyebrow">Inline</div>
          <div className="readout mt-1 flex flex-wrap gap-x-3 gap-y-1 text-muted">
            {SCALAR_TOKENS.map((t) => <span key={t}>{`{{${t}}}`}</span>)}
          </div>
          <div className="eyebrow mt-3">Sections (toggleable at generate time)</div>
          <div className="mt-1 space-y-0.5 text-muted">
            {BLOCK_HELP.map(([t, desc]) => (
              <div key={t}><span className="readout text-ink">{`{{${t}}}`}</span> — {desc}</div>
            ))}
          </div>
        </div>
      )}

      <label className="mt-4 block">
        <span className="eyebrow">Template</span>
        <textarea value={tpl} onChange={(e) => { setTpl(e.target.value); setSaved(false); }} rows={18} spellCheck={false} className={`${ta} readout`} />
      </label>

      <label className="mt-4 block">
        <span className="eyebrow">Default packing list</span>
        <span className="mt-0.5 block text-xs text-muted">New trips start from this; each trip&apos;s copy is editable on its Report tab.</span>
        <textarea value={pack} onChange={(e) => { setPack(e.target.value); setSaved(false); }} rows={10} className={ta} />
      </label>

      <div className="mt-5 flex items-center justify-end gap-3">
        {saved && <span className="text-sm text-muted">Saved.</span>}
        <Button disabled={!dirty} onClick={save}>Save</Button>
      </div>
    </Card>
  );
}
