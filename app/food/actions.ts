"use server";

import { revalidatePath } from "next/cache";
import { eq, max } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { searchFoods, fetchFdcDensity, type FoodHit } from "@/lib/food-search";
import { upsertIngredientFromHit } from "@/lib/food-server";
import {
  requireOwnedIngredient,
  requireOwnedMeal,
  requireOwnedMealIngredient,
} from "@/lib/authorization";

const s = (v: number | null | undefined) => (v == null ? null : String(v));
const sRound = (v: number | null | undefined) => (v == null ? null : String(Math.round(v)));

function bump() {
  revalidatePath("/food");
}

// ---- search (called from the client to populate the add-ingredient dialog) ----
export async function searchNutrition(query: string): Promise<FoodHit[]> {
  await getCurrentUserId();
  return searchFoods(query);
}

// ---- ingredients ----
export type IngredientInput = {
  name: string;
  kcalPer100g: number | null;
  densityGMl: number | null;
  defaultServingG: number | null;
  category: string | null;
  source: string | null;
  sourceId: string | null;
  notes: string | null;
};

export async function createIngredient(v: IngredientInput) {
  const userId = await getCurrentUserId();
  if (!v.name.trim()) throw new Error("Name is required");
  // For a USDA pick with no density yet, try the detail endpoint once.
  let density = v.densityGMl;
  if (density == null && v.source === "usda" && v.sourceId) density = await fetchFdcDensity(v.sourceId);
  await db.insert(schema.ingredient).values({
    userId,
    name: v.name.trim(),
    kcalPer100g: sRound(v.kcalPer100g),
    densityGMl: s(density),
    defaultServingG: v.defaultServingG,
    category: v.category,
    source: v.source ?? "manual",
    sourceId: v.sourceId,
    notes: v.notes,
  });
  bump();
}

export async function updateIngredient(id: number, v: IngredientInput) {
  await requireOwnedIngredient(id);
  await db
    .update(schema.ingredient)
    .set({
      name: v.name.trim(),
      kcalPer100g: sRound(v.kcalPer100g),
      densityGMl: s(v.densityGMl),
      defaultServingG: v.defaultServingG,
      category: v.category,
      notes: v.notes,
    })
    .where(eq(schema.ingredient.id, id));
  bump();
}

export async function deleteIngredient(id: number) {
  await requireOwnedIngredient(id);
  await db.delete(schema.ingredient).where(eq(schema.ingredient.id, id));
  bump();
}

// ---- meals ----
export async function createMeal(name: string): Promise<number> {
  const userId = await getCurrentUserId();
  const [row] = await db
    .insert(schema.meal)
    .values({ userId, name: name.trim() || "New meal" })
    .returning({ id: schema.meal.id });
  bump();
  return row.id;
}

export async function updateMeal(
  id: number,
  patch: { name?: string; baseServings?: number; mealType?: string | null; isHot?: boolean; waterMl?: number | null; notes?: string | null },
) {
  await requireOwnedMeal(id);
  const set: Record<string, unknown> = {};
  if (patch.name != null) set.name = patch.name.trim() || "Untitled";
  if (patch.baseServings != null) set.baseServings = Math.max(1, Math.round(patch.baseServings));
  if ("mealType" in patch) set.mealType = patch.mealType || null;
  if ("isHot" in patch) set.isHot = patch.isHot;
  if ("waterMl" in patch) set.waterMl = patch.waterMl;
  if ("notes" in patch) set.notes = patch.notes;
  if (Object.keys(set).length) await db.update(schema.meal).set(set).where(eq(schema.meal.id, id));
  bump();
}

export async function deleteMeal(id: number) {
  await requireOwnedMeal(id);
  await db.delete(schema.meal).where(eq(schema.meal.id, id));
  bump();
}

export async function addIngredientFromHit(hit: FoodHit) {
  await upsertIngredientFromHit(hit);
  bump();
}

async function insertMealLine(
  mealId: number,
  item: { ingredientId?: number | null; name: string; kcalPer100g: number | null; densityGMl: number | null; amountG: number },
) {
  const [{ m } = { m: -1 }] = await db
    .select({ m: max(schema.mealIngredient.sortOrder) })
    .from(schema.mealIngredient)
    .where(eq(schema.mealIngredient.mealId, mealId));
  await db.insert(schema.mealIngredient).values({
    mealId,
    ingredientId: item.ingredientId ?? null,
    snapshotName: item.name,
    snapshotKcalPer100g: sRound(item.kcalPer100g),
    snapshotDensityGMl: s(item.densityGMl),
    amountG: Math.max(0, Math.round(item.amountG)),
    sortOrder: (m ?? -1) + 1,
  });
}

// Add a pantry ingredient (already saved) to a meal.
export async function addMealIngredient(mealId: number, ingredientId: number, amountG: number) {
  const userId = await requireOwnedMeal(mealId);
  await requireOwnedIngredient(ingredientId, userId);
  const [ing] = await db.select().from(schema.ingredient).where(eq(schema.ingredient.id, ingredientId));
  if (!ing) throw new Error("Ingredient not found");
  await insertMealLine(mealId, { ingredientId, name: ing.name, kcalPer100g: Number(ing.kcalPer100g) || null, densityGMl: ing.densityGMl != null ? Number(ing.densityGMl) : null, amountG });
  bump();
}

// Add a public hit to a meal — saves it to the pantry first, then links it.
export async function addMealIngredientFromHit(mealId: number, hit: FoodHit, amountG?: number) {
  await requireOwnedMeal(mealId);
  const ing = await upsertIngredientFromHit(hit);
  await insertMealLine(mealId, {
    ingredientId: ing.id,
    name: ing.name,
    kcalPer100g: Number(ing.kcalPer100g) || null,
    densityGMl: ing.densityGMl != null ? Number(ing.densityGMl) : null,
    amountG: amountG ?? ing.defaultServingG ?? 50,
  });
  bump();
}

export async function updateMealIngredientAmount(id: number, amountG: number) {
  await requireOwnedMealIngredient(id);
  await db
    .update(schema.mealIngredient)
    .set({ amountG: Math.max(0, Math.round(amountG)) })
    .where(eq(schema.mealIngredient.id, id));
  bump();
}

export async function removeMealIngredient(id: number) {
  await requireOwnedMealIngredient(id);
  await db.delete(schema.mealIngredient).where(eq(schema.mealIngredient.id, id));
  bump();
}
