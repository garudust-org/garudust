use std::path::{Path, PathBuf};

use async_trait::async_trait;
use garudust_core::{
    config::MemoryExpiryConfig,
    error::AgentError,
    memory::{MemoryContent, MemoryStore},
};

pub struct FileMemoryStore {
    memory_path: PathBuf,
    profile_path: PathBuf,
    /// Serialises concurrent writes so two platform adapters cannot interleave
    /// partial content into the same file. Combined with atomic rename so a
    /// crash mid-write cannot leave a truncated file on disk.
    write_lock: tokio::sync::Mutex<()>,
}

impl FileMemoryStore {
    pub fn new(home_dir: &Path) -> Self {
        let memories = home_dir.join("memories");
        Self {
            memory_path: memories.join("MEMORY.md"),
            profile_path: memories.join("USER.md"),
            write_lock: tokio::sync::Mutex::new(()),
        }
    }

    async fn read_file(&self, path: &PathBuf) -> Result<String, AgentError> {
        match tokio::fs::read_to_string(path).await {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(AgentError::Other(anyhow::anyhow!("{e}"))),
        }
    }

    /// Write `content` to `path` atomically:
    /// 1. Acquire the write lock (serialises concurrent callers).
    /// 2. Write to `<path>.tmp` in the same directory.
    /// 3. Rename `.tmp` → `path` (atomic on POSIX; best-effort on Windows).
    ///
    /// A crash after step 2 but before step 3 leaves a stale `.tmp` file that
    /// is harmless and will be overwritten on the next write.
    async fn write_file(&self, path: &PathBuf, content: &str) -> Result<(), AgentError> {
        let _guard = self.write_lock.lock().await;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AgentError::Other(anyhow::anyhow!("{e}")))?;
        }
        // Build the tmp path by appending ".tmp" to the full path string so
        // `MEMORY.md` becomes `MEMORY.md.tmp` — clearly a temp file, same dir.
        let tmp_path = PathBuf::from(format!("{}.tmp", path.display()));
        tokio::fs::write(&tmp_path, content)
            .await
            .map_err(|e| AgentError::Other(anyhow::anyhow!("write tmp: {e}")))?;
        tokio::fs::rename(&tmp_path, path)
            .await
            .map_err(|e| AgentError::Other(anyhow::anyhow!("rename tmp: {e}")))
    }

    /// Expire old entries from MEMORY.md according to `config`.
    /// Returns the number of entries removed (0 means no file write occurred).
    pub async fn expire_entries(&self, config: &MemoryExpiryConfig) -> Result<usize, AgentError> {
        let mut mem = self.read_memory().await?;
        let removed = mem.expire(config);
        if removed > 0 {
            self.write_memory(&mem).await?;
        }
        Ok(removed)
    }
}

#[async_trait]
impl MemoryStore for FileMemoryStore {
    async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
        let raw = self.read_file(&self.memory_path).await?;
        Ok(MemoryContent::parse(&raw))
    }

    async fn write_memory(&self, content: &MemoryContent) -> Result<(), AgentError> {
        self.write_file(&self.memory_path, &content.serialize())
            .await
    }

    async fn read_user_profile(&self) -> Result<String, AgentError> {
        self.read_file(&self.profile_path).await
    }

    async fn write_user_profile(&self, content: &str) -> Result<(), AgentError> {
        self.write_file(&self.profile_path, content).await
    }
}
