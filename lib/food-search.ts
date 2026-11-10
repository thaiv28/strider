import { VOLUME_ML, parseVolumeUnit, densityFromServing } from "@/lib/food";

export type FoodHit = {
  reference?: boolean;
  referenceName?: string | null;
  source: "usda" | "off";
  sourceId: string;
  name: string;
  detail: string | null; // brand / data type, shown as a sublabel
  kcalPer100g: number | null;
  defaultServingG: number | null;
  densityGMl: number | null;
};

const FDC = "https://api.nal.usda.gov/fdc/v1";

function fdcEnergy(nutrients: { nutrientName?: string; unitName?: string; value?: number }[]): number | null {
  const kcal = nutrients.filter((n) => n.unitName?.toUpperCase() === "KCAL" && n.value != null);
  if (!kcal.length) return null;
  // Prefer a plain "Energy" row over Atwater variants when both are present.
  const plain = kcal.find((n) => n.nutrientName === "Energy");
  const v = (plain ?? kcal[0]).value;
  return v == null ? null : Math.round(v);
}

// Map an FDC portion to grams-per-mL when its measure is a volume we know.
function fdcDensity(portions: { amount?: number; gramWeight?: number; modifier?: string; measureUnit?: { name?: string; abbreviation?: string } }[]): number | null {
  for (const p of portions) {
    if (!p.gramWeight || !p.amount) continue;
    for (const label of [p.measureUnit?.name, p.measureUnit?.abbreviation, p.modifier]) {
      const unit = parseVolumeUnit(label ?? "");
      if (!unit) continue;
      const density = +(p.gramWeight / (p.amount * VOLUME_ML[unit])).toFixed(3);
      if (density >= 0.02 && density <= 2.5) return density;
    }
  }
  return null;
}

async function searchFdc(query: string): Promise<FoodHit[]> {
  const key = process.env.FDC_API_KEY;
  if (!key) return [];
  const url = `${FDC}/foods/search?query=${encodeURIComponent(query)}&pageSize=12&api_key=${key}`;
  const r = await fetch(url);
  if (!r.ok) return [];
  const j = await r.json();
  return (j.foods ?? []).map((f: Record<string, unknown>): FoodHit => {
    const grams = f.servingSize && /^(g|grm)$/i.test(String(f.servingSizeUnit)) ? Number(f.servingSize) : null;
    const serving = grams == null ? null : Math.round(grams);
    return {
      source: "usda",
      sourceId: String(f.fdcId),
      name: String(f.description ?? "").replace(/\s+/g, " ").trim(),
      detail: [f.brandOwner, f.dataType].filter(Boolean).join(" · ") || null,
      kcalPer100g: fdcEnergy((f.foodNutrients as never) ?? []),
      defaultServingG: serving,
      densityGMl: densityFromServing(f.householdServingFullText as string | undefined, grams),
    };
  });
}

async function searchOff(query: string): Promise<FoodHit[]> {
  // Search-a-licious full-text endpoint; the legacy cgi/search.pl is throttled (503).
  const url =
    `https://search.openfoodfacts.org/search?q=${encodeURIComponent(query)}&page_size=10` +
    `&fields=code,product_name,brands,nutriments,serving_quantity,serving_size`;
  const r = await fetch(url, { headers: { "User-Agent": "backpack-app/1.0 (personal trip planner)" } });
  if (!r.ok) return [];
  const j = await r.json();
  return (j.hits ?? [])
    .filter((p: Record<string, unknown>) => p.product_name && (p.nutriments as Record<string, unknown>)?.["energy-kcal_100g"] != null)
    .map((p: Record<string, unknown>): FoodHit => {
      const nut = p.nutriments as Record<string, number>;
      const sq = Number(p.serving_quantity);
      const brands = Array.isArray(p.brands) ? p.brands.join(", ") : (p.brands as string);
      return {
        source: "off",
        sourceId: String(p.code),
        name: String(p.product_name).trim(),
        detail: brands?.trim() || "Open Food Facts",
        kcalPer100g: Math.round(nut["energy-kcal_100g"]),
        defaultServingG: Number.isFinite(sq) && sq > 0 ? Math.round(sq) : null,
        densityGMl: densityFromServing(p.serving_size as string | undefined),
      };
    });
}

export async function searchFoods(query: string): Promise<FoodHit[]> {
  const q = query.trim();
  if (q.length < 2) return [];
  const [fdc, off] = await Promise.all([searchFdc(q), searchOff(q)]);
  return [...fdc.filter((h) => h.kcalPer100g != null), ...off].slice(0, 16);
}

// Density needs the FDC detail endpoint (search results omit foodPortions).
export async function fetchFdcDensity(fdcId: string): Promise<number | null> {
  const key = process.env.FDC_API_KEY;
  if (!key) return null;
  const r = await fetch(`${FDC}/food/${fdcId}?api_key=${key}`);
  if (!r.ok) return null;
  const j = await r.json();
  const grams = /^(g|grm)$/i.test(String(j.servingSizeUnit)) ? Number(j.servingSize) : null;
  return fdcDensity(j.foodPortions ?? []) ?? densityFromServing(j.householdServingFullText, grams);
}
