use crate::index::WeiboIndexer;
use crate::storage::Storage;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, clap::Parser)]
pub struct DataDirConfig {
    #[clap(long, default_value_t = default_data_dir())]
    data_dir: String,
}

fn default_data_dir() -> String {
    // 有什么办法直接写 ~/.weise ？
    dirs::home_dir()
        .unwrap()
        .join(".weise")
        .to_str()
        .unwrap()
        .to_string()
}

impl DataDirConfig {
    pub fn storage(&self) -> Result<Storage, anyhow::Error> {
        let storage_path = self.storage_path();
        Storage::open(storage_path)
    }

    pub fn weibo_indexer(&self) -> Result<WeiboIndexer, anyhow::Error> {
        let weibo_indexer = WeiboIndexer::with_index_dir(self.index_dir())?;
        Ok(weibo_indexer)
    }

    pub fn ensure_data_dir_exists(&self) -> Result<(), anyhow::Error> {
        if !Path::new(&self.data_dir).exists() {
            fs::create_dir(&self.data_dir)?;
        }

        let index_dir = self.index_dir();
        if !index_dir.exists() {
            fs::create_dir(&index_dir)?;
        }
        Ok(())
    }

    fn storage_path(&self) -> PathBuf {
        Path::new(&self.data_dir).join("db.db")
    }

    fn index_dir(&self) -> PathBuf {
        Path::new(&self.data_dir).join("index")
    }
}

pub mod crawl;
pub mod index;
pub mod search;
pub mod settings;
pub mod tombstone;
