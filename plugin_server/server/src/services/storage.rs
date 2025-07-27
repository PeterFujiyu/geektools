use std::{path::PathBuf, sync::Arc};
use tokio::fs;
use uuid::Uuid;

use crate::utils::config::Config;

pub struct StorageService {
    config: Arc<Config>,
    upload_dir: PathBuf,
}

impl StorageService {
    pub fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let upload_dir = PathBuf::from(&config.storage.upload_path);
        
        // Create upload directory if it doesn't exist
        std::fs::create_dir_all(&upload_dir)?;

        Ok(Self {
            config,
            upload_dir,
        })
    }

    pub async fn store_plugin_file(
        &self,
        data: Vec<u8>,
        plugin_id: &str,
        version: &str,
    ) -> anyhow::Result<String> {
        let plugin_dir = self.upload_dir.join("plugins").join(plugin_id).join(version);
        fs::create_dir_all(&plugin_dir).await?;

        let filename = format!("{}-{}.tar.gz", plugin_id, version);
        let file_path = plugin_dir.join(&filename);

        fs::write(&file_path, data).await?;

        Ok(file_path.to_string_lossy().to_string())
    }

    pub async fn store_temporary_file(&self, data: Vec<u8>) -> anyhow::Result<PathBuf> {
        let temp_dir = self.upload_dir.join("temp");
        fs::create_dir_all(&temp_dir).await?;

        let filename = format!("{}.tmp", Uuid::new_v4());
        let file_path = temp_dir.join(filename);

        fs::write(&file_path, data).await?;

        Ok(file_path)
    }

    pub async fn cleanup_temporary_file(&self, file_path: &PathBuf) -> anyhow::Result<()> {
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
        Ok(())
    }

    pub fn get_file_url(&self, file_path: &str) -> String {
        if self.config.storage.use_cdn {
            format!("{}/{}", self.config.storage.cdn_base_url, file_path)
        } else {
            format!("/downloads/{}", file_path)
        }
    }
}