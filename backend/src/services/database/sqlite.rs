/**
 * Database client implementation for SQLite.
 *
 * Uses async-sqlite.
 *
 * Probably nee to split the core database code into a separate class,
 * and the queue implementation should also be separate, with its own database.
 *
 * @docs https://docs.rs/async-sqlite/latest/async_sqlite/
 */
use async_sqlite::{JournalMode, Pool, PoolBuilder};
use async_trait::async_trait;
use log::{debug, error, info};

use crate::errors::Error;
use crate::services::database::r#trait::Database;
use crate::types::{Bounds, FileRecord, QueueMessage, TreeInfo, UploadTicket, UserInfo};
use crate::utils::{get_sqlite_path, get_timestamp, get_unique_id};
use crate::Result;

pub struct SqliteDatabase {
    pub pool: Pool,
}

impl SqliteDatabase {
    pub async fn new() -> Result<Self> {
        let path = get_sqlite_path()?;

        Ok(Self {
            pool: Self::create_pool(&path).await?,
        })
    }

    async fn create_pool(path: &str) -> Result<Pool> {
        match path {
            ":memory:" => Self::create_memory_pool().await,
            _ => Self::create_file_pool(path).await,
        }
    }

    async fn create_file_pool(path: &str) -> Result<Pool> {
        info!("Using SQLite database from {}.", path);

        let pool = match PoolBuilder::new()
            .path(path)
            .journal_mode(JournalMode::Wal)
            .open()
            .await
        {
            Ok(value) => value,
            Err(e) => {
                error!("Error connecting to the database: {}", e);
                return Err(Error::DatabaseConnect);
            }
        };

        Ok(pool)
    }

    async fn create_memory_pool() -> Result<Pool> {
        info!("Using an in-memory SQLite database.");

        let pool = match PoolBuilder::new().num_conns(1).open().await {
            Ok(value) => value,
            Err(e) => {
                error!("Error connecting to the database: {}", e);
                return Err(Error::DatabaseConnect);
            }
        };

        Self::setup_memory_db(&pool).await?;

        Ok(pool)
    }

    async fn setup_memory_db(pool: &Pool) -> Result<()> {
        let script = include_str!("../../../dev/schema-sqlite.sql");
        Self::execute_pool(pool, script.to_string()).await?;

        debug!("Memory database initialized.");

        Ok(())
    }

    #[allow(unused)]
    pub async fn execute(&self, script: String) -> Result<()> {
        Self::execute_pool(&self.pool, script).await
    }

    async fn execute_pool(pool: &Pool, script: String) -> Result<()> {
        let res = pool.conn(move |conn| conn.execute_batch(&script)).await;

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Error executing SQL script: {}", e);
                Err(Error::DatabaseQuery)
            }
        }
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn add_tree(&self, tree: &TreeInfo) -> Result<()> {
        let id = tree.id;
        let lat = tree.lat;
        let lon = tree.lon;
        let name = tree.name.clone();
        let height = tree.height;
        let circumference = tree.circumference;
        let diameter = tree.diameter;
        let state = tree.state.clone();
        let added_at = tree.added_at;
        let updated_at = tree.updated_at;
        let added_by = tree.added_by;

        self.pool.conn(move |conn| {
            match conn.execute("INSERT INTO trees (id, lat, lon, name, height, circumference, diameter, state, added_at, updated_at, added_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", (id, lat, lon, name, height, circumference, diameter, state, added_at, updated_at, added_by)) {
                Ok(_) => (),

                Err(e) => {
                    error!("Error adding tree to the database: {}", e);
                    return Err(e);
                },
            };

            debug!("Tree {} added to the database.", id);
            Ok(())
        }).await?;

        Ok(())
    }

    async fn update_tree(&self, tree: &TreeInfo) -> Result<()> {
        let id = tree.id;
        let lat = tree.lat;
        let lon = tree.lon;
        let name = tree.name.clone();
        let height = tree.height;
        let circumference = tree.circumference;
        let diameter = tree.diameter;
        let state = tree.state.clone();
        let updated_at = tree.updated_at;

        self.pool.conn(move |conn| {
            match conn.execute("UPDATE trees set lat = ?, lon = ?, name = ?, height = ?, circumference = ?, diameter = ?, state = ?, updated_at = ? WHERE id = ?", (lat, lon, name, height, circumference, diameter, state, updated_at, id)) {
                Ok(_) => (),

                Err(e) => {
                    error!("Error updating a tree in the database: {}", e);
                    return Err(e);
                },
            };

            debug!("Tree {} updated.", id);
            Ok(())
        }).await?;

        Ok(())
    }

    async fn move_tree(&self, id: u64, lat: f64, lon: f64) -> Result<()> {
        let updated_at = get_timestamp();

        self.pool
            .conn(move |conn| {
                match conn.execute(
                    "UPDATE trees set lat = ?, lon = ?, updated_at = ? WHERE id = ?",
                    (lat, lon, updated_at, id),
                ) {
                    Ok(_) => (),

                    Err(e) => {
                        error!("Error updating a tree in the database: {}", e);
                        return Err(e);
                    }
                };

                debug!("Tree {} moved.", id);
                Ok(())
            })
            .await?;

        Ok(())
    }

    /**
     * Read information on a single tree.
     */
    async fn get_tree(&self, id: u64) -> Result<Option<TreeInfo>> {
        let tree = self.pool.conn(move |conn| {
            let mut stmt = match conn.prepare("SELECT id, lat, lon, name, height, circumference, diameter, state, added_at, updated_at, added_by FROM trees WHERE id = ?") {
                Ok(value) => value,

                Err(e) => {
                    error!("Error preparing SQL statement: {}", e);
                    return Err(e);
                },
            };

            let mut rows = stmt.query([id])?;

            if let Some(row) = rows.next()? {
                let id: u64 = row.get(0)?;

                return Ok(Some(TreeInfo {
                    id,
                    lat: row.get(1)?,
                    lon: row.get(2)?,
                    name: row.get(3)?,
                    height: row.get(4)?, // only in details view
                    circumference: row.get(5)?,
                    diameter: row.get(6)?,
                    state: row.get(7)?,
                    added_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    added_by: row.get(10)?,
                }));
            }

            Ok(None)
        }).await?;

        Ok(tree)
    }

    /**
     * Read all trees from the database.
     *
     * https://docs.rs/rusqlite/0.30.0/rusqlite/index.html
     */
    async fn get_trees(&self, bounds: Bounds) -> Result<Vec<TreeInfo>> {
        let trees = self.pool.conn(move |conn| {
            let mut stmt = match conn.prepare("SELECT id, lat, lon, name, height, circumference, diameter, state, added_at, updated_at, added_by FROM trees WHERE lat <= ? AND lat >= ? AND lon <= ? AND lon >= ? AND state <> 'gone'") {
                Ok(value) => value,

                Err(e) => {
                    error!("Error preparing SQL statement: {}", e);
                    return Err(e);
                },
            };

            let mut rows = stmt.query([bounds.n, bounds.s, bounds.e, bounds.w])?;

            let mut trees: Vec<TreeInfo> = Vec::new();

            while let Some(row) = rows.next()? {
                let id: u64 = row.get(0)?;
                let lat: f64 = row.get(1)?;
                let lon: f64 = row.get(2)?;

                trees.push(TreeInfo {
                    id,
                    lat,
                    lon,
                    name: row.get(3)?,
                    height: row.get(4)?, // only in details view
                    circumference: row.get(5)?,
                    diameter: row.get(6)?,
                    state: row.get(7)?,
                    added_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    added_by: row.get(10)?,
                });
            }

            Ok(trees)
        }).await?;

        Ok(trees)
    }

    /**
     * Record a change in tree properties.
     *
     * Returns new property id.
     */
    async fn add_tree_prop(&self, tree_id: u64, name: &str, value: &str) -> Result<u64> {
        let id = get_unique_id()?;
        let added_at = crate::utils::get_timestamp();
        let name = name.to_string();
        let value = value.to_string();

        self.pool.conn(move |conn| {
            match conn.execute("INSERT INTO trees_props (id, tree_id, added_at, name, value) VALUES (?, ?, ?, ?, ?)", (id, tree_id, added_at, name, value)) {
                Ok(_) => debug!("Property {} added to tree {}.", id, tree_id),

                Err(e) => {
                    error!("Error adding property to tree: {}", e);
                    return Err(e);
                }
            }

            Ok(conn.last_insert_rowid())
        }).await?;

        Ok(id)
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<UserInfo>> {
        let email = email.to_string();

        let user = self
            .pool
            .conn(move |conn| {
                let mut stmt = match conn
                    .prepare("SELECT id, email, name, picture FROM users WHERE email = ?")
                {
                    Ok(value) => value,

                    Err(e) => {
                        error!("Error preparing SQL statement: {}", e);
                        return Err(e);
                    }
                };

                let mut rows = stmt.query([email])?;

                if let Some(row) = rows.next()? {
                    let id: u64 = row.get(0)?;

                    return Ok(Some(UserInfo {
                        id,
                        email: row.get(1)?,
                        name: row.get(2)?,
                        picture: row.get(3)?,
                    }));
                }

                Ok(None)
            })
            .await?;

        Ok(user)
    }

    async fn add_user(&self, user: &UserInfo) -> Result<()> {
        let id = user.id;
        let email = user.email.clone();
        let name = user.name.clone();
        let picture = user.picture.clone();

        self.pool
            .conn(move |conn| {
                conn.execute(
                    "INSERT INTO users (id, email, name, picture) VALUES (?, ?, ?, ?)",
                    (id, email, name, picture),
                )?;
                debug!("User {} added to the database.", id);
                Ok(())
            })
            .await?;

        Ok(())
    }

    async fn add_upload_ticket(&self, ticket: &UploadTicket) -> Result<()> {
        let id = ticket.id;
        let created_at = ticket.created_at;
        let created_by = ticket.created_by;
        let upload_url = ticket.upload_url.to_string();

        self.pool
            .conn(move |conn| {
                conn.execute(
                    "INSERT INTO upload_tickets (id, created_at, created_by, upload_url) VALUES (?, ?, ?, ?)",
                    (id, created_at, created_by, upload_url),
                )?;

                Ok(())
            })
            .await?;

        debug!("Upload ticket {} added to the database.", id);

        Ok(())
    }

    async fn get_upload_ticket(&self, id: u64) -> Result<Option<UploadTicket>> {
        let ticket = self.pool.conn(move |conn| {
            let mut stmt = match conn.prepare("SELECT id, created_at, created_by, upload_url FROM upload_tickets WHERE id = ?") {
                Ok(value) => value,

                Err(e) => {
                    error!("Error preparing SQL statement: {}", e);
                    return Err(e);
                },
            };

            let mut rows = stmt.query([id])?;

            if let Some(row) = rows.next()? {
                return Ok(Some(UploadTicket {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    created_by: row.get(2)?,
                    upload_url: row.get(3)?,
                }));
            }

            Ok(None)
        }).await?;

        Ok(ticket)
    }

    async fn add_queue_message(&self, msg: &QueueMessage) -> Result<()> {
        let id = msg.id;
        let added_at = msg.added_at;
        let available_at = msg.available_at;
        let payload = msg.payload.clone();

        self.pool
            .conn(move |conn| {
                conn.execute(
                    "INSERT INTO queue_messages (id, added_at, available_at, payload) VALUES (?, ?, ?, ?)",
                    (id, added_at, available_at, payload),
                )?;

                Ok(())
            })
            .await?;

        debug!("Queue message {} added to the database.", id);

        Ok(())
    }

    async fn pick_queue_message(&self) -> Result<Option<QueueMessage>> {
        let now = get_timestamp();

        let message = self.pool.conn(move |conn| {
            let mut stmt = match conn.prepare("SELECT id, added_at, available_at, payload FROM queue_messages WHERE available_at <= ? ORDER BY id LIMIT 1") {
                Ok(value) => value,

                Err(e) => {
                    error!("Error preparing SQL statement: {}", e);
                    return Err(e);
                },
            };

            let mut rows = stmt.query([now])?;

            if let Some(row) = rows.next()? {
                return Ok(Some(QueueMessage {
                    id: row.get(0)?,
                    added_at: row.get(1)?,
                    available_at: row.get(2)?,
                    payload: row.get(3)?,
                }));
            }

            Ok(None)
        }).await?;

        Ok(message)
    }

    async fn delay_queue_message(&self, id: u64, available_at: u64) -> Result<()> {
        self.pool
            .conn(move |conn| {
                match conn.execute(
                    "UPDATE queue_messages SET available_at = ? WHERE id = ?",
                    (available_at, id),
                ) {
                    Ok(_) => (),

                    Err(e) => {
                        error!("Error updating a queue message: {}", e);
                        return Err(e);
                    }
                };

                debug!("Queue message {} updated.", id);
                Ok(())
            })
            .await?;

        Ok(())
    }

    async fn add_file(&self, file: &FileRecord) -> Result<()> {
        let id = file.id;
        let added_at = file.added_at;
        let added_by = file.added_by;
        let tree_id = file.tree_id;
        let small_id = file.small_id;
        let large_id = file.large_id;

        self.pool
            .conn(move |conn| {
                match conn.execute(
                    "INSERT INTO files (id, tree_id, added_at, added_by, small_id, large_id) VALUES (?, ?, ?, ?, ?, ?)",
                    (id, tree_id, added_at, added_by, small_id, large_id),
                ) {
                    Ok(_) => (),

                    Err(e) => {
                        error!("Error adding file to the database: {}", e);
                        return Err(e);
                    },
                }

                Ok(())
            })
            .await?;

        debug!("File {} added to the database.", id);

        Ok(())
    }
}

impl Clone for SqliteDatabase {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger;
    use std::env;

    async fn setup() -> Result<SqliteDatabase> {
        env::set_var("TREEMAP_SQLITE_PATH", ":memory:");

        if let Err(_) = env_logger::try_init() {
            debug!("env_logger already initialized.");
        };

        let db = SqliteDatabase::new().await?;
        db.execute(include_str!("./fixtures/init.sql").to_string())
            .await
            .unwrap();

        Ok(db)
    }

    #[tokio::test]
    async fn test_get_trees() -> Result<()> {
        let db = setup().await?;
        db.execute(include_str!("./fixtures/trees.sql").to_string())
            .await
            .unwrap();

        let bounds = Bounds {
            n: 90.0,
            e: 180.0,
            s: -90.0,
            w: -180.0,
        };

        let trees = match db.get_trees(bounds).await {
            Ok(value) => value,

            Err(e) => {
                panic!("Error reading trees from the database: {}", e);
            }
        };

        assert_eq!(trees.len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn test_add_tree() -> Result<()> {
        let db = setup().await?;

        let before = db
            .get_trees(Bounds {
                n: 90.0,
                e: 180.0,
                s: -90.0,
                w: -180.0,
            })
            .await?;

        assert_eq!(before.len(), 0);

        db.add_tree(&TreeInfo {
            id: 123,
            lat: 56.65,
            lon: 28.48,
            name: "Oak".to_string(),
            height: Some(12.0),
            circumference: Some(1.0),
            diameter: None,
            state: "healthy".to_string(),
            added_at: 0,
            updated_at: 0,
            added_by: 0,
        })
        .await?;

        let after = db
            .get_trees(Bounds {
                n: 90.0,
                e: 180.0,
                s: -90.0,
                w: -180.0,
            })
            .await?;

        assert_eq!(after.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_upload_ticket() -> Result<()> {
        let db = setup().await?;

        let before = db.get_upload_ticket(123).await?;
        assert!(before.is_none());

        let ticket = UploadTicket {
            id: 123,
            created_at: 123,
            created_by: 456,
            upload_url: "https://example.com".to_string(),
        };

        db.add_upload_ticket(&ticket)
            .await
            .expect("Error adding upload ticket");

        let after = db
            .get_upload_ticket(123)
            .await?
            .expect("Ticket not created.");
        assert_eq!(after.id, 123);
        assert_eq!(after.created_at, 123);
        assert_eq!(after.created_by, 456);
        assert_eq!(after.upload_url, "https://example.com");

        Ok(())
    }

    #[tokio::test]
    async fn test_add_queue_message() {
        let db = setup().await.expect("Error setting up database.");
        let now = get_timestamp();

        let msg = QueueMessage {
            id: 123,
            added_at: now,
            available_at: now,
            payload: "it works".to_string(),
        };

        db.add_queue_message(&msg)
            .await
            .expect("Error adding message.");

        let pick = db
            .pick_queue_message()
            .await
            .expect("Error picking message.");
        assert_eq!(pick.unwrap().id, 123);
    }

    #[tokio::test]
    async fn test_add_queue_message_not_available() {
        let db = setup().await.expect("Error setting up database.");
        let now = get_timestamp();

        let msg = QueueMessage {
            id: 123,
            added_at: now,
            available_at: now + 10,
            payload: "it works".to_string(),
        };

        db.add_queue_message(&msg)
            .await
            .expect("Error adding message.");

        let pick = db
            .pick_queue_message()
            .await
            .expect("Error picking message.");
        assert!(pick.is_none());
    }

    #[tokio::test]
    async fn test_delay_queue_message() {
        let db = setup().await.expect("Error setting up database.");
        let now = get_timestamp();

        let msg = QueueMessage {
            id: 123,
            added_at: now,
            available_at: now,
            payload: "it works".to_string(),
        };

        db.add_queue_message(&msg)
            .await
            .expect("Error adding message.");
        db.delay_queue_message(123, now + 10)
            .await
            .expect("Error delaying message.");

        let pick = db
            .pick_queue_message()
            .await
            .expect("Error picking message.");
        assert!(pick.is_none());
    }
}
