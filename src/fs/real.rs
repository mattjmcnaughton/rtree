use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::task;

use crate::models::{EntryKind, FsEntry};

use super::FileSystem;

pub struct RealFileSystem;

#[async_trait]
impl FileSystem for RealFileSystem {
    async fn read_dir(&self, dir: &Path) -> Result<Vec<FsEntry>> {
        let dir = dir.to_path_buf();
        task::spawn_blocking(move || {
            let mut entries = Vec::new();
            for entry in std::fs::read_dir(&dir)?.filter_map(|e| e.ok()) {
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };
                let kind = if file_type.is_symlink() {
                    EntryKind::Symlink
                } else if file_type.is_dir() {
                    EntryKind::Directory
                } else if file_type.is_file() {
                    EntryKind::File
                } else {
                    EntryKind::Other
                };

                entries.push(FsEntry {
                    path: entry.path(),
                    name: entry.file_name().to_string_lossy().into_owned(),
                    kind,
                });
            }
            Ok(entries)
        })
        .await?
    }
}
