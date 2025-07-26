use crate::fileio;
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    path::{Path, PathBuf},
};
use tar::Archive;

/// 插件目录：~/.geektools/plugins/
static PLUGINS_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join(".geektools").join("plugins");
    // Create directory if it doesn't exist
    let _ = fileio::create_dir(&dir);
    dir
});

/// 插件元数据文件结构 (info.json)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub scripts: Vec<ScriptEntry>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub min_geektools_version: Option<String>,
}

/// 脚本条目信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptEntry {
    pub name: String,
    pub file: String,
    pub description: String,
    #[serde(default)]
    pub executable: bool,
}

/// 已安装插件的记录
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledPlugin {
    pub info: PluginInfo,
    pub install_path: PathBuf,
    pub installed_at: String,
    #[serde(default)]
    pub enabled: bool,
}

/// 插件管理器
pub struct PluginManager {
    installed_plugins: HashMap<String, InstalledPlugin>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// 创建新的插件管理器实例
    pub fn new() -> Self {
        let mut manager = Self {
            installed_plugins: HashMap::new(),
        };
        
        // 加载已安装的插件
        if let Err(e) = manager.load_installed_plugins() {
            eprintln!("Warning: Failed to load installed plugins: {}", e);
        }
        
        manager
    }

    /// 从 .tar.gz 文件安装插件
    pub fn install_plugin(&mut self, plugin_path: &Path) -> Result<String, String> {
        // 1. 验证文件存在
        if !plugin_path.exists() {
            return Err(format!("Plugin file does not exist: {:?}", plugin_path));
        }

        // 2. 解压并验证插件包
        let temp_dir = self.extract_plugin_package(plugin_path)?;
        let plugin_info = self.validate_plugin_package(&temp_dir)?;

        // 3. 检查是否已安装
        if self.installed_plugins.contains_key(&plugin_info.id) {
            return Err(format!("Plugin '{}' is already installed", plugin_info.id));
        }

        // 4. 检查依赖
        self.check_dependencies(&plugin_info)?;

        // 5. 安装插件到目标目录
        let install_path = PLUGINS_DIR.join(&plugin_info.id);
        if install_path.exists() {
            fileio::remove_dir(&install_path)
                .map_err(|e| format!("Failed to remove existing plugin directory: {}", e))?;
        }

        // 复制插件文件到安装目录
        self.copy_plugin_files(&temp_dir, &install_path)?;

        // 6. 设置脚本可执行权限
        self.set_script_permissions(&install_path, &plugin_info)?;

        // 7. 记录已安装插件
        let installed_plugin = InstalledPlugin {
            info: plugin_info.clone(),
            install_path: install_path.clone(),
            installed_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            enabled: true,
        };

        self.installed_plugins.insert(plugin_info.id.clone(), installed_plugin);
        self.save_installed_plugins()?;

        // 8. 清理临时目录
        let _ = fileio::remove_dir(&temp_dir);

        Ok(plugin_info.id)
    }

    /// 卸载插件
    pub fn uninstall_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        let plugin = self.installed_plugins.get(plugin_id)
            .ok_or_else(|| format!("Plugin '{}' is not installed", plugin_id))?;

        let install_path = plugin.install_path.clone();
        
        // 删除插件目录
        if install_path.exists() {
            fileio::remove_dir(&install_path)
                .map_err(|e| format!("Failed to remove plugin directory: {}", e))?;
        }

        // 从记录中移除
        self.installed_plugins.remove(plugin_id);
        self.save_installed_plugins()?;

        Ok(())
    }

    /// 获取已安装插件列表
    pub fn list_installed_plugins(&self) -> Vec<&InstalledPlugin> {
        self.installed_plugins.values().collect()
    }


    /// 启用/禁用插件
    pub fn toggle_plugin(&mut self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        let plugin = self.installed_plugins.get_mut(plugin_id)
            .ok_or_else(|| format!("Plugin '{}' is not installed", plugin_id))?;

        plugin.enabled = enabled;
        self.save_installed_plugins()?;
        Ok(())
    }

    /// 获取所有已启用插件的脚本
    pub fn get_enabled_scripts(&self) -> Vec<(String, String, PathBuf)> {
        let mut scripts = Vec::new();
        
        for plugin in self.installed_plugins.values() {
            if plugin.enabled {
                for script in &plugin.info.scripts {
                    let script_path = plugin.install_path.join("scripts").join(&script.file);
                    if script_path.exists() {
                        scripts.push((
                            format!("{} - {}", script.name, plugin.info.name),
                            script.description.clone(),
                            script_path,
                        ));
                    }
                }
            }
        }
        
        scripts
    }

    /// 解压插件包到临时目录
    fn extract_plugin_package(&self, plugin_path: &Path) -> Result<PathBuf, String> {
        let temp_dir = env::temp_dir().join(format!("geektools_plugin_{}", rand::random::<u64>()));
        
        // 创建临时目录
        fileio::create_dir(&temp_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        // 打开并解压 .tar.gz 文件
        let file = File::open(plugin_path)
            .map_err(|e| format!("Failed to open plugin file: {}", e))?;
        
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        
        archive.unpack(&temp_dir)
            .map_err(|e| format!("Failed to extract plugin package: {}", e))?;

        Ok(temp_dir)
    }

    /// 验证插件包结构和元数据
    fn validate_plugin_package(&self, plugin_dir: &Path) -> Result<PluginInfo, String> {
        // 检查 info.json 文件
        let info_path = plugin_dir.join("info.json");
        if !info_path.exists() {
            return Err("Plugin package missing info.json file".to_string());
        }

        // 读取并解析 info.json
        let info_content = fileio::read(&info_path)
            .map_err(|e| format!("Failed to read info.json: {}", e))?;
        
        let plugin_info: PluginInfo = serde_json::from_str(&info_content)
            .map_err(|e| format!("Failed to parse info.json: {}", e))?;

        // 验证必要字段
        if plugin_info.id.is_empty() {
            return Err("Plugin ID cannot be empty".to_string());
        }
        if plugin_info.name.is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }
        if plugin_info.version.is_empty() {
            return Err("Plugin version cannot be empty".to_string());
        }

        // 检查 scripts 目录
        let scripts_dir = plugin_dir.join("scripts");
        if !scripts_dir.exists() || !scripts_dir.is_dir() {
            return Err("Plugin package missing scripts directory".to_string());
        }

        // 验证脚本文件是否存在
        for script in &plugin_info.scripts {
            let script_path = scripts_dir.join(&script.file);
            if !script_path.exists() {
                return Err(format!("Script file '{}' not found", script.file));
            }
        }

        Ok(plugin_info)
    }

    /// 检查插件依赖
    fn check_dependencies(&self, plugin_info: &PluginInfo) -> Result<(), String> {
        for dep in &plugin_info.dependencies {
            if !self.installed_plugins.contains_key(dep) {
                return Err(format!("Missing dependency: {}", dep));
            }
        }
        Ok(())
    }

    /// 复制插件文件到安装目录
    fn copy_plugin_files(&self, src_dir: &Path, dest_dir: &Path) -> Result<(), String> {
        fileio::create_dir(dest_dir)
            .map_err(|e| format!("Failed to create plugin directory: {}", e))?;

        // 复制所有文件和目录
        self.copy_directory_recursive(src_dir, dest_dir)
    }

    /// 递归复制目录
    fn copy_directory_recursive(&self, src: &Path, dest: &Path) -> Result<(), String> {
        if src.is_dir() {
            if !dest.exists() {
                fileio::create_dir(dest)
                    .map_err(|e| format!("Failed to create directory {:?}: {}", dest, e))?;
            }

            for entry in src.read_dir()
                .map_err(|e| format!("Failed to read directory {:?}: {}", src, e))? {
                let entry = entry
                    .map_err(|e| format!("Failed to read directory entry: {}", e))?;
                let src_path = entry.path();
                let dest_path = dest.join(entry.file_name());
                
                if src_path.is_dir() {
                    self.copy_directory_recursive(&src_path, &dest_path)?;
                } else {
                    let content = fileio::read(&src_path)
                        .map_err(|e| format!("Failed to read file {:?}: {}", src_path, e))?;
                    fileio::write_bytes(&dest_path, (&content).as_ref())
                        .map_err(|e| format!("Failed to write file {:?}: {}", dest_path, e))?;
                }
            }
        }
        Ok(())
    }

    /// 设置脚本文件可执行权限
    fn set_script_permissions(&self, install_path: &Path, plugin_info: &PluginInfo) -> Result<(), String> {
        #[cfg(unix)]
        {
            let scripts_dir = install_path.join("scripts");
            for script in &plugin_info.scripts {
                if script.executable {
                    let script_path = scripts_dir.join(&script.file);
                    if script_path.exists() {
                        fileio::set_executable(&script_path)
                            .map_err(|e| format!("Failed to set executable permission for '{}': {}", script.file, e))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 加载已安装插件记录
    fn load_installed_plugins(&mut self) -> Result<(), String> {
        let registry_path = PLUGINS_DIR.join("registry.json");
        
        if !registry_path.exists() {
            return Ok(()); // 没有注册表文件是正常的
        }

        let content = fileio::read(&registry_path)
            .map_err(|e| format!("Failed to read plugin registry: {}", e))?;
        
        let plugins: HashMap<String, InstalledPlugin> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse plugin registry: {}", e))?;
        
        self.installed_plugins = plugins;
        Ok(())
    }

    /// 保存已安装插件记录
    fn save_installed_plugins(&self) -> Result<(), String> {
        let registry_path = PLUGINS_DIR.join("registry.json");
        
        let content = serde_json::to_string_pretty(&self.installed_plugins)
            .map_err(|e| format!("Failed to serialize plugin registry: {}", e))?;
        
        fileio::write(&registry_path, &content)
            .map_err(|e| format!("Failed to save plugin registry: {}", e))?;
        
        Ok(())
    }
}

