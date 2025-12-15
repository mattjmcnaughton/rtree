mod real;

#[cfg(test)]
mod mock;

pub use real::RealFileSystem;

#[cfg(test)]
pub use mock::MockFileSystem;

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

use crate::models::FsEntry;

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read_dir(&self, dir: &Path) -> Result<Vec<FsEntry>>;
}
