use crate::db;
use crate::embeddings;
use crate::models;
use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{State, Manager, Emitter};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub duration_sec: Option<i32>,
    pub context_url: Option<String>,
    pub context_note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub memory: Memory,
    pub similarity: f32,
}

// Global state for recording - kept for command layer logic if needed,
// but whisper module is now the source of truth for audio loop.
pub struct RecordingState {
    pub is_recording: bool,
    pub start_time: Option<std::time::Instant>,
}

#[tauri::command]
pub async fn save_memory(
    content: String,
    duration_sec: Option<i32>,
    context_url: Option<String>,
    context_note: Option<String>,
) -> Result<String, String> {
    // Validate input
    if content.trim().is_empty() {
        return Err("Memory content cannot be empty".to_string());
    }
    
    if content.len() > 10000 {
        return Err("Memory content is too long (max 10,000 characters)".to_string());
    }
    
    // Validate duration
    if let Some(duration) = duration_sec {
        if duration > 30 {
            return Err("Recording duration exceeds 30 seconds".to_string());
        }
        if duration < 0 {
            return Err("Invalid recording duration".to_string());
        }
    }

    let id = Uuid::new_v4().to_string();
    
    // Retry database connection on failure
    let conn = db::get_connection()
        .map_err(|e| format!("Database connection failed: {}. Please try again.", e))?;

    // Generate embedding with error handling
    let embedding = embeddings::generate_embedding(&content)
        .map_err(|e| {
            if e.to_string().contains("not initialized") {
                "Embedding model not loaded. Please restart the app.".to_string()
            } else {
                format!("Failed to generate embedding: {}", e)
            }
        })?;

    // Insert into memories table with retry
    conn.execute(
        "INSERT INTO memories (id, content, duration_sec, context_url, context_note) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, content, duration_sec, context_url, context_note],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Memory with this ID already exists".to_string()
        } else if e.to_string().contains("database") || e.to_string().contains("locked") {
            "Database error. Please try again.".to_string()
        } else {
            format!("Failed to save memory: {}", e)
        }
    })?;

    // Get the rowid of the inserted memory
    let rowid = conn.last_insert_rowid();

    // Store embedding in vec_memories virtual table
    embeddings::store_embedding(&conn, rowid, &embedding)
        .map_err(|e| format!("Failed to store embedding: {}", e))?;

    Ok(id)
}

#[tauri::command]
pub async fn search_memories(query: String, limit: i32) -> Result<Vec<SearchResult>, String> {
    let conn = db::get_connection().map_err(|e| e.to_string())?;

    // Generate embedding for query
    let query_embedding = embeddings::generate_embedding(&query)
        .map_err(|e| format!("Failed to generate query embedding: {}", e))?;

    // Search for similar memories
    let matches = embeddings::search_similar(&conn, &query_embedding, limit)
        .map_err(|e| format!("Failed to search memories: {}", e))?;

    // Retrieve full memory data for each match
    let mut results = Vec::new();
    
    if matches.is_empty() {
        return Ok(results);
    }

    // Optimization: Fetch all memories in a single query
    // 1. Create a temporary map of rowid -> similarity for quick lookup
    let similarity_map: HashMap<i64, f32> = matches.iter().cloned().collect();
    
    // 2. Build comma-separated list of rowids for the IN clause
    let rowids: Vec<String> = matches.iter().map(|(id, _)| id.to_string()).collect();
    let query_placeholders = rowids.join(",");
    
    let sql = format!(
        "SELECT rowid, id, content, duration_sec, context_url, context_note, created_at 
         FROM memories 
         WHERE rowid IN ({}) AND is_deleted = 0",
        query_placeholders
    );

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("Failed to prepare bulk query: {}", e))?;
        
    let memory_rows = stmt.query_map([], |row| {
        let rowid: i64 = row.get(0)?;
        Ok((
            rowid,
            Memory {
                id: row.get(1)?,
                content: row.get(2)?,
                duration_sec: row.get(3)?,
                context_url: row.get(4)?,
                context_note: row.get(5)?,
                created_at: row.get(6)?,
            }
        ))
    })
    .map_err(|e| format!("Failed to execute bulk query: {}", e))?;

    for row in memory_rows {
        let (rowid, memory) = row.map_err(|e| e.to_string())?;
        if let Some(&similarity) = similarity_map.get(&rowid) {
            results.push(SearchResult { memory, similarity });
        }
    }
    
    // Re-sort to maintain order (SQL result order is not guaranteed with IN)
    results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

#[tauri::command]
pub async fn get_memory(id: String) -> Result<Memory, String> {
    let conn = db::get_connection().map_err(|e| e.to_string())?;

    let memory = conn
        .query_row(
            "SELECT id, content, duration_sec, context_url, context_note, created_at 
             FROM memories WHERE id = ?1 AND is_deleted = 0",
            [id],
            |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    duration_sec: row.get(2)?,
                    context_url: row.get(3)?,
                    context_note: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .map_err(|e| format!("Memory not found: {}", e))?;

    Ok(memory)
}

#[tauri::command]
pub async fn delete_memory(id: String) -> Result<(), String> {
    let conn = db::get_connection().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE memories SET is_deleted = 1 WHERE id = ?1",
        [id],
    )
    .map_err(|e| format!("Failed to delete memory: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn start_recording(
    state: State<'_, Mutex<RecordingState>>,
) -> Result<(), String> {
    // Reset any stale state
    let _ = crate::whisper::reset_recording_state();
    
    let mut rec_state = state.lock().map_err(|e| e.to_string())?;
    
    if rec_state.is_recording {
        rec_state.is_recording = false;
        rec_state.start_time = None;
    }

    match crate::whisper::start_recording() {
        Ok(()) => {
            rec_state.is_recording = true;
            rec_state.start_time = Some(std::time::Instant::now());
            Ok(())
        }
        Err(e) => Err(String::from(e)) // This converts AppError -> JSON String
    }
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, Mutex<RecordingState>>,
) -> Result<String, String> {
    let mut rec_state = state.lock().map_err(|e| e.to_string())?;
    
    if !rec_state.is_recording {
        return Err("Not recording".to_string());
    }

    rec_state.is_recording = false;
    let _ = rec_state.start_time.take();

    match crate::whisper::stop_recording() {
        Ok(text) => Ok(text),
        Err(e) => Err(String::from(e)) // AppError -> JSON String
    }
}

#[tauri::command]
pub async fn cancel_recording(
    state: State<'_, Mutex<RecordingState>>,
) -> Result<(), String> {
    let mut rec_state = state.lock().map_err(|e| e.to_string())?;
    
    rec_state.is_recording = false;
    rec_state.start_time = None;
    
    crate::whisper::cancel_recording()
        .map_err(|e| String::from(e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn download_models() -> Result<(), String> {
    crate::models::download_models()
        .await
        .map_err(|e| format!("Failed to download models: {}", e))
}

/// Zero-Click Auto-Initialization: Check and download models with progress tracking
#[tauri::command]
pub async fn initialize_app(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
) -> Result<String, String> {
    
    // Get app data directory
    let app_data_dir = app_handle.path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    // Ensure app data directory exists
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }
    
    let models_dir = app_data_dir.join("models");
    std::fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;
    
    let model_path = models_dir.join(models::WHISPER_MODEL_FILENAME);
    
    // PHASE 1: Check for file existence
    if !model_path.exists() {
        
        // Spawn download in background so we can return immediately
        let window_clone = window.clone();
        let app_handle_clone = app_handle.clone();
        let model_path_clone = model_path.clone();
        
        tauri::async_runtime::spawn(async move {
            // Download the model
            // Convert error to String immediately for thread safety
            let result = models::download_file_with_progress(
                models::WHISPER_MODEL_URL,
                &model_path_clone,
                &app_handle_clone,
            ).await.map_err(|e| e.to_string());
            
            match result {
                Ok(()) => {
                    // Verify integrity after download
                    let is_corrupt = match crate::whisper::verify_model_integrity(&model_path_clone) {
                        Ok(valid) => !valid,
                        Err(_) => true,
                    };
                    
                    if is_corrupt {
                        let _ = std::fs::remove_file(&model_path_clone);
                        // Re-download (this will emit progress events again)
                        let re_download_result = models::download_file_with_progress(
                            models::WHISPER_MODEL_URL,
                            &model_path_clone,
                            &app_handle_clone,
                        ).await.map_err(|e| e.to_string());
                        
                        if let Err(e) = re_download_result {
                            let _ = window_clone.emit("initialization-error", e);
                            return;
                        }
                        // Verify again after re-download
                        let is_still_corrupt = match crate::whisper::verify_model_integrity(&model_path_clone) {
                            Ok(valid) => !valid,
                            Err(_) => true,
                        };
                        if is_still_corrupt {
                            let _ = window_clone.emit("initialization-error", "Model verification failed after re-download".to_string());
                            return;
                        }
                    }
                    
                    let _ = window_clone.emit("initialization-complete", ());
                }
                Err(e) => {
                    let _ = window_clone.emit("initialization-error", e);
                }
            }
        });
        
        // Return immediately so frontend knows download is in progress
        return Ok("downloading".to_string());
    }
    
    // PHASE 2: Verify Integrity (Try to Load)
    // Attempt to load the model - if this fails, the file is corrupt
    let is_corrupt = match crate::whisper::verify_model_integrity(&model_path) {
        Ok(valid) => !valid,
        Err(_) => true,
    };
    
    if is_corrupt {
        // Try to delete corrupt file, but don't fail if it doesn't exist
        let _ = std::fs::remove_file(&model_path);
        
        // Spawn re-download in background
        let window_clone = window.clone();
        let app_handle_clone = app_handle.clone();
        let model_path_clone = model_path.clone();
        
        tauri::async_runtime::spawn(async move {
            let re_download_result = models::download_file_with_progress(
                models::WHISPER_MODEL_URL,
                &model_path_clone,
                &app_handle_clone,
            ).await.map_err(|e| e.to_string());
            
            if let Err(e) = re_download_result {
                let _ = window_clone.emit("initialization-error", e);
                return;
            }
            // Verify after re-download
            let is_still_corrupt = match crate::whisper::verify_model_integrity(&model_path_clone) {
                Ok(valid) => !valid,
                Err(_) => true,
            };
            if is_still_corrupt {
                let _ = window_clone.emit("initialization-error", "Model verification failed after re-download".to_string());
            } else {
                let _ = window_clone.emit("initialization-complete", ());
            }
        });
        
        return Ok("downloading".to_string());
    }
    
    Ok("ready".to_string())
}
