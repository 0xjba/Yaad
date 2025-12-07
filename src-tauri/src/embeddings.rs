use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rusqlite::{Connection, Result};
use std::sync::OnceLock;

static EMBEDDER: OnceLock<TextEmbedding> = OnceLock::new();

pub fn init_embedder() -> Result<(), Box<dyn std::error::Error>> {
    // FIX: Handle non_exhaustive struct by using mutable default
    let mut options = InitOptions::default();
    options.model_name = EmbeddingModel::AllMiniLML6V2;
    options.show_download_progress = true;
    
    let embedder = TextEmbedding::try_new(options)
        .map_err(|e| format!("Failed to initialize embedder: {}", e))?;
    
    EMBEDDER.set(embedder).map_err(|_| "Embedder already initialized")?;
    Ok(())
}

pub fn get_embedder() -> Result<&'static TextEmbedding, Box<dyn std::error::Error>> {
    EMBEDDER.get().ok_or_else(|| "Embedder not initialized. Call init_embedder() first.".into())
}

pub fn generate_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let embedder = get_embedder()?;
    let embeddings = embedder.embed(vec![text], None)?;
    
    if embeddings.is_empty() {
        return Err("No embeddings generated".into());
    }
    
    Ok(embeddings[0].clone())
}

pub fn store_embedding(conn: &Connection, rowid: i64, embedding: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    // sqlite-vec expects vectors as binary blobs (raw bytes)
    // Convert f32 slice to u8 slice using bytemuck for safe casting
    let embedding_bytes: &[u8] = bytemuck::cast_slice(embedding);
    
    // Insert into vec_memories virtual table
    // The rowid should match the memories table rowid
    conn.execute(
        "INSERT INTO vec_memories (rowid, embedding) VALUES (?1, ?2)",
        rusqlite::params![rowid, embedding_bytes],
    )
    .map_err(|e| format!("Failed to store embedding: {}", e))?;
    
    Ok(())
}

pub fn search_similar(
    conn: &Connection,
    query_embedding: &[f32],
    limit: i32,
) -> Result<Vec<(i64, f32)>, Box<dyn std::error::Error>> {
    // sqlite-vec uses vec_distance_cosine function (not the old distance() function)
    // Convert query embedding to binary format (raw bytes) using bytemuck
    let query_vector_bytes: &[u8] = bytemuck::cast_slice(query_embedding);
    
    // Search with increased limit to allow for ranking adjustments
    // We'll filter and rank results after retrieval
    let search_limit = (limit * 2).max(20); // Get more results for better ranking
    
    // THE FIX: Use vec_distance_cosine instead of distance()
    // sqlite-vec vec0 virtual table uses vec_distance_cosine function
    // Syntax: vec_distance_cosine(embedding_column, query_vector_bytes)
    // Returns cosine distance (lower is more similar, range 0-2)
    let mut stmt = conn.prepare(
        "SELECT rowid, vec_distance_cosine(embedding, ?1) as dist 
         FROM vec_memories 
         ORDER BY dist ASC
         LIMIT ?2"
    )
    .map_err(|e| format!("Failed to prepare search query: {}", e))?;
    
    let results = stmt.query_map(rusqlite::params![query_vector_bytes, search_limit], |row| {
        let rowid: i64 = row.get(0)?;
        let distance: f64 = row.get(1)?;
        Ok((rowid, distance))
    })
    .map_err(|e| format!("Failed to execute search: {}", e))?;
    
    // Collect and rank results
    let mut matches: Vec<(i64, f64)> = Vec::new();
    for result in results {
        matches.push(result?);
    }
    
    // Convert distance to similarity score with improved calculation
    // Cosine distance ranges from 0 to 2, where 0 is identical
    // Convert to similarity: similarity = 1 - (distance / 2)
    // Apply boost for very close matches
    let mut ranked_matches: Vec<(i64, f32)> = matches
        .into_iter()
        .map(|(rowid, distance)| {
            let normalized_distance = distance.min(2.0) / 2.0;
            let base_similarity = 1.0 - normalized_distance;
            
            // Boost very close matches (distance < 0.1)
            let similarity = if distance < 0.1 {
                base_similarity * 1.1 // 10% boost for very close matches
            } else {
                base_similarity
            };
            
            (rowid, similarity.min(1.0).max(0.0) as f32)
        })
        .collect();
    
    // Sort by similarity (descending) and take top results
    ranked_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked_matches.truncate(limit as usize);
    
    Ok(ranked_matches)
}

/// Generate embeddings for multiple texts (batch processing for performance)
/// This is more efficient than calling generate_embedding multiple times
pub fn generate_embeddings_batch(texts: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let embedder = get_embedder()?;
    
    if texts.is_empty() {
        return Err("No texts provided".into());
    }
    
    // Trim all texts and filter empty ones
    let trimmed: Vec<String> = texts
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    
    if trimmed.is_empty() {
        return Err("All texts are empty".into());
    }
    
    // Generate embeddings in batch (more efficient)
    let embeddings = embedder.embed(trimmed, None)?;
    
    if embeddings.is_empty() {
        return Err("No embeddings generated".into());
    }
    
    Ok(embeddings)
}

