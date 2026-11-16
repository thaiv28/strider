ALTER TABLE blob_ref RENAME TO blob_ref_old;
CREATE TABLE blob_ref(sha256 TEXT PRIMARY KEY NOT NULL,state TEXT NOT NULL CHECK(state IN ('local_only','uploaded','missing')),path TEXT);
INSERT INTO blob_ref SELECT sha256, CASE state WHEN 'present' THEN 'uploaded' WHEN 'upload' THEN 'local_only' ELSE 'missing' END,path FROM blob_ref_old;
DROP TABLE blob_ref_old;
-- Upload intent is per trip: two trips may use identical content.
CREATE TABLE permit_upload(trip_id TEXT PRIMARY KEY,sha256 TEXT NOT NULL);
-- Server regenerates campsite-derived days with its own identities.
CREATE TABLE local_derived_day(sync_id TEXT PRIMARY KEY,trip_id TEXT NOT NULL,day_number INTEGER NOT NULL,edited INTEGER NOT NULL DEFAULT 0);
CREATE TABLE permit_delete(trip_id TEXT PRIMARY KEY);
