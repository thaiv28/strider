-- Older production databases received these additions through the original
-- seed/restore path, but a brand-new database only had migrations 0000-0010.
-- Keep every statement additive/idempotent so this is safe for both paths.

ALTER TABLE "energy_params" ADD COLUMN IF NOT EXISTS "sex" text;--> statement-breakpoint
ALTER TABLE "energy_params" ADD COLUMN IF NOT EXISTS "weight_lb" numeric;--> statement-breakpoint
ALTER TABLE "energy_params" ADD COLUMN IF NOT EXISTS "height_in" numeric;--> statement-breakpoint
ALTER TABLE "energy_params" ADD COLUMN IF NOT EXISTS "age_years" integer;--> statement-breakpoint
ALTER TABLE "gear_item" ADD COLUMN IF NOT EXISTS "price_cents" integer;--> statement-breakpoint

ALTER TABLE "trip" ADD COLUMN IF NOT EXISTS "party_size" integer DEFAULT 1 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN IF NOT EXISTS "grocery_people" integer;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN IF NOT EXISTS "grocery_removed" jsonb;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN IF NOT EXISTS "packing_list" text;--> statement-breakpoint
ALTER TABLE "trip" ALTER COLUMN "water_g_per_day" SET DEFAULT 964;--> statement-breakpoint
ALTER TABLE "trip" ALTER COLUMN "fuel_g_per_day" SET DEFAULT 201;--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "wishlist_replacement" (
	"id" serial PRIMARY KEY NOT NULL,
	"wishlist_item_id" integer NOT NULL REFERENCES "gear_item"("id") ON DELETE cascade,
	"replaces_item_id" integer NOT NULL REFERENCES "gear_item"("id") ON DELETE cascade
);--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "wishlist_replacement_uniq" ON "wishlist_replacement" USING btree ("wishlist_item_id","replaces_item_id");--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "wishlist_loadout" (
	"id" serial PRIMARY KEY NOT NULL,
	"wishlist_item_id" integer NOT NULL REFERENCES "gear_item"("id") ON DELETE cascade,
	"loadout_id" integer NOT NULL REFERENCES "loadout"("id") ON DELETE cascade,
	"included" boolean NOT NULL
);--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "wishlist_loadout_uniq" ON "wishlist_loadout" USING btree ("wishlist_item_id","loadout_id");--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "report_settings" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL REFERENCES "users"("id"),
	"template" text,
	"packing_default" text,
	CONSTRAINT "report_settings_user_id_unique" UNIQUE("user_id")
);--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "calendar_feed" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL REFERENCES "users"("id"),
	"label" text NOT NULL,
	"color" text DEFAULT '#1f7a70' NOT NULL,
	"url" text NOT NULL,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "trip_permit" (
	"trip_id" integer PRIMARY KEY NOT NULL REFERENCES "trip"("id") ON DELETE cascade,
	"filename" text NOT NULL,
	"mime_type" text NOT NULL,
	"size_bytes" integer NOT NULL,
	"data" bytea NOT NULL,
	"uploaded_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "ingredient" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL REFERENCES "users"("id"),
	"name" text NOT NULL,
	"kcal_per_100g" numeric,
	"density_g_ml" numeric,
	"default_serving_g" integer,
	"category" text,
	"source" text,
	"source_id" text,
	"notes" text
);--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "meal" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL REFERENCES "users"("id"),
	"name" text NOT NULL,
	"base_servings" integer DEFAULT 1 NOT NULL,
	"meal_type" "meal_type",
	"is_hot" boolean DEFAULT true NOT NULL,
	"water_ml" integer,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "meal_ingredient" (
	"id" serial PRIMARY KEY NOT NULL,
	"meal_id" integer NOT NULL REFERENCES "meal"("id") ON DELETE cascade,
	"ingredient_id" integer REFERENCES "ingredient"("id"),
	"snapshot_name" text NOT NULL,
	"snapshot_kcal_per_100g" numeric,
	"snapshot_density_g_ml" numeric,
	"amount_g" integer NOT NULL,
	"sort_order" integer DEFAULT 0 NOT NULL
);--> statement-breakpoint

CREATE TABLE IF NOT EXISTS "trip_meal" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL REFERENCES "trip"("id") ON DELETE cascade,
	"day_number" integer NOT NULL,
	"meal_type" "meal_type",
	"meal_id" integer REFERENCES "meal"("id"),
	"ingredient_id" integer REFERENCES "ingredient"("id"),
	"servings" numeric DEFAULT 1 NOT NULL,
	"snapshot_name" text NOT NULL,
	"snapshot_kcal" integer NOT NULL,
	"snapshot_weight_g" integer NOT NULL,
	"sort_order" integer DEFAULT 0 NOT NULL
);
