CREATE SEQUENCE "public"."sync_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 9223372036854775807 START WITH 1 CACHE 1;--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "device_auth_code" (
	"code_hash" text PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"code_challenge" text NOT NULL,
	"device_name" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"used_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "device_session" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"device_name" text NOT NULL,
	"token_hash" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_used_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	CONSTRAINT "device_session_token_hash_unique" UNIQUE("token_hash")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "sync_batch" (
	"batch_id" uuid NOT NULL,
	"user_id" integer NOT NULL,
	"result" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "sync_batch_user_id_batch_id_pk" PRIMARY KEY("user_id","batch_id")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "sync_tombstone" (
	"id" bigserial PRIMARY KEY NOT NULL,
	"tbl" text NOT NULL,
	"sync_id" uuid NOT NULL,
	"owner_user_id" integer NOT NULL,
	"trip_id" integer,
	"server_seq" bigint NOT NULL,
	"deleted_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "calendar_feed" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "calendar_feed" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "energy_params" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "energy_params" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "gear_category" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "gear_category" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "gear_component" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "gear_component" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "gear_item" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "gear_item" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "ingredient" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "ingredient" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "loadout" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "loadout" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "loadout_item" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "loadout_item" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "meal" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "meal" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "meal_ingredient" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "meal_ingredient" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "report_settings" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "report_settings" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trail" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trail" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_campsite" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_campsite" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_consumable" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_consumable" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_day" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_day" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_gear" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_gear" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_gpx" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_gpx" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_meal" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_meal" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_permit" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_permit" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "trip_permit" ADD COLUMN "sha256" text;--> statement-breakpoint
ALTER TABLE "trip_share_visit" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "wishlist_loadout" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "wishlist_loadout" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "wishlist_replacement" ADD COLUMN "sync_id" uuid DEFAULT gen_random_uuid() NOT NULL;--> statement-breakpoint
ALTER TABLE "wishlist_replacement" ADD COLUMN "server_seq" bigint DEFAULT 0 NOT NULL;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "device_auth_code" ADD CONSTRAINT "device_auth_code_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "device_session" ADD CONSTRAINT "device_session_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "sync_batch" ADD CONSTRAINT "sync_batch_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sync_tombstone_owner_seq_idx" ON "sync_tombstone" USING btree ("owner_user_id","server_seq");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "sync_tombstone_trip_seq_idx" ON "sync_tombstone" USING btree ("trip_id","server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "calendar_feed_sync_id_uniq" ON "calendar_feed" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "calendar_feed_server_seq_idx" ON "calendar_feed" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "energy_params_sync_id_uniq" ON "energy_params" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "energy_params_server_seq_idx" ON "energy_params" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "gear_category_sync_id_uniq" ON "gear_category" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "gear_category_server_seq_idx" ON "gear_category" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "gear_component_sync_id_uniq" ON "gear_component" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "gear_component_server_seq_idx" ON "gear_component" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "gear_item_sync_id_uniq" ON "gear_item" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "gear_item_server_seq_idx" ON "gear_item" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "ingredient_sync_id_uniq" ON "ingredient" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "ingredient_server_seq_idx" ON "ingredient" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "loadout_sync_id_uniq" ON "loadout" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "loadout_server_seq_idx" ON "loadout" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "loadout_item_sync_id_uniq" ON "loadout_item" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "loadout_item_server_seq_idx" ON "loadout_item" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "meal_sync_id_uniq" ON "meal" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "meal_server_seq_idx" ON "meal" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "meal_ingredient_sync_id_uniq" ON "meal_ingredient" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "meal_ingredient_server_seq_idx" ON "meal_ingredient" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "report_settings_sync_id_uniq" ON "report_settings" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "report_settings_server_seq_idx" ON "report_settings" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trail_sync_id_uniq" ON "trail" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trail_server_seq_idx" ON "trail" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_sync_id_uniq" ON "trip" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_server_seq_idx" ON "trip" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_campsite_sync_id_uniq" ON "trip_campsite" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_campsite_server_seq_idx" ON "trip_campsite" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_consumable_sync_id_uniq" ON "trip_consumable" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_consumable_server_seq_idx" ON "trip_consumable" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_day_sync_id_uniq" ON "trip_day" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_day_server_seq_idx" ON "trip_day" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_gear_sync_id_uniq" ON "trip_gear" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_gear_server_seq_idx" ON "trip_gear" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_gpx_sync_id_uniq" ON "trip_gpx" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_gpx_server_seq_idx" ON "trip_gpx" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_meal_sync_id_uniq" ON "trip_meal" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_meal_server_seq_idx" ON "trip_meal" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "trip_permit_sync_id_uniq" ON "trip_permit" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_permit_server_seq_idx" ON "trip_permit" USING btree ("server_seq");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "trip_share_visit_server_seq_idx" ON "trip_share_visit" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "wishlist_loadout_sync_id_uniq" ON "wishlist_loadout" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "wishlist_loadout_server_seq_idx" ON "wishlist_loadout" USING btree ("server_seq");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "wishlist_replacement_sync_id_uniq" ON "wishlist_replacement" USING btree ("sync_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "wishlist_replacement_server_seq_idx" ON "wishlist_replacement" USING btree ("server_seq");
--> statement-breakpoint
-- Backfill before installing triggers so existing rows receive unique positive cursors.
-- The same transaction-scoped lock is used by all synced writes and tombstones.
SELECT pg_advisory_xact_lock(7645912384021);
--> statement-breakpoint
UPDATE "gear_category" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "gear_item" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "gear_component" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "wishlist_replacement" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "wishlist_loadout" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "loadout" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "loadout_item" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trail" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_day" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_gear" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_campsite" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_gpx" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_permit" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_meal" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_consumable" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "ingredient" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "meal" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "meal_ingredient" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "calendar_feed" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "energy_params" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "report_settings" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE "trip_share_visit" SET "server_seq" = nextval('sync_seq');
--> statement-breakpoint
UPDATE trip_permit SET sha256 = encode(sha256(data), 'hex');
--> statement-breakpoint
CREATE FUNCTION sync_assign_server_seq() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'UPDATE' THEN
    IF (to_jsonb(NEW) - 'server_seq') IS NOT DISTINCT FROM (to_jsonb(OLD) - 'server_seq') THEN
      NEW.server_seq := OLD.server_seq;
      RETURN NEW;
    END IF;
  END IF;
  PERFORM pg_advisory_xact_lock(7645912384021);
  NEW.server_seq := nextval('sync_seq');
  RETURN NEW;
END;
$$;
--> statement-breakpoint
CREATE FUNCTION sync_record_delete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
  old_row jsonb := to_jsonb(OLD);
  row_owner integer;
  row_trip integer;
  row_sync uuid;
BEGIN
  IF TG_TABLE_NAME = 'trip_share_visit' THEN
    row_owner := OLD.user_id;
    row_trip := OLD.trip_id;
    row_sync := gen_random_uuid();
  ELSE
    row_sync := (old_row ->> 'sync_id')::uuid;
    IF old_row ? 'user_id' THEN
      row_owner := (old_row ->> 'user_id')::integer;
      IF TG_TABLE_NAME = 'trip' THEN
        row_trip := (old_row ->> 'id')::integer;
      END IF;
    ELSIF TG_TABLE_NAME = 'gear_component' THEN
      SELECT user_id INTO row_owner FROM gear_item WHERE id = (old_row ->> 'gear_item_id')::integer;
    ELSIF TG_TABLE_NAME = 'wishlist_replacement' THEN
      SELECT user_id INTO row_owner FROM gear_item
        WHERE id = (old_row ->> 'wishlist_item_id')::integer
          AND EXISTS (SELECT 1 FROM gear_item WHERE id = (old_row ->> 'replaces_item_id')::integer);
    ELSIF TG_TABLE_NAME = 'wishlist_loadout' THEN
      SELECT user_id INTO row_owner FROM gear_item
        WHERE id = (old_row ->> 'wishlist_item_id')::integer
          AND EXISTS (SELECT 1 FROM loadout WHERE id = (old_row ->> 'loadout_id')::integer);
    ELSIF TG_TABLE_NAME = 'loadout_item' THEN
      SELECT user_id INTO row_owner FROM loadout
        WHERE id = (old_row ->> 'loadout_id')::integer
          AND EXISTS (SELECT 1 FROM gear_item WHERE id = (old_row ->> 'gear_item_id')::integer);
    ELSIF TG_TABLE_NAME = 'meal_ingredient' THEN
      SELECT user_id INTO row_owner FROM meal WHERE id = (old_row ->> 'meal_id')::integer;
    ELSE
      row_trip := (old_row ->> 'trip_id')::integer;
      SELECT user_id INTO row_owner FROM trip WHERE id = row_trip;
    END IF;
  END IF;
  -- A missing parent means a cascade; the device cascades from its parent's tombstone.
  IF row_owner IS NULL THEN RETURN OLD; END IF;
  PERFORM pg_advisory_xact_lock(7645912384021);
  INSERT INTO sync_tombstone (tbl, sync_id, owner_user_id, trip_id, server_seq)
    VALUES (TG_TABLE_NAME, row_sync, row_owner, row_trip, nextval('sync_seq'));
  RETURN OLD;
END;
$$;
--> statement-breakpoint
CREATE TRIGGER "gear_category_sync_seq" BEFORE INSERT OR UPDATE ON "gear_category"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "gear_category_sync_delete" AFTER DELETE ON "gear_category"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "gear_item_sync_seq" BEFORE INSERT OR UPDATE ON "gear_item"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "gear_item_sync_delete" AFTER DELETE ON "gear_item"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "gear_component_sync_seq" BEFORE INSERT OR UPDATE ON "gear_component"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "gear_component_sync_delete" AFTER DELETE ON "gear_component"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "wishlist_replacement_sync_seq" BEFORE INSERT OR UPDATE ON "wishlist_replacement"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "wishlist_replacement_sync_delete" AFTER DELETE ON "wishlist_replacement"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "wishlist_loadout_sync_seq" BEFORE INSERT OR UPDATE ON "wishlist_loadout"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "wishlist_loadout_sync_delete" AFTER DELETE ON "wishlist_loadout"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "loadout_sync_seq" BEFORE INSERT OR UPDATE ON "loadout"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "loadout_sync_delete" AFTER DELETE ON "loadout"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "loadout_item_sync_seq" BEFORE INSERT OR UPDATE ON "loadout_item"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "loadout_item_sync_delete" AFTER DELETE ON "loadout_item"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trail_sync_seq" BEFORE INSERT OR UPDATE ON "trail"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trail_sync_delete" AFTER DELETE ON "trail"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_sync_seq" BEFORE INSERT OR UPDATE ON "trip"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_sync_delete" AFTER DELETE ON "trip"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_day_sync_seq" BEFORE INSERT OR UPDATE ON "trip_day"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_day_sync_delete" AFTER DELETE ON "trip_day"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_gear_sync_seq" BEFORE INSERT OR UPDATE ON "trip_gear"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_gear_sync_delete" AFTER DELETE ON "trip_gear"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_campsite_sync_seq" BEFORE INSERT OR UPDATE ON "trip_campsite"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_campsite_sync_delete" AFTER DELETE ON "trip_campsite"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_gpx_sync_seq" BEFORE INSERT OR UPDATE ON "trip_gpx"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_gpx_sync_delete" AFTER DELETE ON "trip_gpx"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_permit_sync_seq" BEFORE INSERT OR UPDATE ON "trip_permit"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_permit_sync_delete" AFTER DELETE ON "trip_permit"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_meal_sync_seq" BEFORE INSERT OR UPDATE ON "trip_meal"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_meal_sync_delete" AFTER DELETE ON "trip_meal"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_consumable_sync_seq" BEFORE INSERT OR UPDATE ON "trip_consumable"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_consumable_sync_delete" AFTER DELETE ON "trip_consumable"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "ingredient_sync_seq" BEFORE INSERT OR UPDATE ON "ingredient"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "ingredient_sync_delete" AFTER DELETE ON "ingredient"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "meal_sync_seq" BEFORE INSERT OR UPDATE ON "meal"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "meal_sync_delete" AFTER DELETE ON "meal"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "meal_ingredient_sync_seq" BEFORE INSERT OR UPDATE ON "meal_ingredient"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "meal_ingredient_sync_delete" AFTER DELETE ON "meal_ingredient"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "calendar_feed_sync_seq" BEFORE INSERT OR UPDATE ON "calendar_feed"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "calendar_feed_sync_delete" AFTER DELETE ON "calendar_feed"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "energy_params_sync_seq" BEFORE INSERT OR UPDATE ON "energy_params"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "energy_params_sync_delete" AFTER DELETE ON "energy_params"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "report_settings_sync_seq" BEFORE INSERT OR UPDATE ON "report_settings"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "report_settings_sync_delete" AFTER DELETE ON "report_settings"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
--> statement-breakpoint
CREATE TRIGGER "trip_share_visit_sync_seq" BEFORE INSERT OR UPDATE ON "trip_share_visit"
  FOR EACH ROW EXECUTE FUNCTION sync_assign_server_seq();
--> statement-breakpoint
CREATE TRIGGER "trip_share_visit_sync_delete" AFTER DELETE ON "trip_share_visit"
  FOR EACH ROW EXECUTE FUNCTION sync_record_delete();
