ALTER TABLE "trip" ADD COLUMN "distance_mi" numeric;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN "elevation_gain_ft" integer;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN "food_g_per_day" integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN "water_g_per_day" integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN "fuel_g_per_day" integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_gear" ADD COLUMN "packed" boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_gear" ADD COLUMN "sort_order" integer DEFAULT 0 NOT NULL;