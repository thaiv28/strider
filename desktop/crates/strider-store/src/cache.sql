-- Local-only caches and offline-map state. None of these tables is synced or enters the outbox.
CREATE TABLE weather_cache(
 key TEXT PRIMARY KEY NOT NULL,
 fetched_at INTEGER NOT NULL,
 body TEXT NOT NULL
);
CREATE TABLE calendar_cache(
 key TEXT PRIMARY KEY NOT NULL,
 fetched_at INTEGER NOT NULL,
 body TEXT NOT NULL
);
CREATE TABLE overpass_cache(
 key TEXT PRIMARY KEY NOT NULL,
 fetched_at INTEGER NOT NULL,
 body TEXT NOT NULL
);
CREATE TABLE map_region(
 trip_sync_id TEXT PRIMARY KEY NOT NULL,
 south REAL NOT NULL,
 west REAL NOT NULL,
 north REAL NOT NULL,
 east REAL NOT NULL,
 min_zoom INTEGER NOT NULL,
 max_zoom INTEGER NOT NULL,
 tiles_total INTEGER NOT NULL DEFAULT 0,
 tiles_done INTEGER NOT NULL DEFAULT 0,
 bytes INTEGER NOT NULL DEFAULT 0,
 state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','downloading','complete','cancelled','failed')),
 updated_at INTEGER NOT NULL DEFAULT 0
);
