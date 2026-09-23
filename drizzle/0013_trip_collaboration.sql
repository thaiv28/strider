ALTER TABLE "trip" ADD COLUMN "edit_token" text;--> statement-breakpoint
CREATE UNIQUE INDEX "trip_edit_token_unique" ON "trip" USING btree ("edit_token");--> statement-breakpoint
CREATE TABLE "trip_share_visit" (
	"trip_id" integer NOT NULL,
	"user_id" integer NOT NULL,
	"permission" text NOT NULL,
	"visited_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "trip_share_visit_permission_check" CHECK ("permission" IN ('view', 'edit'))
);--> statement-breakpoint
ALTER TABLE "trip_share_visit" ADD CONSTRAINT "trip_share_visit_trip_id_trip_id_fk" FOREIGN KEY ("trip_id") REFERENCES "public"."trip"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "trip_share_visit" ADD CONSTRAINT "trip_share_visit_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "trip_share_visit_uniq" ON "trip_share_visit" USING btree ("trip_id","user_id","permission");
