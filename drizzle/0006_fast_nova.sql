CREATE TABLE IF NOT EXISTS "trip_gpx" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL,
	"filename" text,
	"start_lat" numeric,
	"start_lon" numeric,
	"distance_mi" numeric,
	"elevation_gain_ft" integer,
	"min_ele_ft" integer,
	"max_ele_ft" integer,
	"track" jsonb,
	"profile" jsonb,
	"uploaded_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "trip_gpx_trip_id_unique" UNIQUE("trip_id")
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_gpx" ADD CONSTRAINT "trip_gpx_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
