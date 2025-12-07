use std::path::PathBuf;
use std::fs;
use std::io::Write;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

// Model URLs from Hugging Face Hub
pub const WHISPER_MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin";
pub const WHISPER_MODEL_FILENAME: &str = "ggml-small.en.bin";
const EMBEDDING_MODEL_URL: &str = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model_quantized.onnx";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub file: String,
    pub progress: f32,
    pub status: String,
}

// New progress payload for streaming downloads
#[derive(Clone, serde::Serialize)]
pub struct StreamingProgress {
    pub percentage: u64,
    pub current: u64,
    pub total: u64,
}

/// Get the models directory path in app data directory
pub fn get_models_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    use crate::db;
    // Get app data directory (set during initialization)
    let app_data = db::get_app_data_dir()
        .ok_or("App data directory not initialized")?;
    let models_dir = app_data.join("models");
    if !models_dir.exists() {
        fs::create_dir_all(&models_dir)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
    }
    Ok(models_dir)
}

/// Download a file from URL (basic version, no progress)
async fn download_file(
    url: &str,
    dest_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    
    // Atomic Download Strategy:
    // Create a fixed temp file name based on the destination
    let mut temp_path = dest_path.clone();
    
    // Simple way to append extension without nesting
    let file_name = temp_path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("download")
        .to_string();
        
    temp_path.set_file_name(format!("{}.part", file_name));
    
    let mut file = fs::File::create(&temp_path)?;
    let content = response.bytes().await?;
    file.write_all(&content)?;
    
    // 2. Flush & Sync (Ensure data is on disk)
    file.flush()?;
    file.sync_all()?;
    drop(file); // Close handle before rename
    
    // 3. Atomic Rename
    fs::rename(&temp_path, dest_path)?;
    
    Ok(())
}

/// Download a file from URL with progress tracking (for UI updates)
pub async fn download_file_with_progress(
    url: &str,
    dest_path: &PathBuf,
    app_handle: &AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);
    
    // Atomic Download Strategy:
    // Create a fixed temp file name based on the destination
    let mut temp_path = dest_path.clone();
    
    // Simple way to append extension without nesting
    let file_name = temp_path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("download")
        .to_string();
        
    temp_path.set_file_name(format!("{}.part", file_name));
    
    let mut file = fs::File::create(&temp_path)?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_percent = 0;
    
    use futures_util::StreamExt;
    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk)?;
        
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let percentage = (downloaded * 100) / total_size;
            
            // Emit event only on percentage change to save resources
            if percentage > last_percent {
                let _ = app_handle.emit("download-progress", StreamingProgress {
                    percentage,
                    current: downloaded,
                    total: total_size,
                });
                last_percent = percentage;
            }
        }
    }
    
    // 2. Flush & Sync (Ensure data is on disk)
    file.flush()?;
    file.sync_all()?;
    drop(file); // Close handle before rename
    
    // 3. Atomic Rename
    // If this succeeds, the file is guaranteed to be complete
    fs::rename(&temp_path, dest_path)?;
    
    Ok(())
}

/// Download all required models
pub async fn download_models() -> Result<(), Box<dyn std::error::Error>> {
    let models_dir = get_models_dir()?;
    
    // Download Whisper Model
    let model_path = models_dir.join(WHISPER_MODEL_FILENAME);
    if !model_path.exists() {
        download_file(WHISPER_MODEL_URL, &model_path).await?;
    }
    
    // Download Embedding model (ONNX)
    let embedding_path = models_dir.join("model_quantized.onnx");
    if !embedding_path.exists() {
        download_file(EMBEDDING_MODEL_URL, &embedding_path).await?;
    }
    
    Ok(())
}
