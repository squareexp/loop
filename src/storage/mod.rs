use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use sled::Db;
use crate::parser::ast::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopState {
    pub session_id: String,
    pub variables: HashMap<String, Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("Snapshot not found for hash: {0}")]
    NotFound(String),
}

static DB: OnceLock<Db> = OnceLock::new();

pub fn get_db() -> &'static Db {
    DB.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let path = PathBuf::from(home).join(".loop").join("store");
        std::fs::create_dir_all(&path).ok();
        sled::open(&path).expect("Failed to open sled database")
    })
}

pub fn save_snapshot(session_id: &str, variables: &HashMap<String, Value>) -> Result<String, StorageError> {
    let db = get_db();
    
    let state = LoopState {
        session_id: session_id.to_string(),
        variables: variables.clone(),
    };
    
    let bytes = bincode::serialize(&state)?;
    
    // Hash the snapshot payload
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());
    
    // Store in sled under the hash
    db.insert(hash.as_bytes(), bytes)?;
    
    // Track the latest snapshot for this session
    let session_key = format!("session:{}:latest", session_id);
    db.insert(session_key.as_bytes(), hash.as_bytes())?;
    
    db.flush()?;
    
    Ok(hash)
}

pub fn get_snapshot(hash: &str) -> Result<LoopState, StorageError> {
    let db = get_db();
    let opt_bytes = db.get(hash.as_bytes())?;
    let bytes = opt_bytes.ok_or_else(|| StorageError::NotFound(hash.to_string()))?;
    let state: LoopState = bincode::deserialize(&bytes)?;
    Ok(state)
}

pub fn get_latest_snapshot_hash(session_id: &str) -> Result<Option<String>, StorageError> {
    let db = get_db();
    let session_key = format!("session:{}:latest", session_id);
    let opt_hash_bytes = db.get(session_key.as_bytes())?;
    if let Some(hash_bytes) = opt_hash_bytes {
        let hash = String::from_utf8(hash_bytes.to_vec())
            .map_err(|_| StorageError::NotFound("Invalid session hash string".to_string()))?;
        Ok(Some(hash))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_save_and_get_snapshot() {
        let mut variables = HashMap::new();
        variables.insert("counter".to_string(), Value::Integer(42));
        variables.insert("status".to_string(), Value::String("running".to_string()));

        // Warm up the database to initialize file locks/flush threads
        let _ = save_snapshot("test-session", &variables).unwrap();

        // Run benchmark
        let start = Instant::now();
        let hash = save_snapshot("test-session", &variables).unwrap();
        let duration = start.elapsed();
        println!("Save snapshot took: {:?}", duration);
        assert!(duration.as_micros() < 2000, "Snapshot save took longer than 2ms: {:?}", duration);

        let start_get = Instant::now();
        let retrieved = get_snapshot(&hash).unwrap();
        let duration_get = start_get.elapsed();
        println!("Get snapshot took: {:?}", duration_get);
        assert!(duration_get.as_micros() < 2000, "Snapshot get took longer than 2ms: {:?}", duration_get);

        assert_eq!(retrieved.session_id, "test-session");
        assert_eq!(retrieved.variables.get("counter"), Some(&Value::Integer(42)));
    }
}
