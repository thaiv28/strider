"use client";

import { useEffect, useMemo, useState, useTransition } from "react";
import { Card, Button } from "@/components/ui";
import { addDays, fetchDay, isFarFuture, type WxResult } from "@/lib/weather";
import { getClimatology } from "@/app/weather/actions";
import {
  buildScalars,
  markdownToHtml,
  renderFoodPlan,
  renderItinerary,
  renderReport,
  renderRoute,
  renderShoppingList,
  renderWeather,
  templateBlocks,
  type BlockToken,
  type ReportData,
  type WxRow,
} from "@/lib/report";
import { routeMapUrl, saveTripPackingList } from "@/app/trips/actions";

const ta =
  "mt-2 w-full resize-y rounded-md border bg-panel2 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/25";

type Row = { label: string; date: string; lat: number | null; lon: number | null };

// Mirror the weather panel's row construction: one per campsite night plus the
// finish, or per-day at the trip coords when no campsites exist.
function weatherRows(d: ReportData): Row[] {
  if (!d.startDate) return [];
  if (d.campsites.length) {
    const rows: Row[] = d.campsites.map((c, i) => ({ label: `Day ${i + 1}`, date: addDays(d.startDate!, i), lat: c.lat, lon: c.lon }));
    const fin = d.perDayAuto[d.perDayAuto.length - 1];
    if (fin && fin.lat != null && fin.lon != null)
      rows.push({ label: `Day ${d.campsites.length + 1}`, date: addDays(d.startDate, d.nights ?? d.campsites.length), lat: fin.lat, lon: fin.lon });
    return rows;
  }
  if (d.lat != null && d.lon != null)
    return Array.from({ length: d.days }, (_, i) => ({ label: d.days > 1 ? `Day ${i + 1}` : "Forecast", date: addDays(d.startDate!, i), lat: d.lat, lon: d.lon }));
  return [];
}

export function ReportPanel({
  tripId,
  data,
  template,
  packingList,
  generateRequest = 0,
}: {
  tripId: number;
  data: ReportData;
  template: string;
  packingList: string;
  generateRequest?: number;
}) {
  const [, start] = useTransition();
  const [pack, setPack] = useState(packingList);
  const [savedPack, setSavedPack] = useState(true);

  const blocks = useMemo(() => templateBlocks(template), [template]);
  const [config, setConfig] = useState(false);
  const [on, setOn] = useState<Record<string, boolean>>({});
  const [busy, setBusy] = useState(false);
  const [out, setOut] = useState<{ md: string; html: string } | null>(null);

  const openConfig = () => {
    setOn(Object.fromEntries(blocks.map((b) => [b.token, true])));
    setConfig(true);
  };

  useEffect(() => {
    if (generateRequest > 0) openConfig();
    // openConfig intentionally uses the latest rendered template blocks.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [generateRequest]);

  const generate = async () => {
    setBusy(true);
    try {
      const has = (t: BlockToken) => blocks.some((b) => b.token === t);
      let weatherMd = "";
      if (on.weather && has("weather")) {
        const rows = weatherRows(data);
        const results = await Promise.all(
          rows.map(async (r): Promise<WxRow["result"]> => {
            if (isFarFuture(r.date)) {
              const c = await getClimatology(r.lat, r.lon, r.date);
              return c ? { kind: "climatology", climo: c } : { kind: "unavailable", reason: "no climate data" };
            }
            return (await fetchDay(r.lat, r.lon, r.date, data.elevationFt)) as WxResult;
          }),
        );
        weatherMd = renderWeather(rows.map((r, i): WxRow => ({ label: r.label, date: r.date, result: results[i] })));
      }
      let mapMd = "";
      if (on.route_map && has("route_map")) {
        const url = await routeMapUrl(data.track, data.campsites);
        if (url) mapMd = `![Route map](${url})`;
      }
      const withData = { ...data, packingList: pack };
      const render: Record<BlockToken, string> = {
        route_map: mapMd,
        itinerary: renderItinerary(withData),
        route: renderRoute(withData),
        weather: weatherMd,
        food_plan: renderFoodPlan(withData),
        shopping_list: renderShoppingList(withData),
        packing_list: pack.trim(),
      };
      const blockValues: Record<string, string> = {};
      for (const b of blocks) blockValues[b.token] = on[b.token] ? render[b.token] : "";
      const md = renderReport(template, buildScalars(withData), blockValues);
      setOut({ md, html: markdownToHtml(md) });
      setConfig(false);
    } finally {
      setBusy(false);
    }
  };

  const savePack = () =>
    start(async () => {
      await saveTripPackingList(tripId, pack);
      setSavedPack(true);
    });

  return (
    <div className="space-y-6">
      <Card className="p-4">
        <div className="flex items-center justify-between">
          <div className="eyebrow">Packing list — shared</div>
          {!savedPack && <span className="text-xs text-muted">unsaved</span>}
        </div>
        <textarea
          value={pack}
          onChange={(e) => { setPack(e.target.value); setSavedPack(false); }}
          rows={10}
          className={ta}
        />
        <div className="mt-2 flex justify-end">
          <Button variant="outline" disabled={savedPack} onClick={savePack}>Save packing list</Button>
        </div>
      </Card>

      {config && (
        <div className="fixed inset-0 z-[1100] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm" onClick={() => !busy && setConfig(false)}>
          <Card className="w-full max-w-sm p-5" onClick={(e) => e.stopPropagation()}>
            <div className="eyebrow">◇ Include in report</div>
            {blocks.length === 0 ? (
              <p className="mt-3 text-sm text-muted">Your template has no toggleable sections — it&apos;ll generate as-is.</p>
            ) : (
              <div className="mt-3 space-y-2">
                {blocks.map((b) => (
                  <label key={b.token} className="flex items-center gap-2 text-sm">
                    <input type="checkbox" checked={!!on[b.token]} onChange={(e) => setOn((o) => ({ ...o, [b.token]: e.target.checked }))} className="h-4 w-4 accent-[var(--accent)]" />
                    {b.label}
                  </label>
                ))}
              </div>
            )}
            <div className="mt-5 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setConfig(false)} disabled={busy}>Cancel</Button>
              <Button onClick={generate} disabled={busy}>{busy ? "Generating…" : "Generate"}</Button>
            </div>
          </Card>
        </div>
      )}

      {out && <PreviewModal md={out.md} html={out.html} name={data.name} onClose={() => setOut(null)} />}
    </div>
  );
}

function PreviewModal({ md, html, name, onClose }: { md: string; html: string; name: string; onClose: () => void }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    // Apple Notes (and Google Docs) ignore heading CSS margins on paste, so
    // headings land flush against the text above them. Inject an empty paragraph
    // before each heading — that survives as a real blank line. Skip a leading one.
    const spacedHtml = html
      .replace(/(<h[1-6][^>]*>)/gi, "<p>&nbsp;</p>$1")
      .replace(/^\s*<p>&nbsp;<\/p>/i, "")
      .replace(/(<\/h1>)/i, "$1<p>&nbsp;</p>"); // breathing room under the title
    try {
      await navigator.clipboard.write([
        new ClipboardItem({
          "text/html": new Blob([spacedHtml], { type: "text/html" }),
          "text/plain": new Blob([md], { type: "text/plain" }),
        }),
      ]);
    } catch {
      await navigator.clipboard.writeText(md);
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  const download = () => {
    const url = URL.createObjectURL(new Blob([md], { type: "text/markdown" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `${name.replace(/[^\w-]+/g, "_") || "trip"}-report.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="fixed inset-0 z-[1100] flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm" onClick={onClose}>
      <Card className="flex max-h-[85vh] w-full max-w-2xl flex-col p-5" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <div className="eyebrow">◇ Report preview</div>
          <button onClick={onClose} className="text-muted hover:text-accent">✕</button>
        </div>
        <div
          className="report-preview mt-3 min-h-0 flex-1 overflow-y-auto rounded-lg border bg-panel2/30 px-4 py-3 text-sm"
          dangerouslySetInnerHTML={{ __html: html }}
        />
        <div className="mt-4 flex items-center justify-end gap-2">
          <Button variant="outline" onClick={download}>Download .md</Button>
          <Button onClick={copy}>{copied ? "Copied!" : "Copy"}</Button>
        </div>
      </Card>
    </div>
  );
}
