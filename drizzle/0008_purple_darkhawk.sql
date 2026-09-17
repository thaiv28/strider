CREATE TABLE IF NOT EXISTS "trip_campsite" (
	"id" serial PRIMARY KEY NOT NULL,
	"trip_id" integer NOT NULL,
	"distance_mi" numeric NOT NULL,
	"lat" numeric,
	"lon" numeric,
	"ele_ft" integer
);
--> statement-breakpoint
ALTER TABLE "trip_gpx" ADD COLUMN "points" jsonb;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "trip_campsite" ADD CONSTRAINT "trip_campsite_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
