ALTER TABLE "users" ADD COLUMN "google_subject" text;
ALTER TABLE "users" ADD COLUMN "image_url" text;
ALTER TABLE "users" ADD CONSTRAINT "users_google_subject_unique" UNIQUE("google_subject");
