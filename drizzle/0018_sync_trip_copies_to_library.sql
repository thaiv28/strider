-- This intentionally rewrites historical trip copies; the daily backup is the only way back.
UPDATE trip_gear tg
SET snapshot_name = gi.name,
    snapshot_weight_g = COALESCE(gi.weight_g, tg.snapshot_weight_g)
FROM gear_item gi
WHERE tg.gear_item_id = gi.id
  AND (tg.snapshot_name, tg.snapshot_weight_g) IS DISTINCT FROM
      (gi.name, COALESCE(gi.weight_g, tg.snapshot_weight_g));
--> statement-breakpoint
UPDATE meal_ingredient mi
SET snapshot_name = i.name,
    snapshot_kcal_per_100g = COALESCE(ROUND(i.kcal_per_100g), mi.snapshot_kcal_per_100g),
    snapshot_density_g_ml = i.density_g_ml
FROM ingredient i
WHERE mi.ingredient_id = i.id
  AND (mi.snapshot_name, mi.snapshot_kcal_per_100g, mi.snapshot_density_g_ml) IS DISTINCT FROM
      (i.name, COALESCE(ROUND(i.kcal_per_100g), mi.snapshot_kcal_per_100g), i.density_g_ml);
--> statement-breakpoint
UPDATE trip_meal tm
SET snapshot_name = i.name,
    snapshot_kcal = CASE WHEN i.kcal_per_100g IS NULL THEN tm.snapshot_kcal
      ELSE ROUND(i.kcal_per_100g / 100 * tm.snapshot_weight_g)::int END
FROM ingredient i
WHERE tm.ingredient_id = i.id AND tm.meal_id IS NULL
  AND (tm.snapshot_name, tm.snapshot_kcal) IS DISTINCT FROM
      (i.name, CASE WHEN i.kcal_per_100g IS NULL THEN tm.snapshot_kcal
        ELSE ROUND(i.kcal_per_100g / 100 * tm.snapshot_weight_g)::int END);
--> statement-breakpoint
UPDATE trip_meal tm SET snapshot_name = m.name
FROM meal m WHERE tm.meal_id = m.id AND tm.snapshot_name IS DISTINCT FROM m.name;
--> statement-breakpoint
WITH totals AS (
  SELECT m.id, GREATEST(m.base_servings, 1) AS base, SUM(mi.amount_g) AS g,
    SUM(ROUND(COALESCE(mi.snapshot_kcal_per_100g, 0) / 100 * mi.amount_g)) AS kcal
  FROM meal m JOIN meal_ingredient mi ON mi.meal_id = m.id
  GROUP BY m.id, m.base_servings
), changed AS (
  UPDATE trip_meal tm
  SET snapshot_kcal = ROUND(t.kcal * tm.servings / t.base),
      snapshot_weight_g = ROUND(t.g * tm.servings / t.base)
  FROM totals t WHERE tm.meal_id = t.id
    AND (tm.snapshot_kcal, tm.snapshot_weight_g) IS DISTINCT FROM
        (ROUND(t.kcal * tm.servings / t.base)::int, ROUND(t.g * tm.servings / t.base)::int)
  RETURNING tm.id, tm.trip_id, tm.snapshot_weight_g
), new_sums AS (
  SELECT tm.trip_id, SUM(COALESCE(c.snapshot_weight_g, tm.snapshot_weight_g)) AS g
  FROM trip_meal tm LEFT JOIN changed c ON c.id = tm.id
  WHERE tm.trip_id IN (SELECT trip_id FROM changed)
  GROUP BY tm.trip_id
)
UPDATE trip t
SET food_g_per_day = ROUND(s.g::numeric / GREATEST(COALESCE(t.nights, 0) + 1, 1))
FROM new_sums s WHERE t.id = s.trip_id;
