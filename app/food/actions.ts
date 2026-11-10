"use server";

import { revalidatePath } from "next/cache";
import { eq, max } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";
import { searchFoods, fetchFdcDensity, type FoodHit } from "@/lib/food-search";
import { searchReferenceFoods, matchReferenceFoods, getReferenceFood } from "@/lib/usda-reference";
import { upsertIngredientFromHit } from "@/lib/food-server";
import { isAmountUnit, VOLUME_ML } from "@/lib/food";
import { propagateIngredient, propagateMeal, type Tx } from "@/lib/trip-propagation";
import {
  requireOwnedIngredient,
  requireOwnedMeal,
  requireOwnedMealIngredient,
} from "@/lib/authorization";

const s = (v: number | null | undefined) => (v == null ? null : String(v));
const sRound = (v: number | null | undefined) => (v == null ? null : String(Math.round(v)));

function bump() {
  revalidatePath("/", "layout");
}

// ---- search (called from the client to populate the add-ingredient dialog) ----
export async function searchNutrition(query: string): Promise<FoodHit[]> {
  await getCurrentUserId();
  return searchReferenceFoods(query);
}

export async function searchPackagedFoods(query: string): Promise<FoodHit[]> {
  await getCurrentUserId();
  return searchFoods(query);
}
export async function suggestReferenceFoods(name: string): Promise<FoodHit[]> {
  await getCurrentUserId();
  return matchReferenceFoods(name).filter((f) => f.densityGMl != null);
}
export async function linkIngredientDensity(ingredientId: number, fdcId: string) {
  await requireOwnedIngredient(ingredientId);
  const food = getReferenceFood(fdcId);
  if (!food || food.densityGMl == null) throw new Error("Reference food with density required");
  await db.transaction(async (tx) => {
    await tx.update(schema.ingredient).set({ densityGMl: s(food.densityGMl), densitySource: "usda", referenceName: food.referenceName }).where(eq(schema.ingredient.id, ingredientId));
    await propagateIngredient(tx, ingredientId);
  });
  bump();
}

// ---- ingredients ----
export type IngredientInput = {
  name: string;
  kcalPer100g: number | null;
  densityGMl: number | null;
  densitySource: "usda" | "label" | "manual" | null;
  referenceName: string | null;
  defaultServingG: number | null;
  category: string | null;
  source: string | null;
  sourceId: string | null;
  notes: string | null;
};

export async function createIngredient(v: IngredientInput) {
  const userId = await getCurrentUserId();
  if (!v.name.trim()) throw new Error("Name is required");
  // Only packaged USDA hits may need a detail lookup; bundled references are complete.
  let density = v.densityGMl;
  if (density == null && v.source === "usda" && v.sourceId && v.densitySource !== "usda" && !v.referenceName) density = await fetchFdcDensity(v.sourceId);
  await db.insert(schema.ingredient).values({
    userId,
    name: v.name.trim(),
    kcalPer100g: sRound(v.kcalPer100g),
    densityGMl: s(density),
    densitySource: v.densitySource,
    referenceName: v.referenceName,
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
  await db.transaction(async (tx) => {
    await tx
      .update(schema.ingredient)
      .set({
        name: v.name.trim(),
        kcalPer100g: sRound(v.kcalPer100g),
        densityGMl: s(v.densityGMl),
        densitySource: v.densitySource,
        referenceName: v.referenceName,
        source: v.source,
        sourceId: v.sourceId,
        defaultServingG: v.defaultServingG,
        category: v.category,
        notes: v.notes,
      })
      .where(eq(schema.ingredient.id, id));
    await propagateIngredient(tx, id);
  });
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
  if (Object.keys(set).length) await db.transaction(async (tx) => {
    await tx.update(schema.meal).set(set).where(eq(schema.meal.id, id));
    await propagateMeal(tx, id);
  });
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
  tx: Tx,
  mealId: number,
  item: { ingredientId?: number | null; name: string; kcalPer100g: number | null; densityGMl: number | null; amountG: number },
) {
  const [{ m } = { m: -1 }] = await tx
    .select({ m: max(schema.mealIngredient.sortOrder) })
    .from(schema.mealIngredient)
    .where(eq(schema.mealIngredient.mealId, mealId));
  await tx.insert(schema.mealIngredient).values({
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
  await db.transaction(async (tx) => {
    await insertMealLine(tx, mealId, { ingredientId, name: ing.name, kcalPer100g: ing.kcalPer100g != null ? Number(ing.kcalPer100g) : null, densityGMl: ing.densityGMl != null ? Number(ing.densityGMl) : null, amountG });
    await propagateMeal(tx, mealId);
  });
  bump();
}

// Add a public hit to a meal — saves it to the pantry first, then links it.
export async function addMealIngredientFromHit(mealId: number, hit: FoodHit, amountG?: number) {
  await requireOwnedMeal(mealId);
  const ing = await upsertIngredientFromHit(hit);
  await db.transaction(async (tx) => {
    await insertMealLine(tx, mealId, {
      ingredientId: ing.id,
      name: ing.name,
      kcalPer100g: ing.kcalPer100g != null ? Number(ing.kcalPer100g) : null,
      densityGMl: ing.densityGMl != null ? Number(ing.densityGMl) : null,
      amountG: amountG ?? ing.defaultServingG ?? 50,
    });
    await propagateMeal(tx, mealId);
  });
  bump();
}

export async function updateMealIngredientAmount(
  id: number,
  amountG: number,
  entered?: { amount: number; unit: string } | null,
) {
  await requireOwnedMealIngredient(id);
  const valid = entered != null && isAmountUnit(entered.unit) && Number.isFinite(entered.amount) && entered.amount >= 0;
  await db.transaction(async (tx) => {
    const [line] = await tx.select({ mealId: schema.mealIngredient.mealId }).from(schema.mealIngredient).where(eq(schema.mealIngredient.id, id));
    if (!line) throw new Error("Meal ingredient not found");
    await tx.update(schema.mealIngredient).set({
      amountG: Math.max(0, Math.round(amountG)),
      enteredAmount: valid ? String(entered.amount) : null,
      enteredUnit: valid ? entered.unit : null,
    }).where(eq(schema.mealIngredient.id, id));
    await propagateMeal(tx, line.mealId);
  });
  bump();
}

export async function removeMealIngredient(id: number) {
  await requireOwnedMealIngredient(id);
  await db.transaction(async (tx) => {
    const [line] = await tx.select({ mealId: schema.mealIngredient.mealId }).from(schema.mealIngredient).where(eq(schema.mealIngredient.id, id));
    if (!line) throw new Error("Meal ingredient not found");
    await tx.delete(schema.mealIngredient).where(eq(schema.mealIngredient.id, id));
    await propagateMeal(tx, line.mealId);
  });
  bump();
}

export async function setIngredientMeasure(ingredientId: number, unit: string, grams: number) {
  await requireOwnedIngredient(ingredientId);
  if (!Object.hasOwn(VOLUME_ML, unit) || !Number.isFinite(grams) || grams <= 0) throw new Error("A volume unit and positive weight are required");
  const density = +(grams / VOLUME_ML[unit as keyof typeof VOLUME_ML]).toFixed(3);
  await db.transaction(async (tx) => {
    await tx.update(schema.ingredient).set({ densityGMl: String(density), densitySource: "manual", referenceName: null }).where(eq(schema.ingredient.id, ingredientId));
    await propagateIngredient(tx, ingredientId);
  });
  bump();
}
