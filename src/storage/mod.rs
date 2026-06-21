use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use sled::Db;
use crate::parser::ast::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopSnapshot {
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

    let snapshot = LoopSnapshot {
        session_id: session_id.to_string(),
        variables: variables.clone(),
    };

    let bytes = bincode::serialize(&snapshot)?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    db.insert(hash.as_bytes(), bytes)?;

    let session_key = format!("session:{}:latest", session_id);
    db.insert(session_key.as_bytes(), hash.as_bytes())?;
    db.flush()?;

    Ok(hash)
}

pub fn get_snapshot(hash: &str) -> Result<LoopSnapshot, StorageError> {
    let db = get_db();
    let bytes = db.get(hash.as_bytes())?
        .ok_or_else(|| StorageError::NotFound(hash.to_string()))?;
    let snapshot: LoopSnapshot = bincode::deserialize(&bytes)?;
    Ok(snapshot)
}

pub fn get_latest_snapshot_hash(session_id: &str) -> Result<Option<String>, StorageError> {
    let db = get_db();
    let session_key = format!("session:{}:latest", session_id);
    if let Some(hash_bytes) = db.get(session_key.as_bytes())? {
        let hash = String::from_utf8(hash_bytes.to_vec())
            .map_err(|_| StorageError::NotFound("Invalid hash bytes".to_string()))?;
        Ok(Some(hash))
    } else {
        Ok(None)
    }
}
