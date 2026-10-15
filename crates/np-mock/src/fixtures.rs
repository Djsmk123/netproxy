//! Fixture resolution and caching.

use crate::schema::BodySource;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

/// Resolves fixture bodies from files or inline content, with caching.
pub struct FixtureResolver {
    fixtures_dir: PathBuf,
    cache: HashMap<String, CachedFixture>,
}

struct CachedFixture {
    content: Bytes,
    sha256: String,
}

impl FixtureResolver {
    /// Create a new resolver that loads files relative to the given directory.
    pub fn new(fixtures_dir: PathBuf) -> Self {
        Self {
            fixtures_dir,
            cache: HashMap::new(),
        }
    }

    /// Load fixture content from the given source.
    /// Files are cached; inline content is returned directly.
    pub fn load(&mut self, source: &BodySource) -> Result<Bytes, FixtureError> {
        match source {
            BodySource::File { file } => self.load_file(file),
            BodySource::Inline { inline } => Ok(Bytes::from(inline.clone())),
        }
    }

    fn load_file(&mut self, file: &str) -> Result<Bytes, FixtureError> {
        // Check cache first
        if let Some(cached) = self.cache.get(file) {
            return Ok(cached.content.clone());
        }

        // Load from filesystem
        let path = self.fixtures_dir.join(file);
        let content = std::fs::read(&path).map_err(|e| FixtureError::NotFound {
            file: file.to_string(),
            path: path.clone(),
            source: e,
        })?;

        let bytes = Bytes::from(content);
        let sha256 = compute_sha256(&bytes);

        // Cache for future use
        self.cache.insert(
            file.to_string(),
            CachedFixture {
                content: bytes.clone(),
                sha256,
            },
        );

        Ok(bytes)
    }

    /// Get the SHA-256 hash of a cached fixture, if available.
    #[allow(dead_code)]
    pub fn get_hash(&self, file: &str) -> Option<&str> {
        self.cache.get(file).map(|c| c.sha256.as_str())
    }
}

fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Error loading a fixture.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("fixture not found: {file} (tried {path:?}): {source}")]
    NotFound {
        file: String,
        path: PathBuf,
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_inline_content() {
        let dir = tempdir().unwrap();
        let mut resolver = FixtureResolver::new(dir.path().to_path_buf());

        let source = BodySource::Inline {
            inline: "Hello, world!".to_string(),
        };

        let content = resolver.load(&source).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn load_file_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"file content").unwrap();

        let mut resolver = FixtureResolver::new(dir.path().to_path_buf());

        let source = BodySource::File {
            file: "test.txt".to_string(),
        };

        let content = resolver.load(&source).unwrap();
        assert_eq!(content, "file content");
    }

    #[test]
    fn caching_works() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"cached content").unwrap();

        let mut resolver = FixtureResolver::new(dir.path().to_path_buf());

        let source = BodySource::File {
            file: "test.txt".to_string(),
        };

        // First load - reads from file
        let content1 = resolver.load(&source).unwrap();

        // Delete the file
        fs::remove_file(&file_path).unwrap();

        // Second load - should still work from cache
        let content2 = resolver.load(&source).unwrap();

        assert_eq!(content1, content2);
        assert_eq!(content2, "cached content");
    }

    #[test]
    fn missing_file_error() {
        let dir = tempdir().unwrap();
        let mut resolver = FixtureResolver::new(dir.path().to_path_buf());

        let source = BodySource::File {
            file: "missing.txt".to_string(),
        };

        let result = resolver.load(&source);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("fixture not found"));
    }

    #[test]
    fn sha256_computed() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let mut resolver = FixtureResolver::new(dir.path().to_path_buf());

        let source = BodySource::File {
            file: "test.txt".to_string(),
        };

        resolver.load(&source).unwrap();

        // Verify SHA-256 is computed and cached
        let hash = resolver.get_hash("test.txt").unwrap();
        // SHA-256 of "test"
        assert_eq!(
            hash,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }
}
