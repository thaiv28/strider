CREATE TABLE IF NOT EXISTS "loadout" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"name" text NOT NULL,
	"is_default" boolean DEFAULT false NOT NULL,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "loadout_item" (
	"id" serial PRIMARY KEY NOT NULL,
	"loadout_id" integer NOT NULL,
	"gear_item_id" integer NOT NULL,
	"weight_class" "pack_class" DEFAULT 'base' NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "loadout" ADD CONSTRAINT "loadout_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "loadout_item" ADD CONSTRAINT "loadout_item_loadout_id_loadout_id_fk" FOREIGN KEY ("loadout_id") REFERENCES "public"."loadout"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "loadout_item" ADD CONSTRAINT "loadout_item_gear_item_id_gear_item_id_fk" FOREIGN KEY ("gear_item_id") REFERENCES "public"."gear_item"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "loadout_item_uniq" ON "loadout_item" USING btree ("loadout_id","gear_item_id");