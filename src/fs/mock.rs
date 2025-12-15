use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::models::FsEntry;

use super::FileSystem;

#[derive(Clone, Debug)]
enum Response {
    Ok(Vec<FsEntry>),
    Err(String),
}

#[derive(Clone, Default)]
pub struct MockFileSystem {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    responses: HashMap<PathBuf, Response>,
    calls: Vec<PathBuf>,
}

impl MockFileSystem {
    pub fn set_dir_entries(&self, dir: impl Into<PathBuf>, entries: Vec<FsEntry>) {
        let mut inner = self.inner.lock().expect("mock fs lock");
        inner.responses.insert(dir.into(), Response::Ok(entries));
    }

    pub fn set_error(&self, dir: impl Into<PathBuf>, message: impl Into<String>) {
        let mut inner = self.inner.lock().expect("mock fs lock");
        inner
            .responses
            .insert(dir.into(), Response::Err(message.into()));
    }

    pub fn calls(&self) -> Vec<PathBuf> {
        let inner = self.inner.lock().expect("mock fs lock");
        inner.calls.clone()
    }
}

#[async_trait]
impl FileSystem for MockFileSystem {
    async fn read_dir(&self, dir: &Path) -> Result<Vec<FsEntry>> {
        let mut inner = self.inner.lock().expect("mock fs lock");
        inner.calls.push(dir.to_path_buf());

        match inner.responses.get(dir) {
            Some(Response::Ok(entries)) => Ok(entries.clone()),
            Some(Response::Err(message)) => Err(anyhow!("{message}")),
            None => Err(anyhow!("no mock response for {}", dir.display())),
        }
    }
}
