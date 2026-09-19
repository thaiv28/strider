ALTER TABLE "trip" ADD COLUMN "share_token" text;
CREATE UNIQUE INDEX "trip_share_token_unique" ON "trip" USING btree ("share_token");
