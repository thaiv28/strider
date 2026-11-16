import { sql } from "drizzle-orm";
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
  index,
  uuid,
  bigint,
  bigserial,
  primaryKey,
  pgSequence,
  type AnyPgColumn,
} from "drizzle-orm/pg-core";

export const syncSeq = pgSequence("sync_seq");

function syncColumns() {
  return {
    syncId: uuid("sync_id").notNull().defaultRandom(),
    serverSeq: bigint("server_seq", { mode: "bigint" }).notNull().default(sql`0`),
  };
}

function syncIndexes(name: string, t: { syncId: AnyPgColumn; serverSeq: AnyPgColumn }) {
  return {
    syncIdUnique: uniqueIndex(`${name}_sync_id_uniq`).on(t.syncId),
    serverSeqIndex: index(`${name}_server_seq_idx`).on(t.serverSeq),
  };
}

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
  googleSubject: text("google_subject").unique(),
  imageUrl: text("image_url"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
});

// Tunable constants for the calorie-burn model (Calories sheet).
export const energyParams = pgTable("energy_params", {
  ...syncColumns(),
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
}, (t) => syncIndexes("energy_params", t));

// Slot/grouping tree: parent groups (Essentials, Worn, Consumables...) hold
// slots (Shelter, Rain jacket, Fuel...).
export const gearCategory = pgTable("gear_category", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  parentId: integer("parent_id").references((): AnyPgColumn => gearCategory.id),
  weightClass: weightClass("weight_class"),
  sortOrder: integer("sort_order").notNull().default(0),
}, (t) => syncIndexes("gear_category", t));

export const gearItem = pgTable("gear_item", {
  ...syncColumns(),
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
}, (t) => syncIndexes("gear_item", t));

// Contents of a composite kit (First Aid, Repair Kit). Parent gear_item.weight_g
// is the sum of its components.
export const gearComponent = pgTable("gear_component", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  gearItemId: integer("gear_item_id")
    .notNull()
    .references(() => gearItem.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  weightG: integer("weight_g").notNull(),
  quantity: integer("quantity").notNull().default(1),
  notes: text("notes"),
}, (t) => syncIndexes("gear_component", t));

// Which current gear a wishlist item would replace (many-to-many). Drives the
// projected base-weight bar: buying the wishlist item retires the linked items.
export const wishlistReplacement = pgTable(
  "wishlist_replacement",
  {
    ...syncColumns(),
    id: serial("id").primaryKey(),
    wishlistItemId: integer("wishlist_item_id")
      .notNull()
      .references(() => gearItem.id, { onDelete: "cascade" }),
    replacesItemId: integer("replaces_item_id")
      .notNull()
      .references(() => gearItem.id, { onDelete: "cascade" }),
  },
  (t) => ({
    ...syncIndexes("wishlist_replacement", t),
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
    ...syncColumns(),
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
    ...syncIndexes("wishlist_loadout", t),
    wishlistLoadoutUniq: uniqueIndex("wishlist_loadout_uniq").on(t.wishlistItemId, t.loadoutId),
  }),
);

// A reusable kit — a named subset of gear. Trips copy a loadout as a template.
export const loadout = pgTable("loadout", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  isDefault: boolean("is_default").notNull().default(false),
  sortOrder: integer("sort_order").notNull().default(0),
  notes: text("notes"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
}, (t) => syncIndexes("loadout", t));

// Membership of a gear item in a loadout, with a per-loadout worn/packed choice.
export const loadoutItem = pgTable(
  "loadout_item",
  {
    ...syncColumns(),
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
    ...syncIndexes("loadout_item", t),
    loadoutItemUniq: uniqueIndex("loadout_item_uniq").on(t.loadoutId, t.gearItemId),
  }),
);

// Reusable route/destination catalog (a trail can be hiked more than once).
export const trail = pgTable("trail", {
  ...syncColumns(),
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
}, (t) => syncIndexes("trail", t));

export const trip = pgTable("trip", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  trailId: integer("trail_id").references(() => trail.id),
  name: text("name").notNull(),
  status: tripStatus("status").notNull().default("idea"),
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
  // Signed-in collaborators receive edit access after opening this bearer link.
  editToken: text("edit_token").unique(),
  // Shared participant packing list; null falls back to report_settings.packing_default.
  packingList: text("packing_list"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow().notNull(),
}, (t) => syncIndexes("trip", t));

// A visit remembers a link in the visitor's trip list. Revoking a link removes
// its visits, including edit grants, without affecting the other permission.
export const tripShareVisit = pgTable(
  "trip_share_visit",
  {
    serverSeq: bigint("server_seq", { mode: "bigint" }).notNull().default(sql`0`),
    tripId: integer("trip_id").notNull().references(() => trip.id, { onDelete: "cascade" }),
    userId: integer("user_id").notNull().references(() => users.id, { onDelete: "cascade" }),
    permission: text("permission").$type<"view" | "edit">().notNull(),
    visitedAt: timestamp("visited_at", { withTimezone: true }).defaultNow().notNull(),
  },
  (t) => ({ serverSeqIndex: index("trip_share_visit_server_seq_idx").on(t.serverSeq), tripShareVisitUniq: uniqueIndex("trip_share_visit_uniq").on(t.tripId, t.userId, t.permission) }),
);

// Per-user, singleton: the pre-trip report template and the master packing list
// each trip's list is seeded from. Null columns fall back to code defaults (lib/report).
export const reportSettings = pgTable("report_settings", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id)
    .unique(),
  template: text("template"),
  packingDefault: text("packing_default"),
}, (t) => syncIndexes("report_settings", t));

// Read-only Google Calendar (or any) secret iCal feeds, shown on the planning
// calendar. url is a bearer-token .ics address; treat as a secret.
export const calendarFeed = pgTable("calendar_feed", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  label: text("label").notNull(),
  color: text("color").notNull().default("#1f7a70"),
  url: text("url").notNull(),
  sortOrder: integer("sort_order").notNull().default(0),
}, (t) => syncIndexes("calendar_feed", t));

// One uploaded permit per trip (PDF or image), bytes stored inline so it travels
// with DB backups. Replaced on re-upload; tripId is the PK.
export const tripPermit = pgTable("trip_permit", {
  ...syncColumns(),
  sha256: text("sha256"),
  tripId: integer("trip_id")
    .primaryKey()
    .references(() => trip.id, { onDelete: "cascade" }),
  filename: text("filename").notNull(),
  mimeType: text("mime_type").notNull(),
  sizeBytes: integer("size_bytes").notNull(),
  data: bytea("data").notNull(),
  uploadedAt: timestamp("uploaded_at", { withTimezone: true }).defaultNow().notNull(),
}, (t) => syncIndexes("trip_permit", t));

export const tripDay = pgTable("trip_day", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  dayNumber: integer("day_number").notNull(),
  distanceMi: numeric("distance_mi"),
  elevationGainFt: integer("elevation_gain_ft"),
}, (t) => syncIndexes("trip_day", t));

// Per-trip gear selection. Name/weight copies are kept current by
// lib/trip-propagation.ts; category and other allocation choices stay per-trip.
export const tripGear = pgTable(
  "trip_gear",
  {
    ...syncColumns(),
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
    ...syncIndexes("trip_gear", t),
    tripItemUniq: uniqueIndex("trip_gear_trip_item_uniq").on(t.tripId, t.gearItemId),
  }),
);

// Reusable food component with per-100g nutrition. density_g_ml (when known,
// from USDA reference data, a product label, or a user measurement) enables weight<->volume conversion.
export const ingredient = pgTable("ingredient", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  userId: integer("user_id")
    .notNull()
    .references(() => users.id),
  name: text("name").notNull(),
  kcalPer100g: numeric("kcal_per_100g"),
  densityGMl: numeric("density_g_ml"),
  densitySource: text("density_source").$type<"usda" | "label" | "manual">(),
  referenceName: text("reference_name"),
  defaultServingG: integer("default_serving_g"),
  category: text("category"),
  source: text("source"), // 'usda' | 'off' | 'manual'
  sourceId: text("source_id"), // FDC id or Open Food Facts barcode
  notes: text("notes"),
}, (t) => syncIndexes("ingredient", t));

// A recipe yielding base_servings servings. Ingredient amounts are stored for
// the whole recipe at base_servings; per-serving = amount_g / base_servings.
export const meal = pgTable("meal", {
  ...syncColumns(),
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
}, (t) => syncIndexes("meal", t));

// An ingredient line in a meal. Name, nutrition and density copies are kept
// current by lib/trip-propagation.ts; amounts belong to the recipe.
export const mealIngredient = pgTable("meal_ingredient", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  mealId: integer("meal_id")
    .notNull()
    .references(() => meal.id, { onDelete: "cascade" }),
  ingredientId: integer("ingredient_id").references(() => ingredient.id),
  snapshotName: text("snapshot_name").notNull(),
  snapshotKcalPer100g: numeric("snapshot_kcal_per_100g"),
  snapshotDensityGMl: numeric("snapshot_density_g_ml"),
  enteredAmount: numeric("entered_amount"), // whole recipe amount as typed at base_servings
  enteredUnit: text("entered_unit"), // g, oz, or a VOLUME_ML key
  amountG: integer("amount_g").notNull(), // grams for the whole recipe at base_servings
  sortOrder: integer("sort_order").notNull().default(0),
}, (t) => syncIndexes("meal_ingredient", t));

// A meal or standalone ingredient placed on a trip day. Exactly one of
// meal_id / ingredient_id is set. Name/totals copies are kept current by
// lib/trip-propagation.ts; servings and standalone ingredient grams stay per-trip.
export const tripMeal = pgTable("trip_meal", {
  ...syncColumns(),
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
}, (t) => syncIndexes("trip_meal", t));

export const tripConsumable = pgTable("trip_consumable", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  type: consumableType("type").notNull(),
  label: text("label"),
  grams: integer("grams").notNull(),
  dayNumber: integer("day_number"),
}, (t) => syncIndexes("trip_consumable", t));

// Uploaded GPX for a trip: raw XML plus derived route data for map + profile.
export const tripGpx = pgTable("trip_gpx", {
  ...syncColumns(),
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
}, (t) => syncIndexes("trip_gpx", t));

// A night's campsite along the route, placed on the elevation profile.
export const tripCampsite = pgTable("trip_campsite", {
  ...syncColumns(),
  id: serial("id").primaryKey(),
  tripId: integer("trip_id")
    .notNull()
    .references(() => trip.id, { onDelete: "cascade" }),
  distanceMi: numeric("distance_mi").notNull(),
  lat: numeric("lat"),
  lon: numeric("lon"),
  eleFt: integer("ele_ft"),
}, (t) => syncIndexes("trip_campsite", t));

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

// Device bearer tokens and short-lived authorization codes are stored only as hashes.
export const deviceSession = pgTable("device_session", {
  id: serial("id").primaryKey(),
  userId: integer("user_id").notNull().references(() => users.id, { onDelete: "cascade" }),
  deviceName: text("device_name").notNull(),
  tokenHash: text("token_hash").notNull().unique(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  lastUsedAt: timestamp("last_used_at", { withTimezone: true }),
  revokedAt: timestamp("revoked_at", { withTimezone: true }),
});

export const deviceAuthCode = pgTable("device_auth_code", {
  codeHash: text("code_hash").primaryKey(),
  userId: integer("user_id").notNull().references(() => users.id, { onDelete: "cascade" }),
  codeChallenge: text("code_challenge").notNull(),
  deviceName: text("device_name").notNull(),
  expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
  usedAt: timestamp("used_at", { withTimezone: true }),
});

export const syncBatch = pgTable("sync_batch", {
  batchId: uuid("batch_id").notNull(),
  userId: integer("user_id").notNull().references(() => users.id, { onDelete: "cascade" }),
  result: jsonb("result").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
}, (t) => ({ pk: primaryKey({ columns: [t.userId, t.batchId] }) }));

export const syncTombstone = pgTable("sync_tombstone", {
  id: bigserial("id", { mode: "bigint" }).primaryKey(),
  tbl: text("tbl").notNull(),
  syncId: uuid("sync_id").notNull(),
  ownerUserId: integer("owner_user_id").notNull(),
  tripId: integer("trip_id"),
  serverSeq: bigint("server_seq", { mode: "bigint" }).notNull(),
  deletedAt: timestamp("deleted_at", { withTimezone: true }).notNull().defaultNow(),
}, (t) => ({
  ownerSeqIndex: index("sync_tombstone_owner_seq_idx").on(t.ownerUserId, t.serverSeq),
  tripSeqIndex: index("sync_tombstone_trip_seq_idx").on(t.tripId, t.serverSeq),
}));
