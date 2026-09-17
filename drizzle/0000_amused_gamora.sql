CREATE TYPE "public"."consumable_type" AS ENUM('water', 'fuel');--> statement-breakpoint
CREATE TYPE "public"."file_kind" AS ENUM('gpx', 'image', 'doc');--> statement-breakpoint
CREATE TYPE "public"."gear_status" AS ENUM('current', 'retired', 'wishlist');--> statement-breakpoint
CREATE TYPE "public"."meal_type" AS ENUM('breakfast', 'lunch', 'dinner', 'snack');--> statement-breakpoint
CREATE TYPE "public"."pack_class" AS ENUM('base', 'worn');--> statement-breakpoint
CREATE TYPE "public"."trip_status" AS ENUM('idea', 'planned', 'completed');--> statement-breakpoint
CREATE TYPE "public"."weight_class" AS ENUM('base', 'worn', 'consumable');--> statement-breakpoint
CREATE TYPE "public"."wishlist_priority" AS ENUM('purchased', 'high', 'would_be_nice');--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "energy_params" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"bmr" integer DEFAULT 1735 NOT NULL,
	"cal_per_energy_mile" integer DEFAULT 200 NOT NULL,
	"ft_per_energy_mile" integer DEFAULT 625 NOT NULL,
	CONSTRAINT "energy_params_user_id_unique" UNIQUE("user_id")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "file" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"trip_id" integer,
	"kind" "file_kind" NOT NULL,
	"filename" text NOT NULL,
	"storage_url" text NOT NULL,
	"uploaded_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "food_item" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"name" text NOT NULL,
	"calories_per_100g" numeric,
	"default_serving_g" integer,
	"category" text,
	"notes" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "gear_category" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"name" text NOT NULL,
	"parent_id" integer,
	"weight_class" "weight_class",
	"sort_order" integer DEFAULT 0 NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "gear_component" (
	"id" serial PRIMARY KEY NOT NULL,
	"gear_item_id" integer NOT NULL,
	"name" text NOT NULL,
	"weight_g" integer NOT NULL,
	"quantity" integer DEFAULT 1 NOT NULL,
	"notes" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "gear_item" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"category_id" integer,
	"name" text NOT NULL,
	"weight_g" integer,
	"default_weight_class" "weight_class" DEFAULT 'base' NOT NULL,
	"status" "gear_status" DEFAULT 'current' NOT NULL,
	"wishlist_priority" "wishlist_priority",
	"is_kit" boolean DEFAULT false NOT NULL,
	"product_url" text,
	"notes" text,
	"acquired_at" date,
	"retired_at" date,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "trail" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"name" text NOT NULL,
	"region" text,
	"area_type" text,
	"typical_distance_mi" numeric,
	"typical_elevation_ft" integer,
	"notes" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "trip" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"trail_id" integer,
	"name" text NOT NULL,
	"status" "trip_status" DEFAULT 'planned' NOT NULL,
	"start_date" date,
	"nights" integer,
	"trailhead" text,
	"permit_required" boolean,
	"permit_notes" text,
	"driving_notes" text,
	"water_sources" text,
	"companions" text,
	"planning_notes" text,
	"trip_report" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "trip_consumable" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL,
	"type" "consumable_type" NOT NULL,
	"label" text,
	"grams" integer NOT NULL,
	"day_number" integer
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "trip_day" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL,
	"day_number" integer NOT NULL,
	"distance_mi" numeric,
	"elevation_gain_ft" integer
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "trip_food" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL,
	"food_item_id" integer,
	"snapshot_name" text NOT NULL,
	"snapshot_cal_per_100g" numeric,
	"weight_g" integer NOT NULL,
	"day_number" integer,
	"meal_type" "meal_type"
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "trip_gear" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL,
	"gear_item_id" integer,
	"snapshot_name" text NOT NULL,
	"snapshot_weight_g" integer NOT NULL,
	"snapshot_category" text,
	"weight_class" "pack_class" DEFAULT 'base' NOT NULL,
	"quantity" integer DEFAULT 1 NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "users" (
	"id" serial PRIMARY KEY NOT NULL,
	"email" text NOT NULL,
	"name" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "users_email_unique" UNIQUE("email")
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "energy_params" ADD CONSTRAINT "energy_params_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "file" ADD CONSTRAINT "file_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "file" ADD CONSTRAINT "file_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "food_item" ADD CONSTRAINT "food_item_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "gear_category" ADD CONSTRAINT "gear_category_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "gear_category" ADD CONSTRAINT "gear_category_parent_id_gear_category_id_fk" FOREIGN KEY ("parent_id") REFERENCES "public"."gear_category"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "gear_component" ADD CONSTRAINT "gear_component_gear_item_id_gear_item_id_fk" FOREIGN KEY ("gear_item_id") REFERENCES "public"."gear_item"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "gear_item" ADD CONSTRAINT "gear_item_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "gear_item" ADD CONSTRAINT "gear_item_category_id_gear_category_id_fk" FOREIGN KEY ("category_id") REFERENCES "public"."gear_category"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trail" ADD CONSTRAINT "trail_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip" ADD CONSTRAINT "trip_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip" ADD CONSTRAINT "trip_trail_id_trail_id_fk" FOREIGN KEY ("trail_id") REFERENCES "public"."trail"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_consumable" ADD CONSTRAINT "trip_consumable_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_day" ADD CONSTRAINT "trip_day_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_food" ADD CONSTRAINT "trip_food_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_food" ADD CONSTRAINT "trip_food_food_item_id_food_item_id_fk" FOREIGN KEY ("food_item_id") REFERENCES "public"."food_item"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_gear" ADD CONSTRAINT "trip_gear_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_gear" ADD CONSTRAINT "trip_gear_gear_item_id_gear_item_id_fk" FOREIGN KEY ("gear_item_id") REFERENCES "public"."gear_item"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_gear_trip_item_uniq" ON "trip_gear" USING btree ("trip_id","gear_item_id");