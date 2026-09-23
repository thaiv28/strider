import { and, eq, inArray } from "drizzle-orm";
import { notFound as nextNotFound } from "next/navigation";
import { db, schema } from "@/src/db/index";
import { getCurrentUserId } from "@/lib/gear";

function notFound(resource: string): never {
  void resource;
  nextNotFound();
}

export async function requireOwnedTrip(tripId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.trip.id })
    .from(schema.trip)
    .where(and(eq(schema.trip.id, tripId), eq(schema.trip.userId, userId)));
  if (!row) notFound("Trip");
  return userId;
}

export async function tripAccess(tripId: number, userId: number) {
  const [row] = await db.select({ ownerId: schema.trip.userId, editToken: schema.trip.editToken })
    .from(schema.trip).where(eq(schema.trip.id, tripId));
  if (!row) return null;
  if (row.ownerId === userId) return { ownerId: row.ownerId, isOwner: true };
  if (!row.editToken) return null;
  const [visit] = await db.select({ tripId: schema.tripShareVisit.tripId })
    .from(schema.tripShareVisit)
    .where(and(eq(schema.tripShareVisit.tripId, tripId), eq(schema.tripShareVisit.userId, userId), eq(schema.tripShareVisit.permission, "edit")));
  return visit ? { ownerId: row.ownerId, isOwner: false } : null;
}

export async function requireTripEditor(tripId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  if (!await tripAccess(tripId, userId)) notFound("Trip");
  return userId;
}

export async function requireOwnedGearItem(itemId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.gearItem.id })
    .from(schema.gearItem)
    .where(and(eq(schema.gearItem.id, itemId), eq(schema.gearItem.userId, userId)));
  if (!row) notFound("Gear item");
  return userId;
}

export async function requireOwnedGearItems(itemIds: number[], userId?: number) {
  userId ??= await getCurrentUserId();
  const ids = [...new Set(itemIds)];
  if (!ids.length) return userId;
  const rows = await db
    .select({ id: schema.gearItem.id })
    .from(schema.gearItem)
    .where(and(inArray(schema.gearItem.id, ids), eq(schema.gearItem.userId, userId)));
  if (rows.length !== ids.length) notFound("Gear item");
  return userId;
}

export async function requireOwnedCategory(categoryId: number | null, userId?: number) {
  userId ??= await getCurrentUserId();
  if (categoryId == null) return userId;
  const [row] = await db
    .select({ id: schema.gearCategory.id })
    .from(schema.gearCategory)
    .where(and(eq(schema.gearCategory.id, categoryId), eq(schema.gearCategory.userId, userId)));
  if (!row) notFound("Gear category");
  return userId;
}

export async function requireOwnedLoadout(loadoutId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.loadout.id })
    .from(schema.loadout)
    .where(and(eq(schema.loadout.id, loadoutId), eq(schema.loadout.userId, userId)));
  if (!row) notFound("Loadout");
  return userId;
}

export async function requireOwnedIngredient(ingredientId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.ingredient.id })
    .from(schema.ingredient)
    .where(and(eq(schema.ingredient.id, ingredientId), eq(schema.ingredient.userId, userId)));
  if (!row) notFound("Ingredient");
  return userId;
}

export async function requireOwnedMeal(mealId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.meal.id })
    .from(schema.meal)
    .where(and(eq(schema.meal.id, mealId), eq(schema.meal.userId, userId)));
  if (!row) notFound("Meal");
  return userId;
}

export async function requireOwnedMealIngredient(lineId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.mealIngredient.id })
    .from(schema.mealIngredient)
    .innerJoin(schema.meal, eq(schema.mealIngredient.mealId, schema.meal.id))
    .where(and(eq(schema.mealIngredient.id, lineId), eq(schema.meal.userId, userId)));
  if (!row) notFound("Meal ingredient");
  return userId;
}

export async function requireOwnedTripChild(
  kind: "gear" | "day" | "campsite" | "meal",
  childId: number,
  tripId: number,
  userId?: number,
) {
  userId ??= await getCurrentUserId();
  await requireTripEditor(tripId, userId);
  const row = kind === "gear"
    ? (await db.select({ id: schema.tripGear.id }).from(schema.tripGear).where(and(eq(schema.tripGear.id, childId), eq(schema.tripGear.tripId, tripId))))[0]
    : kind === "day"
      ? (await db.select({ id: schema.tripDay.id }).from(schema.tripDay).where(and(eq(schema.tripDay.id, childId), eq(schema.tripDay.tripId, tripId))))[0]
      : kind === "campsite"
        ? (await db.select({ id: schema.tripCampsite.id }).from(schema.tripCampsite).where(and(eq(schema.tripCampsite.id, childId), eq(schema.tripCampsite.tripId, tripId))))[0]
        : (await db.select({ id: schema.tripMeal.id }).from(schema.tripMeal).where(and(eq(schema.tripMeal.id, childId), eq(schema.tripMeal.tripId, tripId))))[0];
  if (!row) notFound("Trip item");
  return userId;
}

export async function requireOwnedCalendarFeed(feedId: number, userId?: number) {
  userId ??= await getCurrentUserId();
  const [row] = await db
    .select({ id: schema.calendarFeed.id })
    .from(schema.calendarFeed)
    .where(and(eq(schema.calendarFeed.id, feedId), eq(schema.calendarFeed.userId, userId)));
  if (!row) notFound("Calendar feed");
  return userId;
}
