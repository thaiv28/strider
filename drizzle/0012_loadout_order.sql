ALTER TABLE "loadout" ADD COLUMN "sort_order" integer DEFAULT 0 NOT NULL;
--> statement-breakpoint
UPDATE "loadout" SET "sort_order" = -"id";
