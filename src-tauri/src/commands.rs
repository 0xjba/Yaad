use crate::db;
use crate::embeddings;
use crate::models;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{State, Manager, Emitter};
use uuid::Uuid;
use swift_rs::{SRString, swift};

swift!(
    pub fn capture_active_window() -> SRString;
    pub fn fetch_metadata_only() -> SRString;
    pub fn check_accessibility_permissions() -> bool;
);

#[derive(Debug, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub screenshot_path: Option<String>,
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

pub struct VisualState {
    pub embedder: Mutex<Option<crate::visuals::VisualEmbedder>>,
}

// NEW COMMAND: Allow frontend to check if we have permissions
#[tauri::command]
pub async fn check_permissions() -> Result<bool, String> {
    let granted = unsafe { check_accessibility_permissions() };
    Ok(granted)
}

#[tauri::command]
pub async fn save_memory(
    app_handle: tauri::AppHandle,
    content: String,
    ocr_text: Option<String>,
    app_name: Option<String>,
    screenshot: Option<String>, // Base64
    duration_sec: Option<i32>,
    context_url: Option<String>,
    context_note: Option<String>,
    visual_state: State<'_, VisualState>,
) -> Result<String, String> {
    println!("save_memory called with content length: {}", content.len());
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
    let mut saved_screenshot_path: Option<String> = None;
    
    // Decode screenshot early if provided, as we need it for both disk and embedding
    let mut decoded_img: Option<image::DynamicImage> = None;
    if let Some(base64_img) = &screenshot {
        println!("Screenshot provided, length: {}", base64_img.len());
        if let Some(img_bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_img).ok() {
            println!("Screenshot decoded successfully, bytes: {}", img_bytes.len());
            // 1. Save to Disk
            if let Ok(app_dir) = app_handle.path().app_local_data_dir() {
                let screenshots_dir = app_dir.join("screenshots");
                if !screenshots_dir.exists() {
                    let _ = std::fs::create_dir_all(&screenshots_dir);
                }
                let filename = format!("{}.jpg", id);
                let file_path = screenshots_dir.join(&filename);
                if std::fs::write(&file_path, &img_bytes).is_ok() {
                    saved_screenshot_path = Some(filename);
                    println!("Screenshot saved to: {:?}", file_path);
                } else {
                    eprintln!("Failed to write screenshot to disk");
                }
            }
            // 2. Load for embedding
            decoded_img = image::load_from_memory(&img_bytes).ok();
        }
    }
    
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
        "INSERT INTO memories (id, content, screenshot_path, ocr_text, app_name, duration_sec, context_url, context_note) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, content, saved_screenshot_path, ocr_text, app_name, duration_sec, context_url, context_note],
    )
    .map_err(|e| {
        eprintln!("Database insert failed: {}", e);
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

    // Generate and store visual embedding if screenshot was decoded
    if let Some(img) = decoded_img {
        let mut embedder_lock = visual_state.embedder.lock().map_err(|e| e.to_string())?;
        if embedder_lock.is_none() {
            let models_dir = crate::models::get_models_dir().map_err(|e| e.to_string())?;
            let clip_path = models_dir.join(crate::models::CLIP_MODEL_FILENAME);
            if clip_path.exists() {
                *embedder_lock = Some(crate::visuals::VisualEmbedder::new(&clip_path).map_err(|e| e.to_string())?);
            }
        }
        
        if let Some(embedder) = embedder_lock.as_ref() {
            if let Ok(visual_embedding) = embedder.generate_embedding(img) {
                let vec_embedding: Vec<f32> = visual_embedding;
                // Store in vec_visuals
                let blob = bytemuck::cast_slice::<f32, u8>(&vec_embedding);
                conn.execute(
                    "INSERT INTO vec_visuals (rowid, embedding) VALUES (?1, ?2)",
                    rusqlite::params![rowid, blob],
                ).map_err(|e| format!("Failed to store visual embedding: {}", e))?;
            }
        }
    }

    println!("Memory saved successfully with id: {}", id);
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
        "SELECT rowid, id, content, screenshot_path, duration_sec, context_url, context_note, created_at 
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
                screenshot_path: row.get(3)?,
                duration_sec: row.get(4)?,
                context_url: row.get(5)?,
                context_note: row.get(6)?,
                created_at: row.get(7)?,
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
            "SELECT id, content, screenshot_path, duration_sec, context_url, context_note, created_at 
             FROM memories WHERE id = ?1 AND is_deleted = 0",
            [id],
            |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    screenshot_path: row.get(2)?,
                    duration_sec: row.get(3)?,
                    context_url: row.get(4)?,
                    context_note: row.get(5)?,
                    created_at: row.get(6)?,
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
pub async fn get_contextual_suggestions(
    visual_state: State<'_, VisualState>,
) -> Result<Vec<SearchResult>, String> {
    // 1. Capture current window
    let result = unsafe { capture_active_window() };
    let result_str = result.to_string();
    if result_str.starts_with("ERROR:") {
        return Err(result_str);
    }
    
    let parsed: serde_json::Value = serde_json::from_str(&result_str).map_err(|e| e.to_string())?;
    let screenshot_base64 = parsed["image"].as_str().ok_or("No image in capture")?;
    let app_name = parsed["app_name"].as_str().unwrap_or("");
    let ocr_text = parsed["ocr"].as_str().unwrap_or("");
    let url = parsed["url"].as_str().unwrap_or("");

    let conn = db::get_connection().map_err(|e| e.to_string())?;
    let mut all_results = Vec::new();

    // 2. Visual search
    if let Some(img_bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, screenshot_base64).ok() {
        if let Ok(img) = image::load_from_memory(&img_bytes) {
            let mut embedder_lock = visual_state.embedder.lock().map_err(|e| e.to_string())?;
            if embedder_lock.is_none() {
                let models_dir = crate::models::get_models_dir().map_err(|e| e.to_string())?;
                let clip_path = models_dir.join(crate::models::CLIP_MODEL_FILENAME);
                if clip_path.exists() {
                    *embedder_lock = Some(crate::visuals::VisualEmbedder::new(&clip_path).map_err(|e| e.to_string())?);
                }
            }
            
            if let Some(embedder) = embedder_lock.as_ref() {
                if let Ok(visual_embedding) = embedder.generate_embedding(img) {
                    let vec_embedding: Vec<f32> = visual_embedding;
                    let blob = bytemuck::cast_slice::<f32, u8>(&vec_embedding);
                    let mut stmt = conn.prepare(
                        "SELECT rowid, distance FROM vec_visuals WHERE embedding MATCH ?1 ORDER BY distance LIMIT 5"
                    ).map_err(|e| e.to_string())?;
                    
                    let matches = stmt.query_map([blob], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, f32>(1)?))
                    }).map_err(|e| e.to_string())?;

                    for m in matches {
                        if let Ok((rowid, distance)) = m {
                            // Convert distance to similarity (1.0 - distance for normalized vectors)
                            let similarity = 1.0 - distance;
                            if similarity > 0.7 {
                                if let Ok(memory) = fetch_memory_by_rowid(&conn, rowid) {
                                    all_results.push(SearchResult { memory, similarity });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. OCR/App Name/URL text search (Passive)
    let text_query = format!("{} {} {}", app_name, url, ocr_text);
    if !text_query.trim().is_empty() {
        let text_embedding = crate::embeddings::generate_embedding(&text_query).map_err(|e| e.to_string())?;
        let matches = crate::embeddings::search_similar(&conn, &text_embedding, 5).map_err(|e| e.to_string())?;
        for (rowid, similarity) in matches {
            if similarity > 0.75 {
                if let Ok(memory) = fetch_memory_by_rowid(&conn, rowid) {
                    all_results.push(SearchResult { memory, similarity });
                }
            }
        }
    }

    // Sort and unique
    all_results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    all_results.dedup_by(|a, b| a.memory.id == b.memory.id);

    Ok(all_results.into_iter().take(5).collect())
}

fn fetch_memory_by_rowid(conn: &rusqlite::Connection, rowid: i64) -> Result<Memory, rusqlite::Error> {
    conn.query_row(
        "SELECT id, content, screenshot_path, duration_sec, context_url, context_note, created_at 
         FROM memories WHERE rowid = ?1 AND is_deleted = 0",
        [rowid],
        |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                screenshot_path: row.get(2)?,
                duration_sec: row.get(3)?,
                context_url: row.get(4)?,
                context_note: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )
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

#[tauri::command]
pub async fn capture_active_window_cmd() -> Result<String, String> {
    let result = unsafe { capture_active_window() };
    let result_str = result.to_string();
    if result_str.starts_with("ERROR:") {
        return Err(result_str);
    }
    Ok(result_str)
}

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
    
    let whisper_path = models_dir.join(models::WHISPER_MODEL_FILENAME);
    let clip_path = models_dir.join(models::CLIP_MODEL_FILENAME);
    
    // PHASE 1: Check for file existence
    if !whisper_path.exists() || !clip_path.exists() {
        // Spawn download in background
        let window_clone = window.clone();
        let app_handle_clone = app_handle.clone();
        
        tauri::async_runtime::spawn(async move {
            let models_to_download = [
                (models::WHISPER_MODEL_URL, models::WHISPER_MODEL_FILENAME),
                (models::CLIP_MODEL_URL, models::CLIP_MODEL_FILENAME),
            ];

            for (url, filename) in models_to_download {
                let dest_path = app_handle_clone.path().app_local_data_dir().unwrap().join("models").join(filename);
                if !dest_path.exists() {
                    let _ = models::download_file_with_progress(url, &dest_path, &app_handle_clone).await;
                }
            }
            
            let _ = window_clone.emit("initialization-complete", ());
        });
        
        return Ok("downloading".to_string());
    }
    
    Ok("ready".to_string())
}
