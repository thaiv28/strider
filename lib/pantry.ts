import { eq, asc } from "drizzle-orm";
import { db, schema } from "@/src/db/index";
import { kcalForGrams, num } from "@/lib/food";

export type IngredientRow = {
  id: number;
  name: string;
  kcalPer100g: number | null;
  densityGMl: number | null;
  defaultServingG: number | null;
  category: string | null;
  source: string | null;
  notes: string | null;
};

export type MealIngredientRow = {
  id: number;
  ingredientId: number | null;
  name: string;
  kcalPer100g: number | null;
  densityGMl: number | null;
  amountG: number; // whole recipe at base servings
  sortOrder: number;
};

export type MealRow = {
  id: number;
  name: string;
  baseServings: number;
  mealType: string | null;
  isHot: boolean;
  waterMl: number | null;
  notes: string | null;
  items: MealIngredientRow[];
  totalG: number; // whole recipe
  totalKcal: number; // whole recipe
};

export async function getIngredients(userId: number): Promise<IngredientRow[]> {
  const rows = await db
    .select()
    .from(schema.ingredient)
    .where(eq(schema.ingredient.userId, userId))
    .orderBy(asc(schema.ingredient.name));
  return rows.map((r) => ({
    id: r.id,
    name: r.name,
    kcalPer100g: num(r.kcalPer100g),
    densityGMl: num(r.densityGMl),
    defaultServingG: r.defaultServingG,
    category: r.category,
    source: r.source,
    notes: r.notes,
  }));
}

export async function getMeals(userId: number): Promise<MealRow[]> {
  const meals = await db
    .select()
    .from(schema.meal)
    .where(eq(schema.meal.userId, userId))
    .orderBy(asc(schema.meal.name));
  const items = await db
    .select({
      id: schema.mealIngredient.id,
      mealId: schema.mealIngredient.mealId,
      ingredientId: schema.mealIngredient.ingredientId,
      snapshotName: schema.mealIngredient.snapshotName,
      snapshotKcalPer100g: schema.mealIngredient.snapshotKcalPer100g,
      snapshotDensityGMl: schema.mealIngredient.snapshotDensityGMl,
      amountG: schema.mealIngredient.amountG,
      sortOrder: schema.mealIngredient.sortOrder,
    })
    .from(schema.mealIngredient)
    .innerJoin(schema.meal, eq(schema.mealIngredient.mealId, schema.meal.id))
    .where(eq(schema.meal.userId, userId))
    .orderBy(asc(schema.mealIngredient.sortOrder), asc(schema.mealIngredient.id));

  const byMeal = new Map<number, MealIngredientRow[]>();
  for (const it of items) {
    const row: MealIngredientRow = {
      id: it.id,
      ingredientId: it.ingredientId,
      name: it.snapshotName,
      kcalPer100g: num(it.snapshotKcalPer100g),
      densityGMl: num(it.snapshotDensityGMl),
      amountG: it.amountG,
      sortOrder: it.sortOrder,
    };
    if (!byMeal.has(it.mealId)) byMeal.set(it.mealId, []);
    byMeal.get(it.mealId)!.push(row);
  }

  return meals.map((m) => {
    const its = byMeal.get(m.id) ?? [];
    return {
      id: m.id,
      name: m.name,
      baseServings: m.baseServings,
      mealType: m.mealType,
      isHot: m.isHot,
      waterMl: m.waterMl,
      notes: m.notes,
      items: its,
      totalG: its.reduce((s, i) => s + i.amountG, 0),
      totalKcal: its.reduce((s, i) => s + kcalForGrams(i.kcalPer100g, i.amountG), 0),
    };
  });
}
