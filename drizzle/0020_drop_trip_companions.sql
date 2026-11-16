-- Custom SQL migration file, put your code below! --
-- trip.companions was created in 0000 and later removed from src/db/schema.ts without a
-- migration, so databases still carry it. Nothing reads or writes it; drop it so the
-- database matches the schema. IF EXISTS keeps fresh databases (and reruns) safe.
ALTER TABLE "trip" DROP COLUMN IF EXISTS "companions";
