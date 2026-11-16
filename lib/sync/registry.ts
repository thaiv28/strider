import "server-only";
import { getTableColumns, getTableName } from "drizzle-orm";
import type { AnyPgTable } from "drizzle-orm/pg-core";
import * as s from "@/src/db/schema";

export type Scope = "user" | "trip" | `parent:${string}`;
export type Entry = { name: string; table: AnyPgTable; scope: Scope; foreignKeys: Record<string, string>; writable: Set<string>; insertOnly: Set<string>; hidden: Set<string> };
const allowed: Record<string, string> = {
  gear_category: "name parent_id weight_class sort_order",
  gear_item: "category_id name weight_g price_cents quantity sort_order default_weight_class status is_kit product_url notes acquired_at retired_at",
  gear_component: "gear_item_id name weight_g quantity notes",
  wishlist_replacement: "wishlist_item_id replaces_item_id",
  wishlist_loadout: "wishlist_item_id loadout_id included",
  loadout: "name is_default sort_order notes",
  loadout_item: "loadout_id gear_item_id weight_class",
  trail: "name region area_type typical_distance_mi typical_elevation_ft notes",
  trip: "trail_id name status start_date nights party_size grocery_people grocery_removed region area_type lat lon distance_mi elevation_gain_ft food_g_per_day water_g_per_day fuel_g_per_day trailhead permit_required permit_notes driving_notes water_sources planning_notes trip_report packing_list",
  trip_day: "trip_id day_number distance_mi elevation_gain_ft",
  trip_gear: "trip_id gear_item_id snapshot_name snapshot_weight_g snapshot_category weight_class quantity packed sort_order",
  trip_campsite: "trip_id distance_mi lat lon ele_ft",
  trip_gpx: "trip_id filename start_lat start_lon distance_mi elevation_gain_ft min_ele_ft max_ele_ft track profile points",
  trip_permit: "filename",
  trip_meal: "trip_id day_number meal_type meal_id ingredient_id servings snapshot_name snapshot_kcal snapshot_weight_g sort_order",
  trip_consumable: "trip_id type label grams day_number",
  ingredient: "name kcal_per_100g density_g_ml density_source reference_name default_serving_g category source source_id notes",
  meal: "name base_servings meal_type is_hot water_ml notes",
  meal_ingredient: "meal_id ingredient_id snapshot_name snapshot_kcal_per_100g snapshot_density_g_ml entered_amount entered_unit amount_g sort_order",
  calendar_feed: "label color url sort_order",
  energy_params: "bmr cal_per_energy_mile ft_per_energy_mile sex weight_lb height_in age_years",
  report_settings: "template packing_default",
};
function entry(table: AnyPgTable, scope: Scope, foreignKeys: Record<string, string> = {}, excluded: string[] = []): Entry {
  const name = getTableName(table);
  const columns = Object.values(getTableColumns(table)).map(c => c.name);
  const insertOnly = new Set(columns.filter(c => c.startsWith("snapshot_") || c === "food_g_per_day"));
  const blocked = new Set(["id", "user_id", "sync_id", "server_seq", "share_token", "edit_token", "data", "created_at", "updated_at", "uploaded_at", ...excluded]);
  return { name, table, scope, foreignKeys, writable: new Set((allowed[name] ?? "").split(" ").filter(c => c && !blocked.has(c))), insertOnly,
    hidden: new Set(name === "trip" ? ["trip_report", "share_token", "edit_token"] : []) };
}
export const registry: Record<string, Entry> = Object.fromEntries([
  entry(s.gearCategory, "user", { parent_id: "gear_category" }),
  entry(s.gearItem, "user", { category_id: "gear_category" }),
  entry(s.gearComponent, "parent:gear_item", { gear_item_id: "gear_item" }),
  entry(s.wishlistReplacement, "parent:gear_item", { wishlist_item_id: "gear_item", replaces_item_id: "gear_item" }),
  entry(s.wishlistLoadout, "parent:gear_item", { wishlist_item_id: "gear_item", loadout_id: "loadout" }),
  entry(s.loadout, "user"), entry(s.loadoutItem, "parent:loadout", { loadout_id: "loadout", gear_item_id: "gear_item" }),
  entry(s.trail, "user"), entry(s.trip, "user", { trail_id: "trail" }),
  ...[s.tripDay, s.tripGear, s.tripCampsite, s.tripGpx, s.tripPermit, s.tripMeal, s.tripConsumable].map(t => entry(t, "trip", {
    trip_id: "trip", ...(t === s.tripGear ? { gear_item_id: "gear_item" } : {}),
    ...(t === s.tripMeal ? { meal_id: "meal", ingredient_id: "ingredient" } : {}),
  })),
  entry(s.ingredient, "user"), entry(s.meal, "user"),
  entry(s.mealIngredient, "parent:meal", { meal_id: "meal", ingredient_id: "ingredient" }),
  entry(s.calendarFeed, "user"), entry(s.energyParams, "user"), entry(s.reportSettings, "user"),
].map(e => [e.name, e]));
// Identifiers only originate in the registry or Drizzle metadata, never request text.
export const ident = (value: string) => `"${value.replaceAll('"', '""')}"`;
export type SyncRow = Record<string, unknown>;
