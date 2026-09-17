import { and, eq } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { fetchFdcDensity, type FoodHit } from "@/lib/food-search";
import { kcalForGrams } from "@/lib/food";

const sRound = (v: number | null | undefined) => (v == null ? null : String(Math.round(v)));
const s = (v: number | null | undefined) => (v == null ? null : String(v));

// Save a public (FDC/OFF) hit to the pantry, deduped by (source, sourceId).
// USDA density is fetched once. Returns the pantry row.
export async function upsertIngredientFromHit(hit: FoodHit) {
  const userId = await getCurrentUserId();
  const [existing] = await db
    .select()
    .from(schema.ingredient)
    .where(and(eq(schema.ingredient.userId, userId), eq(schema.ingredient.source, hit.source), eq(schema.ingredient.sourceId, hit.sourceId)))
    .limit(1);
  if (existing) return existing;
  let density = hit.densityGMl;
  if (density == null && hit.source === "usda") density = await fetchFdcDensity(hit.sourceId);
  const [row] = await db
    .insert(schema.ingredient)
    .values({
      userId,
      name: hit.name,
      kcalPer100g: sRound(hit.kcalPer100g),
      densityGMl: s(density),
      defaultServingG: hit.defaultServingG,
      source: hit.source,
      sourceId: hit.sourceId,
    })
    .returning();
  return row;
}

// Total kcal/weight of a meal at its base servings, plus name & base servings.
export async function computeMealTotals(mealId: number): Promise<{ name: string; kcal: number; g: number; baseServings: number }> {
  const [m] = await db.select().from(schema.meal).where(eq(schema.meal.id, mealId));
  const items = await db.select().from(schema.mealIngredient).where(eq(schema.mealIngredient.mealId, mealId));
  const kcal = items.reduce((sum, it) => sum + kcalForGrams(it.snapshotKcalPer100g != null ? Number(it.snapshotKcalPer100g) : null, it.amountG), 0);
  const g = items.reduce((sum, it) => sum + it.amountG, 0);
  return { name: m?.name ?? "Meal", kcal, g, baseServings: m?.baseServings || 1 };
}
