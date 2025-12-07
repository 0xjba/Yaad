#[cfg(test)]
mod tests {
    use crate::db;
    use rusqlite::Connection;

    #[test]
    fn test_database_initialization() {
        // Test that database can be initialized
        let result = db::init_db();
        assert!(result.is_ok(), "Database initialization should succeed");
    }

    #[test]
    fn test_database_connection() {
        // Test that we can get a database connection
        let conn_result = db::get_connection();
        assert!(conn_result.is_ok(), "Should be able to get database connection");
    }

    #[test]
    fn test_memory_table_exists() {
        // Test that memories table exists after initialization
        let conn = db::get_connection().expect("Failed to get connection");
        let result: Result<String, _> = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='memories'",
            [],
            |row| row.get(0),
        );
        assert!(result.is_ok(), "memories table should exist");
        assert_eq!(result.unwrap(), "memories");
    }

    #[test]
    fn test_vec_memories_table_exists() {
        // Test that vec_memories virtual table exists
        let conn = db::get_connection().expect("Failed to get connection");
        let result: Result<String, _> = conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='vec_memories'",
            [],
            |row| row.get(0),
        );
        assert!(result.is_ok(), "vec_memories virtual table should exist");
    }

    #[test]
    fn test_memory_insert() {
        // Test inserting a memory
        let conn = db::get_connection().expect("Failed to get connection");
        let result = conn.execute(
            "INSERT INTO memories (id, content, duration_sec) VALUES (?1, ?2, ?3)",
            rusqlite::params!["test-id-1", "Test memory content", 5],
        );
        assert!(result.is_ok(), "Should be able to insert memory");
        
        // Clean up
        let _ = conn.execute("DELETE FROM memories WHERE id = 'test-id-1'", []);
    }

    #[test]
    fn test_memory_retrieve() {
        // Test retrieving a memory
        let conn = db::get_connection().expect("Failed to get connection");
        
        // Insert test data
        conn.execute(
            "INSERT INTO memories (id, content) VALUES (?1, ?2)",
            rusqlite::params!["test-id-2", "Test content for retrieval"],
        )
        .expect("Failed to insert test memory");
        
        // Retrieve it
        let result: Result<String, _> = conn.query_row(
            "SELECT content FROM memories WHERE id = 'test-id-2'",
            [],
            |row| row.get(0),
        );
        assert!(result.is_ok(), "Should be able to retrieve memory");
        assert_eq!(result.unwrap(), "Test content for retrieval");
        
        // Clean up
        let _ = conn.execute("DELETE FROM memories WHERE id = 'test-id-2'", []);
    }

    #[test]
    fn test_soft_delete() {
        // Test soft delete functionality
        let conn = db::get_connection().expect("Failed to get connection");
        
        // Insert test data
        conn.execute(
            "INSERT INTO memories (id, content) VALUES (?1, ?2)",
            rusqlite::params!["test-id-3", "Test content for delete"],
        )
        .expect("Failed to insert test memory");
        
        // Soft delete
        conn.execute(
            "UPDATE memories SET is_deleted = 1 WHERE id = 'test-id-3'",
            [],
        )
        .expect("Failed to soft delete");
        
        // Verify it's marked as deleted
        let result: Result<i32, _> = conn.query_row(
            "SELECT is_deleted FROM memories WHERE id = 'test-id-3'",
            [],
            |row| row.get(0),
        );
        assert!(result.is_ok(), "Should be able to check is_deleted");
        assert_eq!(result.unwrap(), 1, "Memory should be marked as deleted");
        
        // Clean up
        let _ = conn.execute("DELETE FROM memories WHERE id = 'test-id-3'", []);
    }
}

