import "server-only";
import data from "@/lib/usda-reference-data.json";
import type { FoodHit } from "@/lib/food-search";

const ALIASES: Record<string, string> = {
  "rolled oats": "oats regular quick dry", "quick oats": "oats regular quick dry",
  "powdered milk": "milk dry", "dry milk": "milk dry", "milk powder": "milk dry",
  evoo: "oil olive", "white sugar": "sugars granulated", "red pepper flakes": "pepper red cayenne",
  cavatappi: "pasta dry enriched", rotini: "pasta dry enriched", penne: "pasta dry enriched", fusilli: "pasta dry enriched", farfalle: "pasta dry enriched",
  "garlic powder": "spices garlic powder", "onion powder": "spices onion powder",
};
const STAPLES: Record<string, string> = { sugar: "sugars granulated", rice: "rice white long grain raw enriched" };
// Restaurant items and baby food are not pantry ingredients.
const HIDDEN = /^(?:Babyfood|Fast [Ff]oods?|Restaurant)\b|^[A-Z'’&. -]{4,},/;
const LESS_LIKELY = /\bcooked\b|\bprepared\b|(?<!not )\breconstituted\b|\bmade with\b|\bglutinous\b|\brestaurant\b|\bsteamed\b|\bbowl\b/i;
const words = (text: string) => (text.toLowerCase().match(/[a-z0-9]+/g) ?? []).map((w) => w.length > 3 && w.endsWith("s") ? w.slice(0, -1) : w);
type RecordFood = { id: number; description: string; density: number | null; kcal: number | null; source: string; tokens: string[]; first: string[]; segments: string[][] };
const foods: RecordFood[] = data.map((r) => ({ id: Number(r[0]), description: String(r[1]), density: r[2] == null ? null : Number(r[2]), kcal: r[3] == null ? null : Number(r[3]), source: String(r[5]), tokens: words(String(r[1])), first: words(String(r[1]).split(",")[0]), segments: String(r[1]).split(",").map(words) })).filter((f) => !HIDDEN.test(f.description));
const byId = new Map(foods.map((f) => [f.id, f]));
const hit = (f: RecordFood, name = f.description): FoodHit => ({ source: "usda", sourceId: String(f.id), name, reference: true, referenceName: f.description, kcalPer100g: f.kcal, densityGMl: f.density, defaultServingG: null, detail: null });
export function getReferenceFood(fdcId: number | string): FoodHit | null {
  const food = byId.get(Number(fdcId));
  return food ? hit(food) : null;
}
export function searchReferenceFoods(query: string, limit = 12, minMatchedShare = 0.5): FoodHit[] {
  const q = query.trim();
  if (q.length < 2) return [];
  // A bare staple means its plain form; a qualified one ("brown sugar") searches as typed.
  let expanded = STAPLES[q.toLowerCase()] ?? q.toLowerCase();
  let aliased = expanded !== q.toLowerCase();
  for (const [phrase, replacement] of Object.entries(ALIASES)) {
    const pattern = new RegExp(`\\b${phrase}\\b`, "g");
    if (pattern.test(expanded)) { expanded = expanded.replace(pattern, replacement); aliased = true; }
  }
  const tokens = words(expanded);
  if (!tokens.length) return [];
  const matches = (token: string, foodToken: string) => token === foodToken || (token.length >= 3 && foodToken.startsWith(token));
  return foods.map((food) => {
    const matched = tokens.filter((t) => food.tokens.some((w) => matches(t, w)));
    return { food, matched, rank: [
      tokens.length - matched.length,
      // A whole-word match ("butter") outranks a prefix ("butterbur").
      matched.filter((t) => !food.tokens.includes(t)).length,
      // USDA leads with the food ("Oil, olive, ..."); a match further along is a blend or side ingredient.
      matched.filter((t) => !food.segments.slice(0, 2).some((seg) => seg.some((w) => matches(t, w)))).length,
      food.first.filter((w) => !tokens.some((t) => matches(t, w))).length,
      LESS_LIKELY.test(food.description) ? 1 : 0,
      food.density != null ? 0 : 1,
      food.source === "sr" ? 0 : 1,
      food.tokens.length,
    ] };
  }).filter((r) => r.matched.length > 0 && r.matched.length >= tokens.length * minMatchedShare)
    .sort((a, b) => { for (let i = 0; i < a.rank.length; i++) if (a.rank[i] !== b.rank[i]) return a.rank[i] - b.rank[i]; return a.food.description.localeCompare(b.food.description, "en") || a.food.id - b.food.id; })
    .slice(0, limit)
    // What you typed is the friendlier name whenever it really describes this food.
    .map(({ food, matched }) => hit(food, (aliased ? matched.length === tokens.length : tokens.every((t) => food.tokens.includes(t))) ? q.charAt(0).toUpperCase() + q.slice(1) : food.description));
}
export function matchReferenceFoods(name: string, limit = 4): FoodHit[] {
  // Product names carry brand words USDA lacks, so one matching word is enough to suggest.
  return searchReferenceFoods(name, foods.length, 0).filter((f) => f.densityGMl != null).slice(0, limit);
}
