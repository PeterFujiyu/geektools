use crate::{fileio, log_only, LOG_FILE};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::Duration,
};

/// 插件市场插件信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketplacePlugin {
    pub id: String,  // 实际API返回的是字符串ID
    pub name: String,
    #[serde(rename = "current_version")]
    pub version: String,  // API字段是current_version
    pub description: String,
    pub author: String,
    #[serde(rename = "downloads")]
    pub download_count: i32,  // API字段是downloads
    pub rating: f32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_file_url")]
    pub file_url: String,  // 可能不存在，提供默认值
    #[serde(default)]
    pub file_size: i64,    // 可能不存在，默认为0
    pub tags: Vec<String>,
}

fn default_file_url() -> String {
    "".to_string()
}

/// 实际API的分页信息结构
#[derive(Debug, Deserialize)]
pub struct PaginationInfo {
    pub limit: i32,
    pub page: i32,
    pub pages: i32,
    pub total: i32,
}

/// 实际API的数据结构
#[derive(Debug, Deserialize)]
pub struct ApiData {
    pub pagination: PaginationInfo,
    pub plugins: Vec<MarketplacePlugin>,
}

/// 实际API的响应结构
#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub data: ApiData,
    pub success: bool,
}

/// 标准化的插件列表响应结构
#[derive(Debug, Deserialize)]
pub struct PluginListResponse {
    pub plugins: Vec<MarketplacePlugin>,
    pub total: i32,
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub per_page: i32,
    #[serde(default)]
    pub total_pages: i32,
}

/// 搜索响应结构
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub plugins: Vec<MarketplacePlugin>,
    pub total: i32,
    pub query: String,
}

/// 排序方式枚举
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Name,
    Rating,
    Downloads,
    CreatedAt,
    UpdatedAt,
}

impl SortBy {
    pub fn to_string(self) -> &'static str {
        match self {
            SortBy::Name => "name",  
            SortBy::Rating => "rating",
            SortBy::Downloads => "download_count",
            SortBy::CreatedAt => "created_at",
            SortBy::UpdatedAt => "updated_at",
        }
    }

    pub fn from_choice(choice: usize) -> Option<Self> {
        match choice {
            1 => Some(SortBy::Name),
            2 => Some(SortBy::Rating),
            3 => Some(SortBy::Downloads),
            4 => Some(SortBy::CreatedAt),
            5 => Some(SortBy::UpdatedAt),
            _ => None,
        }
    }
}

/// 插件市场配置  
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketplaceConfig {
    pub api_url: String,
    pub api_port: u16,
    #[serde(default)]
    pub timeout_seconds: u64,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            api_url: "https://market-api.yshsr.org".to_string(),
            api_port: 443,
            timeout_seconds: 30,
        }
    }
}

/// 插件市场客户端
pub struct MarketplaceClient {
    config: MarketplaceConfig,
    client: Client,
}

impl MarketplaceClient {
    /// 创建新的市场客户端
    pub fn new(config: MarketplaceConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        Ok(Self { config, client })
    }

    /// 构建API完整URL
    fn build_api_url(&self, endpoint: &str) -> String {
        format!("{}:{}/api/v1{}", self.config.api_url, self.config.api_port, endpoint)
    }

    /// 获取插件列表（分页）
    pub fn get_plugins(&self, page: i32, per_page: i32, sort_by: Option<SortBy>) -> Result<PluginListResponse, String> {
        let mut url = format!("{}/plugins?page={}&per_page={}", 
            self.build_api_url(""), page, per_page);
        
        if let Some(sort) = sort_by {
            url = format!("{}&sort_by={}", url, sort.to_string());
        }

        // 记录API请求信息
        log_only!("INFO", "API_REQUEST", "插件市场浏览 URL={}", url);

        let response = self.client
            .get(&url)
            .send()
            .map_err(|e| {
                log_only!("ERROR", "API_REQUEST", "插件市场请求失败: {}", e);
                format!("请求失败: {}", e)
            })?;

        // 记录响应状态
        log_only!("INFO", "API_RESPONSE", "插件市场响应 status={}", response.status());

        if !response.status().is_success() {
            return Err(format!("API请求失败，状态码: {}", response.status()));
        }

        // 先获取响应文本用于调试
        let response_text = response.text()
            .map_err(|e| {
                log_only!("ERROR", "API_RESPONSE", "读取响应文本失败: {}", e);
                format!("读取响应文本失败: {}", e)
            })?;
        
        // 记录响应内容（截取前200字符以避免日志过长）
        let preview = if response_text.len() > 200 {
            format!("{}...", &response_text[..200])
        } else {
            response_text.clone()
        };
        log_only!("INFO", "API_RESPONSE", "插件市场响应内容: {}", preview);

        // 尝试解析为实际的API响应格式
        let plugin_response: PluginListResponse = match serde_json::from_str::<ApiResponse>(&response_text) {
            Ok(api_response) => {
                log_only!("INFO", "API_PARSE", "成功解析插件市场API响应，共 {} 个插件", api_response.data.plugins.len());
                PluginListResponse {
                    plugins: api_response.data.plugins,
                    total: api_response.data.pagination.total,
                    page: api_response.data.pagination.page,
                    per_page: api_response.data.pagination.limit,
                    total_pages: api_response.data.pagination.pages,
                }
            }
            Err(e1) => {
                log_only!("WARN", "API_PARSE", "API格式解析失败，尝试其他格式: {}", e1);
                // 尝试解析为简单格式
                match serde_json::from_str::<PluginListResponse>(&response_text) {
                    Ok(response) => {
                        log_only!("INFO", "API_PARSE", "成功解析为简单格式");
                        response
                    }
                    Err(e2) => {
                        log_only!("WARN", "API_PARSE", "简单格式解析失败，尝试插件数组: {}", e2);
                        // 最后尝试解析为插件数组
                        match serde_json::from_str::<Vec<MarketplacePlugin>>(&response_text) {
                            Ok(plugins) => {
                                log_only!("INFO", "API_PARSE", "成功解析为插件数组，共 {} 个插件", plugins.len());
                                let total = plugins.len() as i32;
                                PluginListResponse {
                                    plugins,
                                    total,
                                    page: 1,
                                    per_page: total,
                                    total_pages: 1,
                                }
                            }
                            Err(e3) => {
                                return Err(format!("所有解析方式都失败:\n1. API格式: {}\n2. 简单格式: {}\n3. 插件数组: {}\n响应内容: {}", e1, e2, e3, response_text));
                            }
                        }
                    }
                }
            }
        };

        Ok(plugin_response)
    }

    /// 搜索插件 (使用插件列表端点进行搜索)
    pub fn search_plugins(&self, query: &str) -> Result<SearchResponse, String> {
        // 使用插件列表API进行搜索
        let url = format!("{}/plugins?search={}", 
            self.build_api_url(""), urlencoding::encode(query));

        // 记录搜索请求信息
        log_only!("INFO", "API_REQUEST", "插件搜索 query='{}' URL={}", query, url);

        let response = self.client
            .get(&url)
            .send()
            .map_err(|e| {
                log_only!("ERROR", "API_REQUEST", "插件搜索请求失败: {}", e);
                format!("搜索请求失败: {}", e)
            })?;

        // 记录搜索响应状态
        log_only!("INFO", "API_RESPONSE", "插件搜索响应 status={}", response.status());

        if !response.status().is_success() {
            return Err(format!("搜索失败，状态码: {}", response.status()));
        }

        // 先获取响应文本用于调试
        let response_text = response.text()
            .map_err(|e| format!("读取搜索响应文本失败: {}", e))?;
        
        // 记录搜索响应内容（截取前200字符以避免日志过长）
        let preview = if response_text.len() > 200 {
            format!("{}...", &response_text[..200])
        } else {
            response_text.clone()
        };
        log_only!("INFO", "API_RESPONSE", "插件搜索响应内容: {}", preview);

        // 尝试解析为实际的API响应格式
        let plugin_response: PluginListResponse = match serde_json::from_str::<ApiResponse>(&response_text) {
            Ok(api_response) => {
                log_only!("INFO", "API_PARSE", "成功解析搜索API响应，共 {} 个插件", api_response.data.plugins.len());
                PluginListResponse {
                    plugins: api_response.data.plugins,
                    total: api_response.data.pagination.total,
                    page: api_response.data.pagination.page,
                    per_page: api_response.data.pagination.limit,
                    total_pages: api_response.data.pagination.pages,
                }
            }
            Err(e1) => {
                log_only!("WARN", "API_PARSE", "搜索API格式解析失败，尝试其他格式: {}", e1);
                // 尝试解析为简单格式
                match serde_json::from_str::<PluginListResponse>(&response_text) {
                    Ok(response) => {
                        log_only!("INFO", "API_PARSE", "成功解析搜索为简单格式");
                        response
                    }
                    Err(e2) => {
                        log_only!("WARN", "API_PARSE", "搜索简单格式解析失败，尝试插件数组: {}", e2);
                        // 最后尝试解析为插件数组
                        match serde_json::from_str::<Vec<MarketplacePlugin>>(&response_text) {
                            Ok(plugins) => {
                                log_only!("INFO", "API_PARSE", "成功解析搜索结果为插件数组，共 {} 个插件", plugins.len());
                                let total = plugins.len() as i32;
                                PluginListResponse {
                                    plugins,
                                    total,
                                    page: 1,
                                    per_page: total,
                                    total_pages: 1,
                                }
                            }
                            Err(e3) => {
                                log_only!("ERROR", "API_PARSE", "所有搜索解析方式都失败: API格式={}, 简单格式={}, 插件数组={}", e1, e2, e3);
                                return Err(format!("搜索解析失败:\n1. API格式: {}\n2. 简单格式: {}\n3. 插件数组: {}\n响应内容: {}", e1, e2, e3, response_text));
                            }
                        }
                    }
                }
            }
        };

        // 转换为SearchResponse格式
        let search_response = SearchResponse {
            plugins: plugin_response.plugins,
            total: plugin_response.total,
            query: query.to_string(),
        };

        Ok(search_response)
    }

    /// 下载插件
    pub fn download_plugin(&self, download_url: &str, save_path: &Path) -> Result<(), String> {
        log_only!("INFO", "DOWNLOAD", "插件下载 URL={}", download_url);
        log_only!("INFO", "DOWNLOAD", "插件保存路径={:?}", save_path);
        
        let response = self.client
            .get(download_url)
            .send()
            .map_err(|e| {
                log_only!("ERROR", "DOWNLOAD", "插件下载请求失败: {}", e);
                format!("下载请求失败: {}", e)
            })?;

        log_only!("INFO", "DOWNLOAD", "插件下载响应 status={}", response.status());

        if !response.status().is_success() {
            log_only!("ERROR", "DOWNLOAD", "插件下载失败，状态码: {}", response.status());
            return Err(format!("下载失败，状态码: {}", response.status()));
        }

        let bytes = response.bytes()
            .map_err(|e| {
                log_only!("ERROR", "DOWNLOAD", "读取下载内容失败: {}", e);
                format!("读取下载内容失败: {}", e)
            })?;

        log_only!("INFO", "DOWNLOAD", "插件下载文件大小: {} bytes", bytes.len());

        fileio::write_bytes(save_path, &bytes)
            .map_err(|e| {
                log_only!("ERROR", "DOWNLOAD", "保存插件文件失败: {}", e);
                format!("保存插件文件失败: {}", e)
            })?;

        log_only!("INFO", "DOWNLOAD", "插件文件保存成功");
        Ok(())
    }

    /// 测试API连接
    pub fn test_connection(&self) -> Result<(), String> {
        let url = format!("{}/health", self.build_api_url(""));
        
        log_only!("INFO", "API_TEST", "测试API连接 URL={}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .map_err(|e| {
                log_only!("ERROR", "API_TEST", "连接测试失败: {}", e);
                format!("连接测试失败: {}", e)
            })?;

        log_only!("INFO", "API_TEST", "连接测试响应 status={}", response.status());

        if response.status().is_success() {
            log_only!("INFO", "API_TEST", "API连接测试成功");
            Ok(())
        } else {
            log_only!("ERROR", "API_TEST", "API服务器响应错误，状态码: {}", response.status());
            Err(format!("API服务器响应错误，状态码: {}", response.status()))
        }
    }
}

/// 本地插件扫描器
pub struct LocalPluginScanner {
    scan_directories: Vec<String>,
}

impl LocalPluginScanner {
    /// 创建新的本地扫描器
    pub fn new() -> Self {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let scan_directories = vec![
            format!("{}/Downloads", home_dir),
            format!("{}/Desktop", home_dir),
            format!("{}/Documents", home_dir),
            ".".to_string(), // 当前目录
        ];

        Self { scan_directories }
    }

    /// 添加扫描目录
    pub fn add_scan_directory(&mut self, directory: String) {
        if !self.scan_directories.contains(&directory) {
            self.scan_directories.push(directory);
        }
    }

    /// 扫描本地插件文件
    pub fn scan_plugins(&self) -> Vec<LocalPluginInfo> {
        let mut plugins = Vec::new();

        for dir in &self.scan_directories {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension == "tar" || extension == "gz" || 
                           (extension == "gz" && path.to_string_lossy().ends_with(".tar.gz")) {
                            
                            if let Some(plugin_info) = self.analyze_plugin_file(&path) {
                                plugins.push(plugin_info);
                            }
                        }
                    }
                }
            }
        }

        plugins
    }

    /// 分析本地插件文件信息
    fn analyze_plugin_file(&self, path: &Path) -> Option<LocalPluginInfo> {
        if let Ok(metadata) = std::fs::metadata(path) {
            let file_name = path.file_name()?.to_string_lossy().to_string();
            let file_size = metadata.len();
            let modified_time = metadata.modified().ok()?;
            
            // 从文件名推断插件信息
            let (name, version) = self.parse_filename(&file_name);
            
            Some(LocalPluginInfo {
                file_path: path.to_path_buf(),
                file_name,
                file_size,
                modified_time: format!("{:?}", modified_time),
                estimated_name: name,
                estimated_version: version,
            })
        } else {
            None
        }
    }

    /// 从文件名解析插件名称和版本
    fn parse_filename(&self, filename: &str) -> (String, String) {
        let base_name = filename
            .strip_suffix(".tar.gz")
            .or_else(|| filename.strip_suffix(".tar"))
            .unwrap_or(filename);

        // 尝试解析版本号 (例如: plugin-name-v1.2.3 或 plugin-name-1.2.3)
        if let Some(last_dash) = base_name.rfind('-') {
            let potential_version = &base_name[last_dash + 1..];
            if potential_version.starts_with('v') || 
               potential_version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                let name = base_name[..last_dash].to_string();
                let version = potential_version.to_string();
                return (name, version);
            }
        }

        (base_name.to_string(), "unknown".to_string())
    }
}

impl Default for LocalPluginScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// 本地插件信息
#[derive(Debug, Clone)]
pub struct LocalPluginInfo {
    pub file_path: std::path::PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub modified_time: String,
    pub estimated_name: String,
    pub estimated_version: String,
}