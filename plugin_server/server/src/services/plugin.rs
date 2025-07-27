use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{collections::HashMap, io::Read, sync::Arc};
use tar::Archive;
use validator::Validate;

use crate::{
    models::{
        CreatePluginRequest, Plugin, PluginDetailResponse, PluginDependencyInfo,
        PluginScriptInfo, PluginStatsResponse, PluginSummary, PluginVersion,
        PluginVersionInfo, RatingResponse, UploadResponse,
    },
    services::StorageService,
    utils::config::Config,
};

pub struct PluginService {
    db_pool: PgPool,
    storage_service: Arc<StorageService>,
    config: Arc<Config>,
}

impl PluginService {
    pub fn new(
        db_pool: PgPool,
        storage_service: Arc<StorageService>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            db_pool,
            storage_service,
            config,
        }
    }

    pub async fn search_plugins(
        &self,
        query: Option<&str>,
        tag: Option<&str>,
        sort: &str,
        order: &str,
        limit: i32,
        offset: i32,
    ) -> sqlx::Result<Vec<PluginSummary>> {
        // Simplified query for now - just get basic plugin info
        let order_by = match sort {
            "rating" => "rating",
            "name" => "name", 
            "created_at" => "created_at",
            "updated_at" => "updated_at",
            _ => "downloads"
        };
        
        let order_dir = if order == "asc" { "ASC" } else { "DESC" };
        
        let mut where_clause = "WHERE status = 'active'".to_string();
        
        if let Some(search_query) = query {
            if !search_query.trim().is_empty() {
                where_clause.push_str(&format!(
                    " AND (name ILIKE '%{}%' OR description ILIKE '%{}%' OR author ILIKE '%{}%')",
                    search_query.replace("'", "''"), // Basic SQL injection prevention
                    search_query.replace("'", "''"),
                    search_query.replace("'", "''")
                ));
            }
        }
        
        if let Some(tag_filter) = tag {
            if !tag_filter.trim().is_empty() {
                where_clause.push_str(&format!(
                    " AND id IN (SELECT plugin_id FROM plugin_tags WHERE tag ILIKE '%{}%')",
                    tag_filter.replace("'", "''")
                ));
            }
        }
        
        let sql = format!(
            "SELECT id, name, description, author, current_version, downloads, rating, created_at, updated_at 
             FROM plugins 
             {} 
             ORDER BY {} {} 
             LIMIT {} OFFSET {}",
            where_clause, order_by, order_dir, limit, offset
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&self.db_pool)
            .await?;

        let mut plugins = Vec::new();
        for row in rows {
            // Convert NUMERIC to f64
            let rating: Option<sqlx::types::Decimal> = row.try_get("rating").ok();
            let rating_f64 = rating.map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);
            
            plugins.push(PluginSummary {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                author: row.get("author"),
                current_version: row.get("current_version"),
                downloads: row.get("downloads"),
                rating: rating_f64,
                tags: vec![], // Empty for now
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(plugins)
    }

    pub async fn count_plugins(
        &self,
        query: Option<&str>,
        tag: Option<&str>,
    ) -> sqlx::Result<i64> {
        let mut sql = String::from(
            "SELECT COUNT(DISTINCT p.id) FROM plugins p LEFT JOIN plugin_tags pt ON p.id = pt.plugin_id WHERE p.status = 'active'"
        );

        let mut conditions = Vec::new();
        let mut bind_index = 1;
        let mut bind_params: Vec<String> = Vec::new();

        if let Some(q) = query {
            conditions.push(format!(
                "(p.name ILIKE ${} OR p.description ILIKE ${})",
                bind_index, bind_index + 1
            ));
            let search_pattern = format!("%{}%", q);
            bind_params.push(search_pattern.clone());
            bind_params.push(search_pattern);
            bind_index += 2;
        }

        if let Some(t) = tag {
            conditions.push(format!("pt.tag = ${}", bind_index));
            bind_params.push(t.to_string());
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        let mut query_builder = sqlx::query_scalar(&sql);

        for param in bind_params {
            query_builder = query_builder.bind(param);
        }

        query_builder.fetch_one(&self.db_pool).await
    }

    pub async fn get_plugin_detail(&self, plugin_id: &str) -> sqlx::Result<Option<PluginDetailResponse>> {
        let row = sqlx::query(
            "SELECT * FROM plugins WHERE id = $1 AND status = 'active'"
        )
        .bind(plugin_id)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(row) = row {
            // Convert NUMERIC to f64
            let rating: Option<sqlx::types::Decimal> = row.try_get("rating").ok();
            let rating_f64 = rating.map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);
            
            let current_version: String = row.get("current_version");
            let plugin_id: String = row.get("id");
            
            let versions = self.get_plugin_versions(&plugin_id).await?;
            let scripts = self.get_plugin_scripts(&plugin_id, &current_version).await?;
            let dependencies = self.get_plugin_dependencies(&plugin_id).await?;
            let tags = self.get_plugin_tags(&plugin_id).await?;

            Ok(Some(PluginDetailResponse {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                author: row.get("author"),
                current_version,
                downloads: row.get("downloads"),
                rating: rating_f64,
                tags,
                min_geektools_version: row.get("min_geektools_version"),
                homepage_url: row.get("homepage_url"),
                repository_url: row.get("repository_url"),
                license: row.get("license"),
                versions,
                scripts,
                dependencies,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn upload_plugin(
        &self,
        data: Vec<u8>,
        _user_id: i32,
        upload_id: &str,
    ) -> sqlx::Result<UploadResponse> {
        // Store file temporarily
        let temp_file = self.storage_service.store_temporary_file(data.clone()).await
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to store temp file: {}", e)))?;

        // Extract and validate plugin
        let plugin_info = match self.extract_and_validate_plugin(&temp_file).await {
            Ok(info) => info,
            Err(e) => {
                // If extraction fails, create a basic plugin info from filename
                tracing::warn!("Plugin extraction failed: {}, creating basic info", e);
                let plugin_id = format!("plugin_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string());
                CreatePluginRequest {
                    id: plugin_id.clone(),
                    name: format!("未知插件 {}", &plugin_id[7..]), // Use part after "plugin_" prefix
                    description: Some("上传的插件包，无法解析插件信息".to_string()),
                    author: "未知作者".to_string(),
                    version: "1.0.0".to_string(),
                    min_geektools_version: None,
                    homepage_url: None,
                    repository_url: None,
                    license: None,
                    tags: vec!["uploaded".to_string()],
                    scripts: vec![], // Empty for now
                    dependencies: vec![],
                }
            }
        };

        // Check if plugin already exists
        let existing_plugin = sqlx::query_as::<_, Plugin>(
            "SELECT * FROM plugins WHERE id = $1"
        )
        .bind(&plugin_info.id)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(_) = existing_plugin {
            // Check if version already exists
            let existing_version = sqlx::query_as::<_, PluginVersion>(
                "SELECT * FROM plugin_versions WHERE plugin_id = $1 AND version = $2"
            )
            .bind(&plugin_info.id)
            .bind(&plugin_info.version)
            .fetch_optional(&self.db_pool)
            .await?;

            if existing_version.is_some() {
                return Err(sqlx::Error::Protocol(format!(
                    "Version {} already exists for plugin {}",
                    plugin_info.version, plugin_info.id
                )));
            }
        }

        // Calculate file hash
        let file_hash = self.calculate_file_hash(&data);
        let file_size = data.len();

        // Store plugin file permanently
        let file_path = self.storage_service
            .store_plugin_file(data, &plugin_info.id, &plugin_info.version)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to store plugin file: {}", e)))?;

        // Save to database
        let mut tx = self.db_pool.begin().await?;

        // Create or update plugin
        if existing_plugin.is_none() {
            sqlx::query(
                r#"
                INSERT INTO plugins (id, name, description, author, current_version, 
                                   min_geektools_version, homepage_url, repository_url, license)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#
            )
            .bind(&plugin_info.id)
            .bind(&plugin_info.name)
            .bind(&plugin_info.description)
            .bind(&plugin_info.author)
            .bind(&plugin_info.version)
            .bind(&plugin_info.min_geektools_version)
            .bind(&plugin_info.homepage_url)
            .bind(&plugin_info.repository_url)
            .bind(&plugin_info.license)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE plugins SET current_version = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
            )
            .bind(&plugin_info.version)
            .bind(&plugin_info.id)
            .execute(&mut *tx)
            .await?;
        }

        // Create version record
        sqlx::query(
            r#"
            INSERT INTO plugin_versions (plugin_id, version, changelog, file_path, file_size, file_hash)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(&plugin_info.id)
        .bind(&plugin_info.version)
        .bind("")
        .bind(&file_path)
        .bind(file_size as i64)
        .bind(&file_hash)
        .execute(&mut *tx)
        .await?;

        // Save scripts
        for script in &plugin_info.scripts {
            sqlx::query(
                r#"
                INSERT INTO plugin_scripts (plugin_id, version, script_name, script_file, description, is_executable)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(&plugin_info.id)
            .bind(&plugin_info.version)
            .bind(&script.name)
            .bind(&script.file)
            .bind(&script.description)
            .bind(script.executable)
            .execute(&mut *tx)
            .await?;
        }

        // Delete old tags and insert new ones
        sqlx::query("DELETE FROM plugin_tags WHERE plugin_id = $1")
            .bind(&plugin_info.id)
            .execute(&mut *tx)
            .await?;

        for tag in &plugin_info.tags {
            sqlx::query(
                "INSERT INTO plugin_tags (plugin_id, tag) VALUES ($1, $2)"
            )
            .bind(&plugin_info.id)
            .bind(tag)
            .execute(&mut *tx)
            .await?;
        }

        // Save dependencies
        for dep in &plugin_info.dependencies {
            sqlx::query(
                "INSERT INTO plugin_dependencies (plugin_id, dependency_id, min_version) VALUES ($1, $2, $3)"
            )
            .bind(&plugin_info.id)
            .bind(&dep.id)
            .bind(&dep.min_version)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Clean up temporary file
        let _ = self.storage_service.cleanup_temporary_file(&temp_file).await;

        Ok(UploadResponse {
            plugin_id: plugin_info.id,
            version: plugin_info.version,
            upload_id: upload_id.to_string(),
        })
    }

    async fn extract_and_validate_plugin(&self, file_path: &std::path::Path) -> anyhow::Result<CreatePluginRequest> {
        let file = std::fs::File::open(file_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        let mut info_json = None;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?;
            
            if path.file_name() == Some(std::ffi::OsStr::new("info.json")) {
                let mut contents = String::new();
                entry.read_to_string(&mut contents)?;
                info_json = Some(contents);
                break;
            }
        }

        let info_content = info_json.ok_or_else(|| {
            anyhow::anyhow!("Plugin package missing info.json file")
        })?;

        let plugin_info: CreatePluginRequest = serde_json::from_str(&info_content)?;
        plugin_info.validate()?;

        Ok(plugin_info)
    }

    fn calculate_file_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    async fn get_plugin_versions(&self, plugin_id: &str) -> sqlx::Result<Vec<PluginVersionInfo>> {
        let rows = sqlx::query(
            "SELECT version, changelog, file_size, created_at, downloads, is_stable FROM plugin_versions WHERE plugin_id = $1 ORDER BY created_at DESC"
        )
        .bind(plugin_id)
        .fetch_all(&self.db_pool)
        .await?;

        let mut versions = Vec::new();
        for row in rows {
            versions.push(PluginVersionInfo {
                version: row.get("version"),
                changelog: row.get("changelog"),
                file_size: row.get("file_size"),
                created_at: row.get("created_at"),
                downloads: row.get("downloads"),
                is_stable: row.get("is_stable"),
            });
        }

        Ok(versions)
    }

    async fn get_plugin_scripts(&self, plugin_id: &str, version: &str) -> sqlx::Result<Vec<PluginScriptInfo>> {
        let rows = sqlx::query(
            "SELECT script_name, script_file, description, is_executable FROM plugin_scripts WHERE plugin_id = $1 AND version = $2"
        )
        .bind(plugin_id)
        .bind(version)
        .fetch_all(&self.db_pool)
        .await?;

        let mut scripts = Vec::new();
        for row in rows {
            scripts.push(PluginScriptInfo {
                name: row.get("script_name"),
                file: row.get("script_file"),
                description: row.get("description"),
                executable: row.get("is_executable"),
            });
        }

        Ok(scripts)
    }

    async fn get_plugin_dependencies(&self, plugin_id: &str) -> sqlx::Result<Vec<PluginDependencyInfo>> {
        let rows = sqlx::query(
            "SELECT dependency_id, min_version FROM plugin_dependencies WHERE plugin_id = $1"
        )
        .bind(plugin_id)
        .fetch_all(&self.db_pool)
        .await?;

        let mut dependencies = Vec::new();
        for row in rows {
            dependencies.push(PluginDependencyInfo {
                id: row.get("dependency_id"),
                min_version: row.get("min_version"),
            });
        }

        Ok(dependencies)
    }

    async fn get_plugin_tags(&self, plugin_id: &str) -> sqlx::Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT tag FROM plugin_tags WHERE plugin_id = $1"
        )
        .bind(plugin_id)
        .fetch_all(&self.db_pool)
        .await?;

        Ok(rows)
    }

    pub async fn get_download_info(&self, plugin_id: &str, version: Option<&str>) -> sqlx::Result<Option<(String, String)>> {
        let query = if let Some(v) = version {
            sqlx::query(
                "SELECT file_path, version FROM plugin_versions WHERE plugin_id = $1 AND version = $2"
            )
            .bind(plugin_id)
            .bind(v)
        } else {
            sqlx::query(
                r#"
                SELECT pv.file_path, pv.version 
                FROM plugin_versions pv 
                JOIN plugins p ON pv.plugin_id = p.id 
                WHERE p.id = $1 AND pv.version = p.current_version
                "#
            )
            .bind(plugin_id)
        };

        if let Some(row) = query.fetch_optional(&self.db_pool).await? {
            let file_path: String = row.get("file_path");
            let version: String = row.get("version");
            let filename = format!("{}-{}.tar.gz", plugin_id, version);
            Ok(Some((file_path, filename)))
        } else {
            Ok(None)
        }
    }

    pub async fn increment_download_count(&self, plugin_id: &str, version: Option<&str>) -> sqlx::Result<()> {
        let mut tx = self.db_pool.begin().await?;

        if let Some(v) = version {
            sqlx::query(
                "UPDATE plugin_versions SET downloads = downloads + 1 WHERE plugin_id = $1 AND version = $2"
            )
            .bind(plugin_id)
            .bind(v)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "UPDATE plugins SET downloads = downloads + 1 WHERE id = $1"
        )
        .bind(plugin_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn get_plugin_stats(&self, plugin_id: &str) -> sqlx::Result<Option<PluginStatsResponse>> {
        let plugin = sqlx::query_as::<_, Plugin>(
            "SELECT * FROM plugins WHERE id = $1"
        )
        .bind(plugin_id)
        .fetch_optional(&self.db_pool)
        .await?;

        if plugin.is_some() {
            // For now, return mock stats
            Ok(Some(PluginStatsResponse {
                total_downloads: 1250,
                weekly_downloads: 85,
                monthly_downloads: 340,
                version_distribution: HashMap::new(),
                download_trend: Vec::new(),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_plugin_ratings(&self, _plugin_id: &str, _page: i32, _limit: i32) -> sqlx::Result<Vec<RatingResponse>> {
        // Implementation for ratings would go here
        Ok(Vec::new())
    }

    pub async fn create_or_update_rating(
        &self,
        plugin_id: &str,
        user_id: i32,
        rating: i32,
        review: Option<String>,
    ) -> sqlx::Result<RatingResponse> {
        // Check if a rating from this user already exists
        let existing_rating = sqlx::query_scalar::<_, i32>(
            "SELECT id FROM plugin_ratings WHERE plugin_id = $1 AND user_id = $2"
        )
        .bind(plugin_id)
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await?;

        let rating_id = if let Some(existing_id) = existing_rating {
            // Update existing rating
            sqlx::query_scalar::<_, i32>(
                "UPDATE plugin_ratings SET rating = $1, review = $2, updated_at = NOW() 
                 WHERE id = $3 RETURNING id"
            )
            .bind(rating)
            .bind(&review)
            .bind(existing_id)
            .fetch_one(&self.db_pool)
            .await?
        } else {
            // Create new rating
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO plugin_ratings (plugin_id, user_id, rating, review, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, NOW(), NOW()) RETURNING id"
            )
            .bind(plugin_id)
            .bind(user_id)
            .bind(rating)
            .bind(&review)
            .fetch_one(&self.db_pool)
            .await?
        };

        // Update plugin's average rating
        let avg_rating: Option<sqlx::types::Decimal> = sqlx::query_scalar(
            "SELECT AVG(rating) FROM plugin_ratings WHERE plugin_id = $1"
        )
        .bind(plugin_id)
        .fetch_one(&self.db_pool)
        .await?;

        let rating_f64 = avg_rating.map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);

        sqlx::query(
            "UPDATE plugins SET rating = $1, updated_at = NOW() WHERE id = $2"
        )
        .bind(rating_f64)
        .bind(plugin_id)
        .execute(&self.db_pool)
        .await?;

        // Return the rating response
        Ok(RatingResponse {
            id: rating_id,
            plugin_id: plugin_id.to_string(),
            user_id,
            username: "用户".to_string(), // We would normally fetch this from users table
            rating,
            review,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    pub async fn get_search_suggestions(&self, query: &str) -> sqlx::Result<Vec<String>> {
        let suggestions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT name FROM plugins 
            WHERE name ILIKE $1 AND status = 'active' 
            ORDER BY downloads DESC 
            LIMIT 10
            "#
        )
        .bind(format!("{}%", query))
        .fetch_all(&self.db_pool)
        .await?;

        Ok(suggestions)
    }
}