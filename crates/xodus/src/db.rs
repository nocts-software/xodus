use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedCatalogProduct {
    pub product_id: String,
    pub title: String,
    pub developer: String,
    pub publisher: String,
    pub description: String,
    pub poster_url: Option<String>,
    pub hero_url: Option<String>,
    pub package_family_name: Option<String>,
    pub content_id: Option<String>,
    pub size_in_bytes: Option<u64>,
    pub raw_json: Option<String>,
    pub updated_at: u64,
    pub ttl: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedEntitlement {
    pub xuid: String,
    pub product_id: String,
    pub sku_id: Option<String>,
    pub title: Option<String>,
    pub entitlement_type: String,
    pub acquired_date: Option<String>,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedUserProfile {
    pub xuid: String,
    pub gamertag: String,
    pub display_pic_url: Option<String>,
    pub gamer_score: Option<String>,
    pub presence_state: Option<String>,
    pub presence_title: Option<String>,
    pub has_gamepass: bool,
    pub subscription_tier: Option<String>,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedFriend {
    pub xuid: String,
    pub friend_xuid: String,
    pub gamertag: String,
    pub display_pic_url: Option<String>,
    pub presence_state: String,
    pub presence_title: Option<String>,
    pub updated_at: u64,
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl Database {
    pub fn default_db_path() -> PathBuf {
        let base_dir = std::env::var_os("XDG_DATA_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|s| !s.is_empty())
                    .map(|h| PathBuf::from(h).join(".local").join("share"))
            })
            .or_else(|| std::env::var_os("USERPROFILE").filter(|s| !s.is_empty()).map(PathBuf::from))
            .unwrap_or_else(std::env::temp_dir);

        base_dir.join("xodus").join("xodus_cache.db")
    }

    pub fn open_default() -> SqlResult<Self> {
        let path = Self::default_db_path();
        Self::open(&path)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&path_buf)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path_buf,
        };
        db.init_tables()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS catalog_cache (
                product_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                developer TEXT NOT NULL DEFAULT '',
                publisher TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                poster_url TEXT,
                hero_url TEXT,
                package_family_name TEXT,
                content_id TEXT,
                size_in_bytes INTEGER,
                raw_json TEXT,
                updated_at INTEGER NOT NULL,
                ttl INTEGER NOT NULL DEFAULT 604800
            );

            CREATE TABLE IF NOT EXISTS user_entitlements (
                xuid TEXT NOT NULL,
                product_id TEXT NOT NULL,
                sku_id TEXT,
                title TEXT,
                entitlement_type TEXT NOT NULL DEFAULT 'owned',
                acquired_date TEXT,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (xuid, product_id)
            );

            CREATE TABLE IF NOT EXISTS user_profiles (
                xuid TEXT PRIMARY KEY,
                gamertag TEXT NOT NULL,
                display_pic_url TEXT,
                gamer_score TEXT,
                presence_state TEXT,
                presence_title TEXT,
                has_gamepass INTEGER NOT NULL DEFAULT 0,
                subscription_tier TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS social_friends (
                xuid TEXT NOT NULL,
                friend_xuid TEXT NOT NULL,
                gamertag TEXT NOT NULL,
                display_pic_url TEXT,
                presence_state TEXT NOT NULL DEFAULT 'Offline',
                presence_title TEXT,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (xuid, friend_xuid)
            );

            CREATE TABLE IF NOT EXISTS kv_cache (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                expires_at INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    // --- Catalog Methods ---
    pub fn get_catalog_product(&self, product_id: &str) -> SqlResult<Option<CachedCatalogProduct>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT product_id, title, developer, publisher, description, poster_url, hero_url, 
                    package_family_name, content_id, size_in_bytes, raw_json, updated_at, ttl 
             FROM catalog_cache WHERE product_id = ?1",
        )?;

        let mut rows = stmt.query(params![product_id])?;
        if let Some(row) = rows.next()? {
            let updated_at: i64 = row.get(11)?;
            let ttl: i64 = row.get(12)?;
            let now = Self::now_secs();
            // Check if expired
            if now > updated_at + ttl {
                return Ok(None);
            }

            let size_bytes: Option<i64> = row.get(9)?;

            Ok(Some(CachedCatalogProduct {
                product_id: row.get(0)?,
                title: row.get(1)?,
                developer: row.get(2)?,
                publisher: row.get(3)?,
                description: row.get(4)?,
                poster_url: row.get(5)?,
                hero_url: row.get(6)?,
                package_family_name: row.get(7)?,
                content_id: row.get(8)?,
                size_in_bytes: size_bytes.map(|s| s as u64),
                raw_json: row.get(10)?,
                updated_at: updated_at as u64,
                ttl: ttl as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_catalog_products(&self) -> SqlResult<Vec<CachedCatalogProduct>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT product_id, title, developer, publisher, description, poster_url, hero_url, 
                    package_family_name, content_id, size_in_bytes, raw_json, updated_at, ttl 
             FROM catalog_cache ORDER BY title ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let size_bytes: Option<i64> = row.get(9)?;
            let updated_at: i64 = row.get(11)?;
            let ttl: i64 = row.get(12)?;
            Ok(CachedCatalogProduct {
                product_id: row.get(0)?,
                title: row.get(1)?,
                developer: row.get(2)?,
                publisher: row.get(3)?,
                description: row.get(4)?,
                poster_url: row.get(5)?,
                hero_url: row.get(6)?,
                package_family_name: row.get(7)?,
                content_id: row.get(8)?,
                size_in_bytes: size_bytes.map(|s| s as u64),
                raw_json: row.get(10)?,
                updated_at: updated_at as u64,
                ttl: ttl as u64,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_fresh_catalog_products(&self) -> SqlResult<Vec<CachedCatalogProduct>> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        let mut stmt = conn.prepare(
            "SELECT product_id, title, developer, publisher, description, poster_url, hero_url, 
                    package_family_name, content_id, size_in_bytes, raw_json, updated_at, ttl 
             FROM catalog_cache 
             WHERE ?1 <= (updated_at + ttl)
             ORDER BY title ASC",
        )?;

        let rows = stmt.query_map(params![now], |row| {
            let size_bytes: Option<i64> = row.get(9)?;
            let updated_at: i64 = row.get(11)?;
            let ttl: i64 = row.get(12)?;
            Ok(CachedCatalogProduct {
                product_id: row.get(0)?,
                title: row.get(1)?,
                developer: row.get(2)?,
                publisher: row.get(3)?,
                description: row.get(4)?,
                poster_url: row.get(5)?,
                hero_url: row.get(6)?,
                package_family_name: row.get(7)?,
                content_id: row.get(8)?,
                size_in_bytes: size_bytes.map(|s| s as u64),
                raw_json: row.get(10)?,
                updated_at: updated_at as u64,
                ttl: ttl as u64,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn save_catalog_product(&self, p: &CachedCatalogProduct) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        let size_bytes = p.size_in_bytes.map(|s| s as i64);
        let updated = if p.updated_at == 0 { now } else { p.updated_at as i64 };
        let ttl = if p.ttl == 0 { 604800i64 } else { p.ttl as i64 };

        conn.execute(
            r#"
            INSERT INTO catalog_cache (
                product_id, title, developer, publisher, description, poster_url, hero_url,
                package_family_name, content_id, size_in_bytes, raw_json, updated_at, ttl
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(product_id) DO UPDATE SET
                title = excluded.title,
                developer = excluded.developer,
                publisher = excluded.publisher,
                description = excluded.description,
                poster_url = excluded.poster_url,
                hero_url = excluded.hero_url,
                package_family_name = excluded.package_family_name,
                content_id = excluded.content_id,
                size_in_bytes = excluded.size_in_bytes,
                raw_json = excluded.raw_json,
                updated_at = excluded.updated_at,
                ttl = excluded.ttl
            "#,
            params![
                p.product_id,
                p.title,
                p.developer,
                p.publisher,
                p.description,
                p.poster_url,
                p.hero_url,
                p.package_family_name,
                p.content_id,
                size_bytes,
                p.raw_json,
                updated,
                ttl
            ],
        )?;
        Ok(())
    }

    pub fn save_catalog_products_batch(&self, products: &[CachedCatalogProduct]) -> SqlResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = Self::now_secs();
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO catalog_cache (
                    product_id, title, developer, publisher, description, poster_url, hero_url,
                    package_family_name, content_id, size_in_bytes, raw_json, updated_at, ttl
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(product_id) DO UPDATE SET
                    title = excluded.title,
                    developer = excluded.developer,
                    publisher = excluded.publisher,
                    description = excluded.description,
                    poster_url = excluded.poster_url,
                    hero_url = excluded.hero_url,
                    package_family_name = excluded.package_family_name,
                    content_id = excluded.content_id,
                    size_in_bytes = excluded.size_in_bytes,
                    raw_json = excluded.raw_json,
                    updated_at = excluded.updated_at,
                    ttl = excluded.ttl
                "#,
            )?;

            for p in products {
                let size_bytes = p.size_in_bytes.map(|s| s as i64);
                let updated = if p.updated_at == 0 { now } else { p.updated_at as i64 };
                let ttl = if p.ttl == 0 { 604800i64 } else { p.ttl as i64 };

                stmt.execute(params![
                    p.product_id,
                    p.title,
                    p.developer,
                    p.publisher,
                    p.description,
                    p.poster_url,
                    p.hero_url,
                    p.package_family_name,
                    p.content_id,
                    size_bytes,
                    p.raw_json,
                    updated,
                    ttl
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    // --- Entitlements Methods ---
    pub fn get_user_entitlements(&self, xuid: &str) -> SqlResult<Vec<CachedEntitlement>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT xuid, product_id, sku_id, title, entitlement_type, acquired_date, updated_at 
             FROM user_entitlements WHERE xuid = ?1",
        )?;

        let rows = stmt.query_map(params![xuid], |row| {
            let updated_at: i64 = row.get(6)?;
            Ok(CachedEntitlement {
                xuid: row.get(0)?,
                product_id: row.get(1)?,
                sku_id: row.get(2)?,
                title: row.get(3)?,
                entitlement_type: row.get(4)?,
                acquired_date: row.get(5)?,
                updated_at: updated_at as u64,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn save_user_entitlements(&self, xuid: &str, items: &[CachedEntitlement]) -> SqlResult<()> {
        self.replace_user_entitlements(xuid, items)
    }

    pub fn replace_user_entitlements(&self, xuid: &str, items: &[CachedEntitlement]) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM user_entitlements WHERE xuid = ?1", params![xuid])?;
        for it in items {
            tx.execute(
                r#"
                INSERT INTO user_entitlements (
                    xuid, product_id, sku_id, title, entitlement_type, acquired_date, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    xuid,
                    it.product_id,
                    it.sku_id,
                    it.title,
                    it.entitlement_type,
                    it.acquired_date,
                    now
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn clean_invalid_entitlements(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        // Purge any entitlements whose product_id is purely numeric or invalid format
        conn.execute(
            "DELETE FROM user_entitlements WHERE LENGTH(product_id) != 12 AND product_id GLOB '[0-9]*'",
            [],
        )?;
        Ok(())
    }

    // --- Profile & Game Pass Status Methods ---
    pub fn get_user_profile(&self, xuid: &str) -> SqlResult<Option<CachedUserProfile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT xuid, gamertag, display_pic_url, gamer_score, presence_state, 
                    presence_title, has_gamepass, subscription_tier, updated_at 
             FROM user_profiles WHERE xuid = ?1",
        )?;

        let mut rows = stmt.query(params![xuid])?;
        if let Some(row) = rows.next()? {
            let has_gp: i32 = row.get(6)?;
            let updated_at: i64 = row.get(8)?;
            Ok(Some(CachedUserProfile {
                xuid: row.get(0)?,
                gamertag: row.get(1)?,
                display_pic_url: row.get(2)?,
                gamer_score: row.get(3)?,
                presence_state: row.get(4)?,
                presence_title: row.get(5)?,
                has_gamepass: has_gp != 0,
                subscription_tier: row.get(7)?,
                updated_at: updated_at as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn save_user_profile(&self, prof: &CachedUserProfile) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        let updated = if prof.updated_at == 0 { now } else { prof.updated_at as i64 };
        conn.execute(
            r#"
            INSERT INTO user_profiles (
                xuid, gamertag, display_pic_url, gamer_score, presence_state,
                presence_title, has_gamepass, subscription_tier, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(xuid) DO UPDATE SET
                gamertag = excluded.gamertag,
                display_pic_url = excluded.display_pic_url,
                gamer_score = excluded.gamer_score,
                presence_state = excluded.presence_state,
                presence_title = excluded.presence_title,
                has_gamepass = excluded.has_gamepass,
                subscription_tier = excluded.subscription_tier,
                updated_at = excluded.updated_at
            "#,
            params![
                prof.xuid,
                prof.gamertag,
                prof.display_pic_url,
                prof.gamer_score,
                prof.presence_state,
                prof.presence_title,
                if prof.has_gamepass { 1 } else { 0 },
                prof.subscription_tier,
                updated
            ],
        )?;
        Ok(())
    }

    pub fn update_presence_state(&self, xuid: &str, state: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        conn.execute(
            "UPDATE user_profiles SET presence_state = ?1, updated_at = ?2 WHERE xuid = ?3",
            params![state, now, xuid],
        )?;
        Ok(())
    }

    // --- Friends List Methods ---
    pub fn get_friends(&self, xuid: &str) -> SqlResult<Vec<CachedFriend>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT xuid, friend_xuid, gamertag, display_pic_url, presence_state, presence_title, updated_at 
             FROM social_friends WHERE xuid = ?1",
        )?;

        let rows = stmt.query_map(params![xuid], |row| {
            let updated_at: i64 = row.get(6)?;
            Ok(CachedFriend {
                xuid: row.get(0)?,
                friend_xuid: row.get(1)?,
                gamertag: row.get(2)?,
                display_pic_url: row.get(3)?,
                presence_state: row.get(4)?,
                presence_title: row.get(5)?,
                updated_at: updated_at as u64,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn save_friends(&self, xuid: &str, friends: &[CachedFriend]) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        let tx = conn.unchecked_transaction()?;

        tx.execute("DELETE FROM social_friends WHERE xuid = ?1", params![xuid])?;

        for f in friends {
            tx.execute(
                r#"
                INSERT INTO social_friends (
                    xuid, friend_xuid, gamertag, display_pic_url, presence_state, presence_title, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    xuid,
                    f.friend_xuid,
                    f.gamertag,
                    f.display_pic_url,
                    f.presence_state,
                    f.presence_title,
                    now
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // --- Key-Value Cache Methods ---
    pub fn get_kv(&self, key: &str) -> SqlResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let now = Self::now_secs();
        let mut stmt = conn.prepare("SELECT value, expires_at FROM kv_cache WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let expires_at: i64 = row.get(1)?;
            if expires_at != 0 && now > expires_at {
                return Ok(None);
            }
            let val: String = row.get(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn set_kv(&self, key: &str, value: &str, ttl_secs: u64) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let expires_at = if ttl_secs > 0 { Self::now_secs() + (ttl_secs as i64) } else { 0 };
        conn.execute(
            r#"
            INSERT INTO kv_cache (key, value, expires_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                expires_at = excluded.expires_at
            "#,
            params![key, value, expires_at],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_in_memory_crud() {
        let db = Database::open_in_memory().expect("failed to open in memory db");

        // 1. Catalog Product
        let product = CachedCatalogProduct {
            product_id: "9N44Q5Q49DBC".into(),
            title: "Brotato".into(),
            developer: "Blobfish".into(),
            publisher: "Seaven Studio".into(),
            description: "Top-down arena shooter".into(),
            poster_url: Some("https://shared.steamstatic.com/store_item_assets/steam/apps/2042420/library_600x900.jpg".into()),
            hero_url: None,
            package_family_name: None,
            content_id: None,
            size_in_bytes: Some(423_200_000),
            raw_json: None,
            updated_at: 0,
            ttl: 604800,
        };
        db.save_catalog_product(&product).unwrap();
        let fetched = db.get_catalog_product("9N44Q5Q49DBC").unwrap().expect("product not found");
        assert_eq!(fetched.title, "Brotato");
        assert_eq!(fetched.developer, "Blobfish");

        // 2. User Profile & Game Pass
        let profile = CachedUserProfile {
            xuid: "2533274812345678".into(),
            gamertag: "nocatix".into(),
            display_pic_url: Some("http://avatar.xboxlive.com/pic.png".into()),
            gamer_score: Some("15420".into()),
            presence_state: Some("Online".into()),
            presence_title: Some("Brotato".into()),
            has_gamepass: true,
            subscription_tier: Some("GamePassUltimate".into()),
            updated_at: 0,
        };
        db.save_user_profile(&profile).unwrap();
        let fetched_prof = db.get_user_profile("2533274812345678").unwrap().expect("profile not found");
        assert_eq!(fetched_prof.gamertag, "nocatix");
        assert!(fetched_prof.has_gamepass);

        // 3. Friends list
        let friends = vec![
            CachedFriend {
                xuid: "2533274812345678".into(),
                friend_xuid: "111".into(),
                gamertag: "MasterChief".into(),
                display_pic_url: None,
                presence_state: "InGame".into(),
                presence_title: Some("Halo Infinite".into()),
                updated_at: 0,
            }
        ];
        db.save_friends("2533274812345678", &friends).unwrap();
        let fetched_friends = db.get_friends("2533274812345678").unwrap();
        assert_eq!(fetched_friends.len(), 1);
        assert_eq!(fetched_friends[0].gamertag, "MasterChief");

        // 4. KV Cache
        db.set_kv("test_key", "test_value", 3600).unwrap();
        assert_eq!(db.get_kv("test_key").unwrap().as_deref(), Some("test_value"));
    }
}

