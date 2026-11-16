ALTER TABLE row_meta ADD COLUMN base_values TEXT;
CREATE TABLE sync_log(id INTEGER PRIMARY KEY, at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), kind TEXT NOT NULL, detail TEXT NOT NULL);
-- Retain trip scope even after a queued delete removes the local row.
ALTER TABLE outbox ADD COLUMN trip_id TEXT;
UPDATE outbox SET trip_id=sync_id WHERE tbl='trip';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_day WHERE sync_id=outbox.sync_id) WHERE tbl='trip_day';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_gear WHERE sync_id=outbox.sync_id) WHERE tbl='trip_gear';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_meal WHERE sync_id=outbox.sync_id) WHERE tbl='trip_meal';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_consumable WHERE sync_id=outbox.sync_id) WHERE tbl='trip_consumable';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_gpx WHERE sync_id=outbox.sync_id) WHERE tbl='trip_gpx';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_campsite WHERE sync_id=outbox.sync_id) WHERE tbl='trip_campsite';
UPDATE outbox SET trip_id=(SELECT trip_id FROM trip_permit WHERE sync_id=outbox.sync_id) WHERE tbl='trip_permit';
