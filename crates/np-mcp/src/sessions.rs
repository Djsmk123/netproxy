//! Session save/load functionality.
//!
//! Enables AI to save and restore NetProxy sessions (flows + mocks).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub created_at: String,
    pub mocks: Vec<Value>,
    pub flows: Vec<Value>,
    pub metadata: Value,
}

/// Get the sessions directory.
fn sessions_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("sessions")
}

/// Save a session.
pub fn save_session(
    data_dir: &Path,
    name: &str,
    mocks: Vec<Value>,
    flows: Vec<Value>,
    metadata: Option<Value>,
) -> Result<Value, SessionError> {
    let sessions_path = sessions_dir(data_dir);
    std::fs::create_dir_all(&sessions_path)?;

    let session = Session {
        name: name.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        mocks,
        flows,
        metadata: metadata.unwrap_or(serde_json::json!({})),
    };

    let session_file = sessions_path.join(format!("{}.json", sanitize_filename(name)));
    let json = serde_json::to_string_pretty(&session)?;
    std::fs::write(&session_file, json)?;

    Ok(serde_json::json!({
        "success": true,
        "name": name,
        "path": session_file.to_string_lossy(),
        "mocks_count": session.mocks.len(),
        "flows_count": session.flows.len()
    }))
}

/// Load a session.
pub fn load_session(data_dir: &Path, name: &str) -> Result<Session, SessionError> {
    let sessions_path = sessions_dir(data_dir);
    let session_file = sessions_path.join(format!("{}.json", sanitize_filename(name)));

    if !session_file.exists() {
        return Err(SessionError::NotFound(name.to_string()));
    }

    let json = std::fs::read_to_string(&session_file)?;
    let session: Session = serde_json::from_str(&json)?;

    Ok(session)
}

/// List all saved sessions.
pub fn list_sessions(data_dir: &Path) -> Result<Value, SessionError> {
    let sessions_path = sessions_dir(data_dir);

    if !sessions_path.exists() {
        return Ok(serde_json::json!({
            "sessions": [],
            "count": 0
        }));
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(&sessions_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<Session>(&json) {
                    sessions.push(serde_json::json!({
                        "name": session.name,
                        "created_at": session.created_at,
                        "mocks_count": session.mocks.len(),
                        "flows_count": session.flows.len()
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "sessions": sessions,
        "count": sessions.len()
    }))
}

/// Delete a session.
pub fn delete_session(data_dir: &Path, name: &str) -> Result<Value, SessionError> {
    let sessions_path = sessions_dir(data_dir);
    let session_file = sessions_path.join(format!("{}.json", sanitize_filename(name)));

    if !session_file.exists() {
        return Err(SessionError::NotFound(name.to_string()));
    }

    std::fs::remove_file(&session_file)?;

    Ok(serde_json::json!({
        "success": true,
        "deleted": name
    }))
}

/// Sanitize filename to prevent path traversal.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
