use std::path::{Path, PathBuf};

use async_trait::async_trait;
use garudust_core::{
    error::AgentError,
    memory::{MemoryContent, MemoryStore},
};

pub struct FileMemoryStore {
    memory_path: PathBuf,
    profile_path: PathBuf,
}

impl FileMemoryStore {
    pub fn new(home_dir: &Path) -> Self {
        let memories = home_dir.join("memories");
        Self {
            memory_path: memories.join("MEMORY.md"),
            profile_path: memories.join("USER.md"),
        }
    }

    async fn read_file(&self, path: &PathBuf) -> Result<String, AgentError> {
        match tokio::fs::read_to_string(path).await {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(AgentError::Other(anyhow::anyhow!("{e}"))),
        }
    }

    async fn write_file(&self, path: &PathBuf, content: &str) -> Result<(), AgentError> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AgentError::Other(anyhow::anyhow!("{e}")))?;
        }
        tokio::fs::write(path, content)
            .await
            .map_err(|e| AgentError::Other(anyhow::anyhow!("{e}")))
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
