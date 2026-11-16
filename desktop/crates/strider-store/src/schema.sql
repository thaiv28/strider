CREATE TABLE energy_params(
 sync_id TEXT PRIMARY KEY NOT NULL,
 bmr INTEGER NOT NULL DEFAULT 1735,
 cal_per_energy_mile INTEGER NOT NULL DEFAULT 200,
 ft_per_energy_mile INTEGER NOT NULL DEFAULT 625,
 sex TEXT,
 weight_lb TEXT,
 height_in TEXT,
 age_years INTEGER
);
CREATE UNIQUE INDEX energy_params_singleton ON energy_params((1));
CREATE TABLE gear_category(
 sync_id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL,
 parent_id TEXT REFERENCES gear_category(sync_id),
 weight_class TEXT CHECK (weight_class IN ('base','worn','consumable')),
 sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE gear_item(
 sync_id TEXT PRIMARY KEY NOT NULL,
 category_id TEXT REFERENCES gear_category(sync_id),
 name TEXT NOT NULL,
 weight_g INTEGER,
 price_cents INTEGER,
 quantity INTEGER NOT NULL DEFAULT 1,
 sort_order INTEGER NOT NULL DEFAULT 0,
 default_weight_class TEXT NOT NULL DEFAULT 'base' CHECK (default_weight_class IN ('base','worn','consumable')),
 status TEXT NOT NULL DEFAULT 'current' CHECK (status IN ('current','retired','wishlist')),
 is_kit INTEGER NOT NULL DEFAULT 0 CHECK (is_kit IN (0,1)),
 product_url TEXT,
 notes TEXT,
 acquired_at TEXT,
 retired_at TEXT,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE gear_component(
 sync_id TEXT PRIMARY KEY NOT NULL,
 gear_item_id TEXT NOT NULL REFERENCES gear_item(sync_id) ON DELETE CASCADE,
 name TEXT NOT NULL,
 weight_g INTEGER NOT NULL,
 quantity INTEGER NOT NULL DEFAULT 1,
 notes TEXT
);
CREATE TABLE wishlist_replacement(
 sync_id TEXT PRIMARY KEY NOT NULL,
 wishlist_item_id TEXT NOT NULL REFERENCES gear_item(sync_id) ON DELETE CASCADE,
 replaces_item_id TEXT NOT NULL REFERENCES gear_item(sync_id) ON DELETE CASCADE,
 UNIQUE(wishlist_item_id,replaces_item_id)
);
CREATE TABLE wishlist_loadout(
 sync_id TEXT PRIMARY KEY NOT NULL,
 wishlist_item_id TEXT NOT NULL REFERENCES gear_item(sync_id) ON DELETE CASCADE,
 loadout_id TEXT NOT NULL REFERENCES loadout(sync_id) ON DELETE CASCADE,
 included INTEGER NOT NULL CHECK (included IN (0,1)),
 UNIQUE(wishlist_item_id,loadout_id)
);
CREATE TABLE loadout(
 sync_id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL,
 is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0,1)),
 sort_order INTEGER NOT NULL DEFAULT 0,
 notes TEXT,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE loadout_item(
 sync_id TEXT PRIMARY KEY NOT NULL,
 loadout_id TEXT NOT NULL REFERENCES loadout(sync_id) ON DELETE CASCADE,
 gear_item_id TEXT NOT NULL REFERENCES gear_item(sync_id) ON DELETE CASCADE,
 weight_class TEXT NOT NULL DEFAULT 'base' CHECK (weight_class IN ('base','worn')),
 UNIQUE(loadout_id,gear_item_id)
);
CREATE TABLE trail(
 sync_id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL,
 region TEXT,
 area_type TEXT,
 typical_distance_mi TEXT,
 typical_elevation_ft INTEGER,
 notes TEXT
);
CREATE TABLE trip(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trail_id TEXT REFERENCES trail(sync_id),
 name TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'idea' CHECK (status IN ('idea','planned','completed')),
 start_date TEXT,
 nights INTEGER,
 party_size INTEGER NOT NULL DEFAULT 1,
 grocery_people INTEGER,
 grocery_removed TEXT,
 region TEXT,
 area_type TEXT,
 lat TEXT,
 lon TEXT,
 distance_mi TEXT,
 elevation_gain_ft INTEGER,
 food_g_per_day INTEGER NOT NULL DEFAULT 0,
 water_g_per_day INTEGER NOT NULL DEFAULT 964,
 fuel_g_per_day INTEGER NOT NULL DEFAULT 201,
 trailhead TEXT,
 permit_required INTEGER CHECK (permit_required IN (0,1)),
 permit_notes TEXT,
 driving_notes TEXT,
 water_sources TEXT,
 planning_notes TEXT,
 trip_report TEXT,
 share_token TEXT UNIQUE,
 edit_token TEXT UNIQUE,
 packing_list TEXT,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 access TEXT NOT NULL DEFAULT 'owner' CHECK(access IN ('owner','edit','view'))
);
CREATE TABLE report_settings(
 sync_id TEXT PRIMARY KEY NOT NULL,
 template TEXT,
 packing_default TEXT
);
CREATE UNIQUE INDEX report_settings_singleton ON report_settings((1));
CREATE TABLE calendar_feed(
 sync_id TEXT PRIMARY KEY NOT NULL,
 label TEXT NOT NULL,
 color TEXT NOT NULL DEFAULT '#1f7a70',
 url TEXT NOT NULL,
 sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE trip_permit(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE UNIQUE,
 filename TEXT NOT NULL,
 mime_type TEXT NOT NULL,
 size_bytes INTEGER NOT NULL,
 uploaded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 sha256 TEXT
);
CREATE TABLE trip_day(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE,
 day_number INTEGER NOT NULL,
 distance_mi TEXT,
 elevation_gain_ft INTEGER
);
CREATE TABLE trip_gear(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE,
 gear_item_id TEXT REFERENCES gear_item(sync_id),
 snapshot_name TEXT NOT NULL,
 snapshot_weight_g INTEGER NOT NULL,
 snapshot_category TEXT,
 weight_class TEXT NOT NULL DEFAULT 'base' CHECK (weight_class IN ('base','worn')),
 quantity INTEGER NOT NULL DEFAULT 1,
 packed INTEGER NOT NULL DEFAULT 0 CHECK (packed IN (0,1)),
 sort_order INTEGER NOT NULL DEFAULT 0,
 UNIQUE(trip_id,gear_item_id)
);
CREATE TABLE ingredient(
 sync_id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL,
 kcal_per_100g TEXT,
 density_g_ml TEXT,
 density_source TEXT,
 reference_name TEXT,
 default_serving_g INTEGER,
 category TEXT,
 source TEXT,
 source_id TEXT,
 notes TEXT
);
CREATE TABLE meal(
 sync_id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL,
 base_servings INTEGER NOT NULL DEFAULT 1,
 meal_type TEXT CHECK (meal_type IN ('breakfast','lunch','dinner','snack')),
 is_hot INTEGER NOT NULL DEFAULT 1 CHECK (is_hot IN (0,1)),
 water_ml INTEGER,
 notes TEXT,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE meal_ingredient(
 sync_id TEXT PRIMARY KEY NOT NULL,
 meal_id TEXT NOT NULL REFERENCES meal(sync_id) ON DELETE CASCADE,
 ingredient_id TEXT REFERENCES ingredient(sync_id),
 snapshot_name TEXT NOT NULL,
 snapshot_kcal_per_100g TEXT,
 snapshot_density_g_ml TEXT,
 entered_amount TEXT,
 entered_unit TEXT,
 amount_g INTEGER NOT NULL,
 sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE trip_meal(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE,
 day_number INTEGER NOT NULL,
 meal_type TEXT CHECK (meal_type IN ('breakfast','lunch','dinner','snack')),
 meal_id TEXT REFERENCES meal(sync_id),
 ingredient_id TEXT REFERENCES ingredient(sync_id),
 servings TEXT NOT NULL DEFAULT '1',
 snapshot_name TEXT NOT NULL,
 snapshot_kcal INTEGER NOT NULL,
 snapshot_weight_g INTEGER NOT NULL,
 sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE trip_consumable(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE,
 type TEXT NOT NULL CHECK (type IN ('water','fuel')),
 label TEXT,
 grams INTEGER NOT NULL,
 day_number INTEGER
);
CREATE TABLE trip_gpx(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE UNIQUE,
 filename TEXT,
 start_lat TEXT,
 start_lon TEXT,
 distance_mi TEXT,
 elevation_gain_ft INTEGER,
 min_ele_ft INTEGER,
 max_ele_ft INTEGER,
 track TEXT,
 profile TEXT,
 points TEXT,
 uploaded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE trip_campsite(
 sync_id TEXT PRIMARY KEY NOT NULL,
 trip_id TEXT NOT NULL REFERENCES trip(sync_id) ON DELETE CASCADE,
 distance_mi TEXT NOT NULL,
 lat TEXT,
 lon TEXT,
 ele_ft INTEGER
);
CREATE TABLE outbox(seq INTEGER PRIMARY KEY AUTOINCREMENT,batch_id TEXT NOT NULL,tbl TEXT NOT NULL,sync_id TEXT NOT NULL,op TEXT NOT NULL CHECK(op IN ('upsert','delete')),columns TEXT NOT NULL,base_seq INTEGER,created_at TEXT NOT NULL);
CREATE TABLE row_meta(tbl TEXT,sync_id TEXT,server_seq INTEGER,PRIMARY KEY(tbl,sync_id));
CREATE TABLE sync_state(key TEXT PRIMARY KEY,value TEXT);
CREATE TABLE conflict(id INTEGER PRIMARY KEY,tbl TEXT,sync_id TEXT,col TEXT,local_value TEXT,server_value TEXT,created_at TEXT,dismissed INTEGER DEFAULT 0);
CREATE TABLE blob_ref(sha256 TEXT PRIMARY KEY,state TEXT CHECK(state IN ('present','wanted','upload')),path TEXT);
