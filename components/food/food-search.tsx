"use client";
import { Children, useEffect, useRef, useState, type ReactNode, type KeyboardEvent } from "react";
import { searchNutrition, searchPackagedFoods } from "@/app/food/actions";
import type { FoodHit } from "@/lib/food-search";
import { Badge } from "@/components/ui";

export const FOOD_SEARCH_PLACEHOLDER = "Search foods, e.g. olive oil, rolled oats";
export function useFoodSearch(query: string) {
  const [hits, setHits] = useState<FoodHit[]>([]);
  const [packaged, setPackaged] = useState<FoodHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [packageStatus, setPackageStatus] = useState("");
  const generation = useRef(0);
  const loadPackaged = async (q = query, version = generation.current) => {
    setPackageStatus("Searching packaged products…");
    try {
      const rows = await searchPackagedFoods(q);
      if (generation.current !== version) return;
      setPackaged(rows);
      setPackageStatus(rows.length ? "" : "No packaged products found.");
    } catch { if (generation.current === version) { setPackaged([]); setPackageStatus("Packaged search is unavailable."); } }
  };
  useEffect(() => {
    const version = ++generation.current;
    setPackageStatus("");
    if (query.trim().length < 2) { setHits([]); setPackaged([]); setSearching(false); return; }
    setSearching(true);
    const timer = setTimeout(async () => {
      try {
        const rows = await searchNutrition(query);
        if (generation.current !== version) return;
        setHits(rows);
        if (!rows.length) void loadPackaged(query, version);
        else setPackaged([]);
      } catch { if (generation.current === version) setHits([]); }
      finally { if (generation.current === version) setSearching(false); }
    }, 150);
    return () => { clearTimeout(timer); generation.current++; };
  }, [query]);
  return { hits, packaged, searching, packageStatus, loadPackaged: () => loadPackaged() };
}
export function foodSearchKeys(event: KeyboardEvent<HTMLInputElement>, clear: () => void) {
  if (event.key === "Escape") { event.preventDefault(); clear(); }
  if (event.key === "Enter") {
    const first = event.currentTarget.parentElement?.querySelector<HTMLButtonElement>("[data-food-results] button");
    if (first) { event.preventDefault(); first.click(); }
  }
}
export function FoodSearchResults({ query, search, onPick, children }: { query: string; search: ReturnType<typeof useFoodSearch>; onPick: (hit: FoodHit) => void; children?: ReactNode }) {
  if (query.trim().length < 1) return null;
  const row = (h: FoodHit, packaged = false) => <button type="button" data-reference-id={h.reference ? h.sourceId : undefined} key={`${h.source}-${h.sourceId}`} onClick={() => onPick(h)} className="flex w-full items-center justify-between gap-2 border-b px-3 py-2 text-left hover:bg-panel2/60">
    <span className="min-w-0 flex-1"><span className="block break-words text-sm">{h.referenceName ?? h.name}</span><span className="block break-words text-xs text-muted">{packaged ? h.detail : h.densityGMl == null ? "weight only" : `1 tbsp = ${(h.densityGMl * 14.7868).toFixed(1)} g`}</span></span>
    <span className="readout shrink-0 text-right text-xs text-muted">{h.kcalPer100g ?? "—"} kcal/100g{packaged && <span className="block"><Badge>packaged</Badge></span>}</span>
  </button>;
  return <div data-food-results className="mt-2 max-h-64 overflow-y-auto rounded-lg border">
    {children}
    {search.searching && !search.hits.length && !search.packaged.length && !Children.count(children) && <div className="px-3 py-2 text-sm text-muted">Searching…</div>}
    {search.hits.map((h) => row(h))}
    <button type="button" className="w-full px-3 py-2 text-left text-sm text-muted hover:bg-panel2/60 break-words" onClick={search.loadPackaged}>Search packaged products for “{query}”</button>
    {search.packaged.map((h) => row(h, true))}
    {search.packageStatus && <div className="px-3 py-2 text-xs text-muted">{search.packageStatus}</div>}
  </div>;
}
