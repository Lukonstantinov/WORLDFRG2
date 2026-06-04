pub mod schema;
pub mod tile_store;
pub mod metadata;
pub mod world_cache;

use rusqlite::Connection;
use std::any::Any;
use std::sync::{Arc, Mutex};
use world_cache::WorldTiles;

/// Cheap fingerprint of the tile table: (sum of versions, row count). Every tile
/// write does `INSERT OR REPLACE … version+1`, so this value changes on any edit
/// — letting the decompressed-tile cache invalidate itself without manual bumps.
pub type TileFingerprint = (i64, i64);

/// Identity of a memoized coarse cost grid: (fingerprint.0, fingerprint.1,
/// block_sea, rivers-json hash). Computed in `query_commands`; stored opaquely
/// here so `db` need not know the `CoarseCost` type.
pub type CostKey = (i64, i64, bool, u64);

pub struct WorldDb {
    pub conn: Mutex<Connection>,
    /// Decompressed-tile snapshot shared by overlay/query commands, with the
    /// fingerprint it was built from.
    tiles_cache: Mutex<Option<(TileFingerprint, Arc<WorldTiles>)>>,
    /// Last coarse cost grid (for trade routes / matrix / political), kept opaque
    /// (`dyn Any`) so the three commands that share it don't each rebuild it.
    cost_cache: Mutex<Option<(CostKey, Arc<dyn Any + Send + Sync>)>>,
}

impl WorldDb {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            tiles_cache: Mutex::new(None),
            cost_cache: Mutex::new(None),
        }
    }

    pub fn open_or_create(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| e.to_string())?;
        schema::create_tables(&conn).map_err(|e| e.to_string())?;
        Ok(Self::new(conn))
    }

    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        schema::create_tables(&conn).map_err(|e| e.to_string())?;
        Ok(Self::new(conn))
    }

    fn fingerprint(conn: &Connection) -> Result<TileFingerprint, String> {
        conn.query_row(
            "SELECT COALESCE(SUM(version), 0), COUNT(*) FROM tiles",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).map_err(|e| e.to_string())
    }

    /// Return the decompressed-tile snapshot, rebuilding it only when the tile
    /// data has changed. The caller must already hold the `conn` lock (every
    /// query command does), so this never re-locks `conn` — it only fingerprints
    /// and, on a miss, decompresses.
    pub fn cached_tiles_with_conn(&self, conn: &Connection) -> Result<Arc<WorldTiles>, String> {
        let fp = Self::fingerprint(conn)?;
        {
            let cache = self.tiles_cache.lock().map_err(|e| e.to_string())?;
            if let Some((cfp, arc)) = cache.as_ref() {
                if *cfp == fp { return Ok(arc.clone()); }
            }
        }
        let mut world = WorldTiles::load(conn)?;
        world.fingerprint = fp;
        let world = Arc::new(world);
        *self.tiles_cache.lock().map_err(|e| e.to_string())? = Some((fp, world.clone()));
        Ok(world)
    }

    /// Fetch the memoized coarse cost grid for `key`, if present (caller downcasts
    /// the `dyn Any` back to its `CoarseCost`).
    pub fn cost_cache_get(&self, key: &CostKey) -> Option<Arc<dyn Any + Send + Sync>> {
        let cache = self.cost_cache.lock().ok()?;
        let (k, v) = cache.as_ref()?;
        if k == key { Some(v.clone()) } else { None }
    }

    /// Store the coarse cost grid under `key`, replacing any previous one (only
    /// the latest is kept — callers in a fan-out reuse it within the same world).
    pub fn cost_cache_put(&self, key: CostKey, val: Arc<dyn Any + Send + Sync>) {
        if let Ok(mut cache) = self.cost_cache.lock() {
            *cache = Some((key, val));
        }
    }
}
