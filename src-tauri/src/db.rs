use rusqlite::{Connection, Result};
use sqlite_vec::sqlite3_vec_init;
use std::path::PathBuf;
use std::sync::{OnceLock, Once};

const MAX_DURATION_SEC: i32 = 60;

// Store app data directory path (set during initialization)
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

// Ensure sqlite-vec extension is registered only once globally
static VEC_EXTENSION_INIT: Once = Once::new();

/// Get the app data directory (for use by other modules)
pub fn get_app_data_dir() -> Option<&'static PathBuf> {
    APP_DATA_DIR.get()
}

/// Initialize the database with app data directory from Tauri
/// This should be called once during app setup with the Tauri app handle
pub fn set_app_data_dir(app_data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    APP_DATA_DIR.set(app_data_dir)
        .map_err(|_| "App data directory already set".into())
}

fn get_db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let app_data = APP_DATA_DIR.get()
        .ok_or("App data directory not initialized. Call set_app_data_dir() first.")?;
    Ok(app_data.join("yaad.db"))
}

fn register_vec_extension() {
    VEC_EXTENSION_INIT.call_once(|| {
        // THE FIX: Register the extension globally using auto-extension
        // This tells SQLite: "Hey, when I say 'vec0', use this code."
        unsafe {
            use rusqlite::ffi;
            // Register the vec0 extension as an auto-extension
            ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }
    });
}

pub fn init_db() -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL: Register the vec0 extension BEFORE opening any connections
    register_vec_extension();
    
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;

    // Enable WAL mode for better performance
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to enable WAL mode: {}", e))?;

    // Create memories table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            screenshot_path TEXT,
            ocr_text TEXT,
            app_name TEXT,
            duration_sec INTEGER,
            context_url TEXT,
            context_note TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            is_synced BOOLEAN DEFAULT 0,
            is_deleted BOOLEAN DEFAULT 0
        )",
        [],
    )?;

    // Ensure v2.0 columns exist (Migration for existing DBs)
    let _ = conn.execute("ALTER TABLE memories ADD COLUMN screenshot_path TEXT", []);
    let _ = conn.execute("ALTER TABLE memories ADD COLUMN ocr_text TEXT", []);
    let _ = conn.execute("ALTER TABLE memories ADD COLUMN app_name TEXT", []);

    // Create vec_memories virtual table for text vector search
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_memories USING vec0(
            embedding float[384]
        )",
        [],
    )?;

    // Create vec_visuals virtual table for image vector search (Tiny CLIP is 512)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_visuals USING vec0(
            embedding float[512]
        )",
        [],
    )?;

    // Create cleanup trigger for deleted memories (Text)
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS delete_vector 
        AFTER UPDATE OF is_deleted ON memories 
        WHEN NEW.is_deleted = 1
        BEGIN
          DELETE FROM vec_memories WHERE rowid = NEW.rowid;
          DELETE FROM vec_visuals WHERE rowid = NEW.rowid;
        END",
        [],
    )?;

    Ok(())
}

pub fn get_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    // Ensure extension is registered before opening connection
    register_vec_extension();
    
    let db_path = get_db_path()?;
    
    // Open connection with optimizations
    let conn = Connection::open(&db_path)?;
    
    // Enable WAL mode for better concurrency and performance
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to enable WAL mode: {}", e))?;
    
    // Optimize for read-heavy workload
    conn.execute("PRAGMA synchronous = NORMAL", [])
        .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
    
    // Increase cache size for better performance (16MB)
    conn.execute("PRAGMA cache_size = -4096", [])
        .map_err(|e| format!("Failed to set cache size: {}", e))?;
    
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_db() {
        // This would need a test database path
        // For now, just verify the function compiles
        assert!(true);
    }
}

