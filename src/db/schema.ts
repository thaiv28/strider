import {
  pgTable,
  pgEnum,
  serial,
  integer,
  text,
  boolean,
  timestamp,
  numeric,
  date,
  jsonb,
  customType,
  uniqueIndex,
  type AnyPgColumn,
} from "drizzle-orm/pg-core";

const bytea = customType<{ data: Buffer; default: false }>({
  dataType() {
    return "bytea";
  },
});

// Canonical weight unit across the schema is grams (integer). oz/lb are derived
// in the UI to avoid rounding drift.

export const weightClass = pgEnum("weight_class", ["base", "worn", "consumable"]);
// Per-trip a selected item is only ever base (packed) or worn — consumables are
// computed from the meal plan + water/fuel rates, not from gear selection.
export const packClass = pgEnum("pack_class", ["base", "worn"]);
export const gearStatus = pgEnum("gear_status", ["current", "retired", "wishlist"]);
export const tripStatus = pgEnum("trip_status", ["idea", "planned", "completed"]);
export const fileKind = pgEnum("file_kind", ["gpx", "image", "doc"]);
export const consumableType = pgEnum("consumable_type", ["water", "fuel"]);
export const mealType = pgEnum("meal_type", ["breakfast", "lunch", "dinner", "snack"]);

export const users = pgTable("users", {
  id: serial("id").primaryKey(),
  email: text("email").notNull().unique(),
  name: text("name"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
});

// Tunable constants for the calorie-burn model (Calories sheet).
export const energyParams = pgTable("energy_params", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id)
    .unique(),
  bmr: integer("bmr").notNull().default(1735),
  calPerEnergyMile: integer("cal_per_energy_mile").notNull().default(200),
  ftPerEnergyMile: integer("ft_per_energy_mile").notNull().default(625),
  // Optional body stats for the Mifflin–St Jeor BMR calculator (imperial).
  sex: text("sex"), // 'male' | 'female'
  weightLb: numeric("weight_lb"),
  heightIn: numeric("height_in"),
  ageYears: integer("age_years"),
});

// Slot/grouping tree: parent groups (Essentials, Worn, Consumables...) hold
// slots (Shelter, Rain jacket, Fuel...).
export const gearCategory = pgTable("gear_category", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  parentId: integer("parent_id").references((): AnyPgColumn => gearCategory.id),
  weightClass: weightClass("weight_class"),
  sortOrder: integer("sort_order").notNull().default(0),
});

export const gearItem = pgTable("gear_item", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  categoryId: integer("category_id").references(() => gearCategory.id),
  name: text("name").notNull(),
  weightG: integer("weight_g"), // per-unit weight; total = weightG * quantity
  priceCents: integer("price_cents"), // per-unit price; surfaced on the wishlist
  quantity: integer("quantity").notNull().default(1),
  sortOrder: integer("sort_order").notNull().default(0),
  defaultWeightClass: weightClass("default_weight_class").notNull().default("base"),
  status: gearStatus("status").notNull().default("current"),
  isKit: boolean("is_kit").notNull().default(false),
  productUrl: text("product_url"),
  notes: text("notes"),
  acquiredAt: date("acquired_at"),
  retiredAt: date("retired_at"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow().notNull(),
});

// Contents of a composite kit (First Aid, Repair Kit). Parent gear_item.weight_g
// is the sum of its components.
export const gearComponent = pgTable("gear_component", {
  id: serial("id").primaryKey(),
  gearItemId: integer("gear_item_id")
    .notNull()
    .references(() => gearItem.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  weightG: integer("weight_g").notNull(),
  quantity: integer("quantity").notNull().default(1),
  notes: text("notes"),
});

// Which current gear a wishlist item would replace (many-to-many). Drives the
// projected base-weight bar: buying the wishlist item retires the linked items.
export const wishlistReplacement = pgTable(
  "wishlist_replacement",
  {
    id: serial("id").primaryKey(),
    wishlistItemId: integer("wishlist_item_id")
      .notNull()
      .references(() => gearItem.id, { onDelete: "cascade" }),
    replacesItemId: integer("replaces_item_id")
      .notNull()
      .references(() => gearItem.id, { onDelete: "cascade" }),
  },
  (t) => ({
    wishlistReplacementUniq: uniqueIndex("wishlist_replacement_uniq").on(
      t.wishlistItemId,
      t.replacesItemId,
    ),
  }),
);

// Explicit override of a wishlist item's assumed loadout membership. Absent →
// assumed (member iff any item it replaces is in the loadout). Present → forced
// in (included=true) or out (false), so the projection is under full control.
export const wishlistLoadout = pgTable(
  "wishlist_loadout",
  {
    id: serial("id").primaryKey(),
    wishlistItemId: integer("wishlist_item_id")
      .notNull()
      .references(() => gearItem.id, { onDelete: "cascade" }),
    loadoutId: integer("loadout_id")
      .notNull()
      .references(() => loadout.id, { onDelete: "cascade" }),
    included: boolean("included").notNull(),
  },
  (t) => ({
    wishlistLoadoutUniq: uniqueIndex("wishlist_loadout_uniq").on(t.wishlistItemId, t.loadoutId),
  }),
);

// A reusable kit — a named subset of gear. Trips copy a loadout as a template.
export const loadout = pgTable("loadout", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  isDefault: boolean("is_default").notNull().default(false),
  notes: text("notes"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
});

// Membership of a gear item in a loadout, with a per-loadout worn/packed choice.
export const loadoutItem = pgTable(
  "loadout_item",
  {
    id: serial("id").primaryKey(),
    loadoutId: integer("loadout_id")
      .notNull()
      .references(() => loadout.id, { onDelete: "cascade" }),
    gearItemId: integer("gear_item_id")
      .notNull()
      .references(() => gearItem.id, { onDelete: "cascade" }),
    weightClass: packClass("weight_class").notNull().default("base"),
  },
  (t) => ({
    loadoutItemUniq: uniqueIndex("loadout_item_uniq").on(t.loadoutId, t.gearItemId),
  }),
);

// Reusable route/destination catalog (a trail can be hiked more than once).
export const trail = pgTable("trail", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  region: text("region"),
  areaType: text("area_type"),
  typicalDistanceMi: numeric("typical_distance_mi"),
  typicalElevationFt: integer("typical_elevation_ft"),
  notes: text("notes"),
});

export const trip = pgTable("trip", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  trailId: integer("trail_id").references(() => trail.id),
  name: text("name").notNull(),
  status: tripStatus("status").notNull().default("planned"),
  startDate: date("start_date"),
  nights: integer("nights"),
  partySize: integer("party_size").notNull().default(1),
  // People count the grocery list scales by; null → follow party_size.
  groceryPeople: integer("grocery_people"),
  // Grocery-item keys the user removed from the list (excluded from copy/report).
  groceryRemoved: jsonb("grocery_removed").$type<string[]>(),
  // Per-trip overrides for the linked trail's fields (null → use trail).
  region: text("region"),
  areaType: text("area_type"),
  lat: numeric("lat"),
  lon: numeric("lon"),
  distanceMi: numeric("distance_mi"),
  elevationGainFt: integer("elevation_gain_ft"),
  // food_g_per_day is a per-day rate (× days); water/fuel are flat total grams
  // carried (average), defaulting to 34.0 oz water and 7.1 oz fuel.
  foodGPerDay: integer("food_g_per_day").notNull().default(0),
  waterGPerDay: integer("water_g_per_day").notNull().default(964),
  fuelGPerDay: integer("fuel_g_per_day").notNull().default(201),
  trailhead: text("trailhead"),
  permitRequired: boolean("permit_required"),
  permitNotes: text("permit_notes"),
  drivingNotes: text("driving_notes"),
  waterSources: text("water_sources"),
  planningNotes: text("planning_notes"),
  tripReport: text("trip_report"),
  // Bearer token for the trip's persistent, unauthenticated read-only view.
  // Null means sharing is disabled; rotating/revoking invalidates the old URL.
  shareToken: text("share_token").unique(),
  // Shared participant packing list; null falls back to report_settings.packing_default.
  packingList: text("packing_list"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow().notNull(),
});

// Per-user, singleton: the pre-trip report template and the master packing list
// each trip's list is seeded from. Null columns fall back to code defaults (lib/report).
export const reportSettings = pgTable("report_settings", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id)
    .unique(),
  template: text("template"),
  packingDefault: text("packing_default"),
});

// Read-only Google Calendar (or any) secret iCal feeds, shown on the planning
// calendar. url is a bearer-token .ics address; treat as a secret.
export const calendarFeed = pgTable("calendar_feed", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  label: text("label").notNull(),
  color: text("color").notNull().default("#1f7a70"),
  url: text("url").notNull(),
  sortOrder: integer("sort_order").notNull().default(0),
});

// One uploaded permit per trip (PDF or image), bytes stored inline so it travels
// with DB backups. Replaced on re-upload; tripId is the PK.
export const tripPermit = pgTable("trip_permit", {
  tripId: integer("trip_id")
    .primaryKey()
    .references(() => trip.id, { onDelete: "cascade" }),
  filename: text("filename").notNull(),
  mimeType: text("mime_type").notNull(),
  sizeBytes: integer("size_bytes").notNull(),
  data: bytea("data").notNull(),
  uploadedAt: timestamp("uploaded_at", { withTimezone: true }).defaultNow().notNull(),
});

export const tripDay = pgTable("trip_day", {
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  dayNumber: integer("day_number").notNull(),
  distanceMi: numeric("distance_mi"),
  elevationGainFt: integer("elevation_gain_ft"),
});

// Per-trip gear selection. Name/weight/category are frozen at trip time so a
// completed trip's totals never shift when live gear is re-weighed or retired.
export const tripGear = pgTable(
  "trip_gear",
  {
    id: serial("id").primaryKey(),
    tripId: integer("trip_id")
      .notNull()
      .references(() => trip.id, { onDelete: "cascade" }),
    gearItemId: integer("gear_item_id").references(() => gearItem.id),
    snapshotName: text("snapshot_name").notNull(),
    snapshotWeightG: integer("snapshot_weight_g").notNull(), // per-unit
    snapshotCategory: text("snapshot_category"),
    weightClass: packClass("weight_class").notNull().default("base"),
    quantity: integer("quantity").notNull().default(1),
    packed: boolean("packed").notNull().default(false),
    sortOrder: integer("sort_order").notNull().default(0),
  },
  (t) => ({
    tripItemUniq: uniqueIndex("trip_gear_trip_item_uniq").on(t.tripId, t.gearItemId),
  }),
);

// Reusable food component with per-100g nutrition. density_g_ml (when known,
// from API portion data or entered once) enables weight<->volume conversion.
export const ingredient = pgTable("ingredient", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  kcalPer100g: numeric("kcal_per_100g"),
  densityGMl: numeric("density_g_ml"),
  defaultServingG: integer("default_serving_g"),
  category: text("category"),
  source: text("source"), // 'usda' | 'off' | 'manual'
  sourceId: text("source_id"), // FDC id or Open Food Facts barcode
  notes: text("notes"),
});

// A recipe yielding base_servings servings. Ingredient amounts are stored for
// the whole recipe at base_servings; per-serving = amount_g / base_servings.
export const meal = pgTable("meal", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  baseServings: integer("base_servings").notNull().default(1),
  mealType: mealType("meal_type"),
  isHot: boolean("is_hot").notNull().default(true), // needs a stove → consumes fuel
  waterMl: integer("water_ml"), // boiling water to cook; null on a hot meal → default rate
  notes: text("notes"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
});

// An ingredient line in a meal. Nutrition is snapshotted so editing the pantry
// ingredient later never silently changes an existing recipe's totals.
export const mealIngredient = pgTable("meal_ingredient", {
  id: serial("id").primaryKey(),
  mealId: integer("meal_id")
    .notNull()
    .references(() => meal.id, { onDelete: "cascade" }),
  ingredientId: integer("ingredient_id").references(() => ingredient.id),
  snapshotName: text("snapshot_name").notNull(),
  snapshotKcalPer100g: numeric("snapshot_kcal_per_100g"),
  snapshotDensityGMl: numeric("snapshot_density_g_ml"),
  amountG: integer("amount_g").notNull(), // grams for the whole recipe at base_servings
  sortOrder: integer("sort_order").notNull().default(0),
});

// A meal or standalone ingredient placed on a trip day. Exactly one of
// meal_id / ingredient_id is set. Totals are snapshotted at plan time.
export const tripMeal = pgTable("trip_meal", {
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  dayNumber: integer("day_number").notNull(),
  mealType: mealType("meal_type"),
  mealId: integer("meal_id").references(() => meal.id),
  ingredientId: integer("ingredient_id").references(() => ingredient.id),
  servings: numeric("servings").notNull().default("1"), // meal scale; ingredient uses snapshot grams directly
  snapshotName: text("snapshot_name").notNull(),
  snapshotKcal: integer("snapshot_kcal").notNull(), // total for this entry
  snapshotWeightG: integer("snapshot_weight_g").notNull(), // total for this entry
  sortOrder: integer("sort_order").notNull().default(0),
});

export const tripConsumable = pgTable("trip_consumable", {
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  type: consumableType("type").notNull(),
  label: text("label"),
  grams: integer("grams").notNull(),
  dayNumber: integer("day_number"),
});

// Uploaded GPX for a trip: raw XML plus derived route data for map + profile.
export const tripGpx = pgTable("trip_gpx", {
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" })
    .unique(),
  filename: text("filename"),
  startLat: numeric("start_lat"),
  startLon: numeric("start_lon"),
  distanceMi: numeric("distance_mi"),
  elevationGainFt: integer("elevation_gain_ft"),
  minEleFt: integer("min_ele_ft"),
  maxEleFt: integer("max_ele_ft"),
  track: jsonb("track").$type<[number, number][]>(), // [lat, lon] polyline (legacy)
  profile: jsonb("profile").$type<{ d: number; e: number }[]>(), // cumulative mi, ele ft (legacy)
  // Unified route points shared by the map + elevation profile + campsite placement.
  points: jsonb("points").$type<{ d: number; lat: number; lon: number; e: number }[]>(),
  uploadedAt: timestamp("uploaded_at", { withTimezone: true }).defaultNow().notNull(),
});

// A night's campsite along the route, placed on the elevation profile.
export const tripCampsite = pgTable("trip_campsite", {
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  distanceMi: numeric("distance_mi").notNull(),
  lat: numeric("lat"),
  lon: numeric("lon"),
  eleFt: integer("ele_ft"),
});

export const file = pgTable("file", {
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  tripId: integer("trip_id").references(() => trip.id, { onDelete: "cascade" }),
  kind: fileKind("kind").notNull(),
  filename: text("filename").notNull(),
  storageUrl: text("storage_url").notNull(),
  uploadedAt: timestamp("uploaded_at", { withTimezone: true }).defaultNow().notNull(),
});
