import "server-only";

import { sql } from "drizzle-orm";
import { db } from "@/src/db/index";
import { VOLUME_ML } from "@/lib/food";

export type Tx = Parameters<Parameters<typeof db.transaction>[0]>[0];

async function syncFoodWeightForTrips(tx: Tx, tripIds: number[]) {
  if (!tripIds.length) return;
  await tx.execute(sql`
    UPDATE trip t
    SET food_g_per_day = ROUND(s.g::numeric / GREATEST(COALESCE(t.nights, 0) + 1, 1))
    FROM (
      SELECT trip_id, SUM(snapshot_weight_g) AS g FROM trip_meal
      WHERE trip_id IN (${sql.join(tripIds.map((id) => sql`${id}`), sql`, `)})
      GROUP BY trip_id
    ) s WHERE t.id = s.trip_id
  `);
}

export async function propagateGearItem(tx: Tx, gearItemId: number) {
  await tx.execute(sql`
    UPDATE trip_gear tg
    SET snapshot_name = gi.name, snapshot_weight_g = COALESCE(gi.weight_g, tg.snapshot_weight_g)
    FROM gear_item gi WHERE gi.id = ${gearItemId} AND tg.gear_item_id = gi.id
  `);
}

export async function propagateMeal(tx: Tx, mealId: number) {
  await tx.execute(sql`
    UPDATE trip_meal tm SET snapshot_name = m.name
    FROM meal m WHERE m.id = ${mealId} AND tm.meal_id = m.id
  `);
  // The inner join preserves imported totals and recipes whose last line was removed.
  const changed = await tx.execute<{ trip_id: number }>(sql`
    WITH totals AS (
      SELECT m.id, GREATEST(m.base_servings, 1) AS base, SUM(mi.amount_g) AS g,
        SUM(ROUND(COALESCE(mi.snapshot_kcal_per_100g, 0) / 100 * mi.amount_g)) AS kcal
      FROM meal m JOIN meal_ingredient mi ON mi.meal_id = m.id
      WHERE m.id = ${mealId} GROUP BY m.id, m.base_servings
    )
    UPDATE trip_meal tm SET snapshot_kcal = ROUND(t.kcal * tm.servings / t.base),
      snapshot_weight_g = ROUND(t.g * tm.servings / t.base)
    FROM totals t WHERE tm.meal_id = t.id
      AND (tm.snapshot_kcal, tm.snapshot_weight_g) IS DISTINCT FROM
        (ROUND(t.kcal * tm.servings / t.base)::int, ROUND(t.g * tm.servings / t.base)::int)
    RETURNING tm.trip_id
  `);
  await syncFoodWeightForTrips(tx, [...new Set(changed.rows.map((row) => row.trip_id))]);
}

export async function propagateIngredient(tx: Tx, ingredientId: number) {
  // Only trusted unit constants enter raw SQL; user-supplied values remain parameters.
  const volumeUnits = sql.raw(Object.keys(VOLUME_ML).map((unit) => `'${unit}'`).join(", "));
  const volumeCase = sql.raw(Object.entries(VOLUME_ML).map(([unit, ml]) => `WHEN '${unit}' THEN ${ml}`).join(" "));
  const changed = await tx.execute<{ meal_id: number }>(sql`
    UPDATE meal_ingredient mi
    SET snapshot_name = i.name,
      snapshot_kcal_per_100g = COALESCE(ROUND(i.kcal_per_100g), mi.snapshot_kcal_per_100g),
      snapshot_density_g_ml = i.density_g_ml,
      amount_g = CASE WHEN i.density_g_ml IS NOT NULL AND mi.entered_amount IS NOT NULL
        AND mi.entered_unit IN (${volumeUnits})
        THEN ROUND(mi.entered_amount * CASE mi.entered_unit ${volumeCase} END * i.density_g_ml)::int
        ELSE mi.amount_g END
    FROM ingredient i WHERE i.id = ${ingredientId} AND mi.ingredient_id = i.id
    RETURNING mi.meal_id
  `);
  for (const mealId of new Set(changed.rows.map((row) => row.meal_id))) await propagateMeal(tx, mealId);
  await tx.execute(sql`
    UPDATE trip_meal tm SET snapshot_name = i.name,
      snapshot_kcal = CASE WHEN i.kcal_per_100g IS NULL THEN tm.snapshot_kcal
        ELSE ROUND(i.kcal_per_100g / 100 * tm.snapshot_weight_g)::int END
    FROM ingredient i WHERE i.id = ${ingredientId} AND tm.ingredient_id = i.id AND tm.meal_id IS NULL
  `);
}
