//! Device-local caches (weather, calendar feeds, Overpass) and offline map region state.
//! These tables are never synced and never enter the outbox or undo history.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheKind {
    Weather,
    Calendar,
    Overpass,
}
impl CacheKind {
    fn table(self) -> &'static str {
        match self {
            Self::Weather => "weather_cache",
            Self::Calendar => "calendar_cache",
            Self::Overpass => "overpass_cache",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheEntry {
    /// Unix seconds.
    pub fetched_at: i64,
    pub body: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MapRegionRecord {
    pub trip_sync_id: String,
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub tiles_total: u64,
    pub tiles_done: u64,
    pub bytes: u64,
    /// pending, downloading, complete, cancelled or failed.
    pub state: String,
    pub updated_at: i64,
}
impl Store {
    pub fn cache_get(&self, kind: CacheKind, key: &str) -> Result<Option<CacheEntry>> {
        Ok(self
            .connection
            .query_row(
                &format!("SELECT fetched_at, body FROM {} WHERE key=?", kind.table()),
                [key],
                |r| {
                    Ok(CacheEntry {
                        fetched_at: r.get(0)?,
                        body: r.get(1)?,
                    })
                },
            )
            .optional()?)
    }
    pub fn cache_put(&self, kind: CacheKind, key: &str, fetched_at: i64, body: &str) -> Result<()> {
        self.connection.execute(
            &format!(
                "INSERT INTO {} (key, fetched_at, body) VALUES (?1, ?2, ?3) \
                 ON CONFLICT(key) DO UPDATE SET fetched_at=excluded.fetched_at, body=excluded.body",
                kind.table()
            ),
            params![key, fetched_at, body],
        )?;
        Ok(())
    }
    pub fn cache_delete(&self, kind: CacheKind, key: &str) -> Result<()> {
        self.connection
            .execute(&format!("DELETE FROM {} WHERE key=?", kind.table()), [key])?;
        Ok(())
    }
    pub fn save_map_region(&self, r: &MapRegionRecord) -> Result<()> {
        let state_ok = matches!(
            r.state.as_str(),
            "pending" | "downloading" | "complete" | "cancelled" | "failed"
        );
        if !state_ok {
            return invalid("Unknown map region state");
        }
        self.connection.execute(
            "INSERT INTO map_region (trip_sync_id, south, west, north, east, min_zoom, max_zoom, \
             tiles_total, tiles_done, bytes, state, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) \
             ON CONFLICT(trip_sync_id) DO UPDATE SET south=excluded.south, west=excluded.west, \
             north=excluded.north, east=excluded.east, min_zoom=excluded.min_zoom, \
             max_zoom=excluded.max_zoom, tiles_total=excluded.tiles_total, \
             tiles_done=excluded.tiles_done, bytes=excluded.bytes, state=excluded.state, \
             updated_at=excluded.updated_at",
            params![
                r.trip_sync_id,
                r.south,
                r.west,
                r.north,
                r.east,
                r.min_zoom,
                r.max_zoom,
                r.tiles_total as i64,
                r.tiles_done as i64,
                r.bytes as i64,
                r.state,
                r.updated_at
            ],
        )?;
        Ok(())
    }
    pub fn map_regions(&self) -> Result<Vec<MapRegionRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT trip_sync_id, south, west, north, east, min_zoom, max_zoom, tiles_total, \
             tiles_done, bytes, state, updated_at FROM map_region ORDER BY trip_sync_id",
        )?;
        let out = stmt
            .query_map([], |r| {
                Ok(MapRegionRecord {
                    trip_sync_id: r.get(0)?,
                    south: r.get(1)?,
                    west: r.get(2)?,
                    north: r.get(3)?,
                    east: r.get(4)?,
                    min_zoom: r.get(5)?,
                    max_zoom: r.get(6)?,
                    tiles_total: r.get::<_, i64>(7)?.max(0) as u64,
                    tiles_done: r.get::<_, i64>(8)?.max(0) as u64,
                    bytes: r.get::<_, i64>(9)?.max(0) as u64,
                    state: r.get(10)?,
                    updated_at: r.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(out)
    }
    pub fn map_region(&self, trip: &str) -> Result<Option<MapRegionRecord>> {
        Ok(self
            .map_regions()?
            .into_iter()
            .find(|r| r.trip_sync_id == trip))
    }
    pub fn delete_map_region(&self, trip: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM map_region WHERE trip_sync_id=?", [trip])?;
        Ok(())
    }
    /// The Trip's GPX route as `[lat, lon]` pairs, or empty without a route.
    pub fn trip_track(&self, trip: &str) -> Result<Vec<[f64; 2]>> {
        let raw: Option<Option<String>> = self
            .connection
            .query_row("SELECT track FROM trip_gpx WHERE trip_id=?", [trip], |r| {
                r.get(0)
            })
            .optional()?;
        match raw.flatten() {
            Some(raw) => {
                serde_json::from_str(&raw).map_err(|_| Error::Invalid("Invalid GPX track".into()))
            }
            None => Ok(vec![]),
        }
    }
}
