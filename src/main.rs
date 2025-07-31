mod fileio;
mod i18n;
mod scripts;
mod plugins;
mod errors;
mod recovery;
mod logging;
mod config;

use plugins::{PluginManager, MarketplaceConfig};
use errors::{GeekToolsError, Result};
use recovery::{RecoveryHandler, RetryConfig, execute_with_recovery};
use logging::{LoggingConfig, init_logging};
use config::{Config, ConfigManager, CustomScript};

use chrono::Local;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{self, Value};
use std::process::exit;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{self, Write},
    path::Path,
    path::PathBuf,
    process::{self, Command},
    sync::{Arc, Mutex, RwLock},
};
// 读取build tag

// 编译期嵌入的文件内容（保持原样，含换行 / 空白）

const BUILD_TAG: &str = include_str!("./buildtag.env");

// ────────────────────────────────────────────────────────────────────────────
// 1️⃣ 统一的调试宏：只在 DEBUG 文ce开启时打印
// ────────────────────────────────────────────────────────────────────────────
static DEBUG_ENABLED: Lazy<bool> = Lazy::new(|| {
    fileio::compat::read_compat("DEBUG")
        .map(|s| s.trim() == "DEBUG=true")
        .unwrap_or(false)
});

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if *DEBUG_ENABLED {
            log_println!($($arg)*);
        }
    };
}

// ───────────────────────────────── 语言和翻译系统 ────────────────────────────────
use i18n::{Language, t};

/// 配置文件路径：~/.geektools/config.json
static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".geektools").join("config.json")
});

/// 自定义脚本存储目录：~/.geektools/custom_scripts/
static CUSTOM_SCRIPTS_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".geektools").join("custom_scripts")
});

/// 日志文件路径：~/.geektools/logs/YYYYMMDDHHMM.logs
static LOG_FILE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let ts = Local::now().format("%Y%m%d%H%M").to_string();
    PathBuf::from(home)
        .join(".geektools")
        .join("logs")
        .join(format!("{ts}.logs"))
});

pub static LOG_FILE: Lazy<Mutex<File>> = Lazy::new(|| {
    // Ensure parent directory exists
    if let Some(parent) = LOG_FILE_PATH.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    let file = File::options()
        .create(true)
        .append(true)
        .open(&*LOG_FILE_PATH)
        .unwrap_or_else(|e| {
            eprintln!("Failed to open log file: {e}");
            File::create("/dev/null").unwrap()
        });
    Mutex::new(file)
});

macro_rules! log_println {
    ($($arg:tt)*) => {{
        use std::io::Write;
        if let Ok(mut f) = LOG_FILE.lock() {
            let _ = writeln!(f, $($arg)*);
        }
        println!($($arg)*);
    }};
}

macro_rules! log_print {
    ($($arg:tt)*) => {{
        use std::io::Write;
        if let Ok(mut f) = LOG_FILE.lock() {
            let _ = write!(f, $($arg)*);
            let _ = f.flush();
        }
        print!($($arg)*);
    }};
}

macro_rules! log_eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        if let Ok(mut f) = LOG_FILE.lock() {
            let _ = writeln!(f, $($arg)*);
        }
        eprintln!($($arg)*);
    }};
}

// 仅记录到日志文件的宏（不输出到控制台）
#[macro_export]
macro_rules! log_only {
    ($level:expr, $category:expr, $($arg:tt)*) => {{
        use std::io::Write;
        if let Ok(mut f) = LOG_FILE.lock() {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(f, "{} {} {} {}", $level, timestamp, $category, format!($($arg)*));
        }
    }};
}

/// 应用程序状态
struct AppState {
    config_manager: ConfigManager,
    current_language: Language,
    recovery_handler: RecoveryHandler,
}

impl AppState {
    fn new() -> Result<Self> {
        let config_manager = ConfigManager::new(CONFIG_PATH.clone())?;
        let config = config_manager.get_config();
        let config_read = config.read().unwrap();
        
        let current_language = match config_read.language.as_str() {
            "zh" | "Chinese" => Language::Chinese,
            _ => Language::English,
        };
        
        let recovery_handler = RecoveryHandler::new(
            RetryConfig::default(),
            current_language,
        );
        
        // Initialize logging
        let _ = init_logging(&config_read.logging, Some(LOG_FILE_PATH.clone()));
        
        Ok(Self {
            config_manager,
            current_language,
            recovery_handler,
        })
    }

    // 基础翻译
    fn get_translation(&self, key_path: &str) -> String {
        t(key_path, &[], self.current_language)
    }

    // 含占位符替换
    fn get_formatted_translation(&self, key_path: &str, args: &[&str]) -> String {
        let indices: Vec<String> = (0..args.len()).map(|i| i.to_string()).collect();
        let params: Vec<(&str, &str)> = indices.iter()
            .zip(args.iter())
            .map(|(idx, &val)| (idx.as_str(), val))
            .collect();
        t(key_path, &params, self.current_language)
    }

    // 主菜单文本
    fn get_menu_text(&self) -> String {
        format!(
            "\n{}\n1. {}\n2. {}\n3. {}\n4. {}\n5. {}\n6. {}\n{}",
            self.get_translation("menu.title"),
            self.get_translation("menu.run_existing_script"),
            self.get_translation("menu.run_script_from_network"),
            self.get_translation("menu.custom_scripts"),
            self.get_translation("menu.plugin_management"),
            self.get_translation("menu.settings"),
            self.get_translation("menu.exit"),
            self.get_translation("menu.prompt_extended")
        )
    }

    // 语言切换菜单
    fn get_language_menu_text(&self) -> String {
        format!(
            "\n{}\n1. {}\n2. {}\n{}",
            self.get_translation("language_menu.title"),
            self.get_translation("language_menu.english"),
            self.get_translation("language_menu.chinese"),
            self.get_translation("language_menu.prompt")
        )
    }

    // 设置菜单
    fn get_settings_menu_text(&self) -> String {
        format!(
            "\n{}\n1. {}\n2. {}\n3. {}\n4. {}\n{}",
            self.get_translation("settings_menu.title"),
            self.get_translation("settings_menu.change_language"),
            self.get_translation("settings_menu.change_version"),
            self.get_translation("settings_menu.clear_personalization"),
            self.get_translation("settings_menu.back"),
            self.get_translation("settings_menu.prompt")
        )
    }

    // 插件管理菜单
    fn get_plugin_menu_text(&self) -> String {
        format!(
            "\n{}\n1. {}\n2. {}\n3. {}\n4. {}\n5. {}\n6. {}\n7. {}\n{}",
            self.get_translation("plugin_menu.title"),
            self.get_translation("plugin_menu.marketplace"),
            self.get_translation("plugin_menu.local_scan"),
            self.get_translation("plugin_menu.install"),
            self.get_translation("plugin_menu.list"),
            self.get_translation("plugin_menu.uninstall"),
            self.get_translation("plugin_menu.toggle"),
            self.get_translation("plugin_menu.back"),
            self.get_translation("plugin_menu.prompt_extended")
        )
    }

    // 自定义脚本管理菜单
    fn get_custom_scripts_menu_text(&self) -> String {
        format!(
            "\n{}\n1. {}\n2. {}\n3. {}\n4. {}\n{}",
            self.get_translation("custom_script_menu.title"),
            self.get_translation("custom_script_menu.add"),
            self.get_translation("custom_script_menu.list"),
            self.get_translation("custom_script_menu.remove"),
            self.get_translation("custom_script_menu.back"),
            self.get_translation("custom_script_menu.prompt")
        )
    }
}

/// 查询 IP-API 的返回结构
#[derive(Deserialize)]
struct IpApiResp {
    #[serde(rename = "countryCode")]
    country_code: String,
}

/// 加载或初始化用户语言 (legacy function for backward compatibility)
fn load_or_init_language() -> Language {
    // Try to load from the new config system first
    if let Ok(config_manager) = ConfigManager::new(CONFIG_PATH.clone()) {
        let config = config_manager.get_config();
        let config_read = config.read().unwrap();
        return match config_read.language.as_str() {
            "zh" => Language::Chinese,
            _ => Language::English,
        };
    }

    // Fallback to IP API detection
    let default_lang = match Client::new().get("http://ip-api.com/json/").send() {
        Ok(resp) => {
            if let Ok(json) = resp.json::<IpApiResp>() {
                matches!(json.country_code.as_str(), "CN" | "HK" | "MO" | "TW")
                    .then_some(Language::Chinese)
                    .unwrap_or(Language::English)
            } else {
                Language::English
            }
        }
        Err(err) => {
            log_eprintln!("IP API request failed: {err}");
            Language::English
        }
    };

    default_lang
}


#[derive(Deserialize)]
struct GhAsset {
    browser_download_url: String,
    name: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

/// 调试版：获取 GitHub Releases（正式 + 预发布），并在控制台输出全过程。
// ────────────────────────────────────────────────────────────────────────────
// 2️⃣ fetch_releases 内全部 println! → debug_log!
// ────────────────────────────────────────────────────────────────────────────
fn fetch_releases() -> std::result::Result<Vec<GhRelease>, GeekToolsError> {
    let repo = repo_path_from_cargo()?;
    let url = format!("https://api.github.com/repos/{repo}/releases");
    debug_log!("[DEBUG] 即将请求 GitHub API: {url}");

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!(
            "geektools/{} (+{})",
            env!("CARGO_PKG_VERSION"),
            "PeterFujiyu/geektools"
        ))
        .build()?;

    let resp = client
        .get(&url)
        .send()?;
    debug_log!("[DEBUG] 收到响应，状态码: {}", resp.status());

    if !resp.status().is_success() {
        return Err(GeekToolsError::ConfigError {
            message: format!("HTTP non-success status: {}", resp.status()),
        });
    }

    let text = resp.text()?;
    debug_log!("[DEBUG] 响应体长度: {}", text.len());

    let releases: Vec<GhRelease> = serde_json::from_str(&text)?;
    debug_log!("[DEBUG] 解析成功，共 {} 条", releases.len());

    Ok(releases)
}

fn asset_name() -> Option<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", _) => Some("geektools-macos-universal"),
        ("linux", "x86_64") => Some("geektools-linux-x64"),
        ("linux", "aarch64") => Some("geektools-linux-arm64"),
        _ => None,
    }
}

fn download_and_replace(url: &str) -> std::result::Result<(), GeekToolsError> {
    let resp = reqwest::blocking::get(url)?;
    let bytes = resp.bytes()?;
    let exe = env::current_exe()?;
    let mut tmp = exe.clone();
    tmp.set_extension("tmp");
    fileio::write_bytes(&tmp, &bytes)?;
    #[cfg(unix)]
    {
        let _ = fileio::set_executable(&tmp);
    }
    fileio::rename(&tmp, &exe)?;
    Ok(())
}

fn update_to_release(release: &GhRelease, app_state: &AppState) {
    let name = match asset_name() {
        Some(n) => n,
        None => {
            log_println!("{}", app_state.get_translation("update_menu.not_found"));
            return;
        }
    };
    let asset = match release.assets.iter().find(|a| a.name == name) {
        Some(a) => a,
        None => {
            log_println!("{}", app_state.get_translation("update_menu.not_found"));
            return;
        }
    };
    log_println!(
        "{}",
        app_state.get_formatted_translation("update_menu.downloading", &[&release.tag_name])
    );
    match download_and_replace(&asset.browser_download_url) {
        Ok(_) => log_println!("{}", app_state.get_translation("update_menu.success")),
        Err(e) => log_println!(
            "{}",
            app_state.get_formatted_translation("update_menu.replace_failed", &[&e.to_string()])
        ),
    }
}

fn update_to_latest(prerelease: bool, app_state: &AppState) {
    match fetch_releases() {
        Ok(releases) => {
            if let Some(rel) = releases.into_iter().find(|r| r.prerelease == prerelease) {
                update_to_release(&rel, app_state);
            } else {
                log_println!("{}", app_state.get_translation("update_menu.no_release"));
            }
        }
        Err(e) => log_println!(
            "{}",
            app_state.get_formatted_translation("update_menu.download_failed", &[&e.to_string()])
        ),
    }
}

// ---------------------------------------------------------------------------
// choose_other 加一行 DEBUG，让我们知道 fetch_releases 是否正常返回
// ---------------------------------------------------------------------------
// ────────────────────────────────────────────────────────────────────────────
// 3️⃣ choose_other 开头同样替换
// ────────────────────────────────────────────────────────────────────────────
fn choose_other(app_state: &AppState) {
    debug_log!("[DEBUG] 进入 choose_other()");
    match fetch_releases() {
        Ok(mut releases) => {
            debug_log!("[DEBUG] fetch_releases() 成功，数量: {}", releases.len());
            // 如果一个都没有就直接返回
            if releases.is_empty() {
                log_println!("{}", app_state.get_translation("update_menu.no_release"));
                return;
            }

            // 根据 tag 名倒序，让最新的排在最前面（可按需要改成基于发布时间）
            releases.sort_by(|a, b| b.tag_name.cmp(&a.tag_name));

            // 输出版本列表，预发布版额外标记一下
            for (i, r) in releases.iter().enumerate() {
                if r.prerelease {
                    log_println!("{}. {} (prerelease)", i + 1, r.tag_name);
                } else {
                    log_println!("{}. {}", i + 1, r.tag_name);
                }
            }

            let prompt = app_state.get_formatted_translation(
                "update_menu.select_prompt",
                &[&releases.len().to_string()],
            );

            loop {
                log_print!("{}", prompt);
                let _ = io::stdout().flush();

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    log_println!("{}", app_state.get_translation("main.invalid_choice"));
                    continue;
                }

                let trimmed = input.trim();
                if trimmed.eq_ignore_ascii_case("exit") {
                    return;
                }

                if let Ok(idx) = trimmed.parse::<usize>() {
                    if (1..=releases.len()).contains(&idx) {
                        let rel = &releases[idx - 1];
                        update_to_release(rel, app_state);
                        return;
                    }
                }

                log_println!("{}", app_state.get_translation("main.invalid_choice"));
            }
        }
        Err(e) => {
            log_eprintln!("[DEBUG] fetch_releases() 失败: {e}");
            log_println!(
                "{}",
                app_state.get_formatted_translation("update_menu.download_failed", &[&e.to_string()])
            );
        }
    }
}

fn change_version(app_state: &AppState) {
    loop {
        log_println!(
            "\n{}\n1. {}\n2. {}\n3. {}",
            app_state.get_translation("update_menu.title"),
            app_state.get_translation("update_menu.latest"),
            app_state.get_translation("update_menu.latest_dev"),
            app_state.get_translation("update_menu.other")
        );
        log_print!("{}", app_state.get_translation("update_menu.prompt"));
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }
        match input.trim() {
            "1" => {
                update_to_latest(false, app_state);
                break;
            }
            "2" => {
                update_to_latest(true, app_state);
                break;
            }
            "3" => {
                choose_other(app_state);
                break;
            }
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }
    }
}

// ──────────────────────────────── 运行本地脚本 ─────────────────────────────
fn run_existing_script(app_state: &AppState) {
    // 0. 清理缓存
    use std::env;

    let mut tmp_path = env::temp_dir();
    tmp_path.push("geektools");

    // 如果缓存目录存在则递归删除
    if tmp_path.exists() {
        if let Err(e) = fileio::remove_dir(&tmp_path) {
            log_eprintln!("⚠️  无法删除旧缓存目录 {:?}: {e}", tmp_path);
        }
    }

    // 重新创建空目录，忽略已存在的错误
    let _ = fileio::create_dir(&tmp_path);
    log_println!("清理成功 ✅");
    // 1. 读取 info.json（已打包进二进制）
    let data = match scripts::get_string("info.json") {
        Some(s) => s,
        None => {
            log_println!(
                "{}",
                app_state.get_translation("script_execution.no_scripts")
            );
            return;
        }
    };

    let info: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            log_println!(
                "{}",
                app_state
                    .get_formatted_translation("script_execution.invalid_json", &[&e.to_string()])
            );
            return;
        }
    };
    let map = match info.as_object() {
        Some(m) if !m.is_empty() => m,
        _ => {
            log_println!(
                "{}",
                app_state.get_translation("script_execution.no_scripts")
            );
            return;
        }
    };

    // 2. 加载自定义脚本
    let config = load_user_config();
    let custom_scripts: Vec<(usize, &CustomScript)> = config.custom_scripts.iter().enumerate().collect();

    // 2.5. 加载插件脚本
    let plugin_manager = PluginManager::new();
    let plugin_scripts = plugin_manager.get_enabled_scripts();

    // 3. 计算总脚本数量
    let total_scripts = map.len() + custom_scripts.len() + plugin_scripts.len();
    if total_scripts == 0 {
        log_println!(
            "{}",
            app_state.get_translation("script_execution.no_scripts")
        );
        return;
    }

    // 4. 展示脚本列表
    log_println!(
        "{}",
        app_state.get_translation("script_execution.available_scripts")
    );

    // 内置脚本
    let names: Vec<&String> = map.keys().collect();
    for (i, name) in names.iter().enumerate() {
        let desc = map
            .get(*name)
            .and_then(|v| {
                v.get(match app_state.current_language {
                    Language::English => "English",
                    Language::Chinese => "Chinese",
                })
            })
            .and_then(Value::as_str)
            .unwrap_or("");
        log_println!("{}. {} - {}", i + 1, name, desc);
    }

    // 自定义脚本
    for (i, (_, script)) in custom_scripts.iter().enumerate() {
        log_println!("{}. {} - {} [自定义]", names.len() + i + 1, script.name, script.description.as_deref().unwrap_or("无描述"));
    }

    // 插件脚本
    for (i, (name, description, _)) in plugin_scripts.iter().enumerate() {
        log_println!("{}. {} - {} [插件]", names.len() + custom_scripts.len() + i + 1, name, description);
    }

    // 5. 处理用户选择
    let prompt = app_state
        .get_formatted_translation("script_execution.run_prompt", &[&total_scripts.to_string()]);
    loop {
        log_print!("{}", prompt);
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }
        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") {
            log_println!(
                "{}",
                app_state.get_translation("script_execution.returning")
            );
            return;
        }
        if let Ok(idx) = input.parse::<usize>() {
            if (1..=total_scripts).contains(&idx) {
                if idx <= names.len() {
                    // 内置脚本
                    let script_name = names[idx - 1];
                    log_println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "script_execution.running_script",
                            &[script_name]
                        )
                    );

                    if script_name.ends_with(".link") {
                        // .link 文件仍使用原有逻辑
                        let script_path = match scripts::materialize(script_name) {
                            Ok(p) => p,
                            Err(e) => {
                                log_println!(
                                    "{}",
                                    app_state.get_formatted_translation(
                                        "script_execution.failed_read_info",
                                        &[&e.to_string()]
                                    )
                                );
                                return;
                            }
                        };
                        run_link_script(&script_path, app_state);
                    } else {
                        // .sh 文件使用新的依赖解析逻辑
                        match scripts::materialize_with_deps(script_name) {
                            Ok(script_paths) => {
                                run_sh_scripts_with_deps(&script_paths, app_state);
                            }
                            Err(e) => {
                                log_println!(
                                    "{}",
                                    app_state.get_formatted_translation(
                                        "script_execution.failed_read_info",
                                        &[&e.to_string()]
                                    )
                                );
                                return;
                            }
                        }
                    }
                } else if idx <= names.len() + custom_scripts.len() {
                    // 自定义脚本
                    let custom_idx = idx - names.len() - 1;
                    let (_, custom_script) = custom_scripts[custom_idx];
                    log_println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "script_execution.running_script",
                            &[&custom_script.name]
                        )
                    );
                    match &custom_script.file_path {
                        Some(file_path) => run_custom_script_from_file(file_path, app_state),
                        None => {
                            if let Some(url) = &custom_script.url {
                                log_println!("⚠️  脚本没有保存的文件路径，正在从URL重新下载...");
                                run_custom_script_from_url(url, app_state);
                            } else {
                                log_println!("❌ 脚本既没有文件路径也没有URL，无法执行");
                            }
                        }
                    }
                } else {
                    // 插件脚本
                    let plugin_idx = idx - names.len() - custom_scripts.len() - 1;
                    let (name, _, script_path) = &plugin_scripts[plugin_idx];
                    log_println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "script_execution.running_script",
                            &[name]
                        )
                    );
                    log_println!("正在执行插件脚本: {}", script_path.file_name().unwrap_or_default().to_string_lossy());
                    run_sh_script(script_path, app_state);
                }
                return;
            }
        }
        log_println!(
            "{}",
            app_state.get_formatted_translation(
                "script_execution.invalid_choice",
                &[&total_scripts.to_string()]
            )
        );
    }
}

// 根据脚本的 shebang 选择解释器执行脚本
fn execute_script(path: &Path) -> io::Result<process::ExitStatus> {
    if let Ok(content) = fileio::read(path) {
        if let Some(first_line) = content.lines().next() {
            if let Some(stripped) = first_line.strip_prefix("#!") {
                let parts: Vec<&str> = stripped.trim().split_whitespace().collect();
                if let Some(program) = parts.first() {
                    let mut cmd = Command::new(program);
                    for arg in &parts[1..] {
                        cmd.arg(arg);
                    }
                    return cmd.arg(path).status();
                }
            }
        }
    }
    Command::new("sh").arg(path).status()
}

// 直接执行 .sh
fn run_sh_script(path: &Path, app_state: &AppState) {
    match execute_script(path) {
        Ok(status) if !status.success() => log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_status", &[&status.to_string()])
        ),
        Err(e) => log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_execute", &[&e.to_string()])
        ),
        _ => {}
    }
}

// 运行自定义脚本（从文件）
fn run_custom_script_from_file(file_path: &str, app_state: &AppState) {
    let script_path = Path::new(file_path);
    
    if !script_path.exists() {
        log_println!("❌ 脚本文件不存在: {}", file_path);
        log_println!("   提示：请尝试重新添加此脚本");
        return;
    }
    
    log_println!("正在执行自定义脚本: {}", script_path.file_name().unwrap_or_default().to_string_lossy());
    match execute_script(script_path) {
        Ok(status) if status.success() => {
            log_println!("{}", app_state.get_translation("url_script.success"));
        }
        Ok(status) => {
            log_println!("❌ 自定义脚本执行失败，退出码: {}", status);
        }
        Err(e) => {
            log_println!("❌ 自定义脚本执行出错: {}", e);
        }
    }
}

// 运行自定义脚本（从URL下载，向后兼容）
fn run_custom_script_from_url(url: &str, _app_state: &AppState) {
    log_println!("正在从URL下载自定义脚本: {}", url);
    
    match download_script_content(url) {
        Ok(content) => {
            let mut tmp_path = env::temp_dir();
            let file_name = format!("custom_script_{}.sh", rand::random::<u64>());
            tmp_path.push(file_name);
            
            if let Err(e) = fileio::write(&tmp_path, &content) {
                log_println!("❌ 写入脚本失败: {}", e);
                return;
            }
            
            #[cfg(unix)]
            {
                let _ = fileio::set_executable(&tmp_path);
            }
            
            log_println!("正在执行自定义脚本...");
            match execute_script(&tmp_path) {
                Ok(status) if status.success() => {
                    log_println!("✅ 自定义脚本执行成功");
                }
                Ok(status) => {
                    log_println!("❌ 自定义脚本执行失败，退出码: {}", status);
                }
                Err(e) => {
                    log_println!("❌ 自定义脚本执行出错: {}", e);
                }
            }
            
            let _ = fileio::remove_file(&tmp_path);
        }
        Err(e) => {
            log_println!("❌ 下载自定义脚本失败: {}", e);
        }
    }
}

// 按顺序执行多个 .sh 脚本（支持依赖关系）
fn run_sh_scripts_with_deps(paths: &[PathBuf], app_state: &AppState) {
    if paths.is_empty() {
        log_println!("{}", app_state.get_translation("script_execution.no_scripts"));
        return;
    }
    
    for (i, path) in paths.iter().enumerate() {
        let script_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        if paths.len() > 1 {
            log_println!(
                "正在执行脚本 {}/{}: {}",
                i + 1,
                paths.len(),
                script_name
            );
        }
        
        match execute_script(path) {
            Ok(status) if status.success() => {
                if paths.len() > 1 {
                    log_println!("✅ {} 执行成功", script_name);
                }
            }
            Ok(status) => {
                log_println!(
                    "❌ {} 执行失败，退出码: {}",
                    script_name,
                    status
                );
                log_println!("停止执行后续脚本");
                return;
            }
            Err(e) => {
                log_println!(
                    "❌ {} 执行出错: {}",
                    script_name,
                    e
                );
                log_println!("停止执行后续脚本");
                return;
            }
        }
    }
    
    if paths.len() > 1 {
        log_println!("🎉 所有脚本执行完成");
    }
}

// 处理 .link —— 下载远程脚本后执行
fn run_link_script(path: &Path, app_state: &AppState) {
    // 0. 清理缓存
    use std::env;

    let mut tmp_path = env::temp_dir();
    tmp_path.push("geektools");

    // 如果缓存目录存在则递归删除
    if tmp_path.exists() {
        if let Err(e) = fileio::remove_dir(&tmp_path) {
            log_eprintln!("⚠️  无法删除旧缓存目录 {:?}: {e}", tmp_path);
        }
    }

    // 重新创建空目录，忽略已存在的错误
    let _ = fileio::create_dir(&tmp_path);

    // 1. 读取 URL
    let url = match fileio::read(path) {
        Ok(s) => s.trim().to_string(),
        Err(e) => {
            log_println!(
                "{}",
                app_state.get_formatted_translation("link_script.failed_read", &[&e.to_string()])
            );
            return;
        }
    };
    log_println!(
        "{}",
        app_state.get_formatted_translation("link_script.downloading", &[&url])
    );

    // 2. 下载
    let resp = match reqwest::blocking::get(&url) {
        Ok(r) => r,
        Err(e) => {
            log_println!(
                "{}",
                app_state.get_formatted_translation("url_script.failed_fetch", &[&e.to_string()])
            );
            return;
        }
    };
    let content = match resp.text() {
        Ok(t) => t,
        Err(e) => {
            log_println!(
                "{}",
                app_state
                    .get_formatted_translation("url_script.failed_read_content", &[&e.to_string()])
            );
            return;
        }
    };

    // 3. 写入临时文件

    let file_name = format!("script_{}.sh", rand::random::<u64>());
    tmp_path.push(file_name);
    if let Err(e) = fileio::write(&tmp_path, &content) {
        log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_write", &[&e.to_string()])
        );
        return;
    }
    // 4. 设置可执行
    #[cfg(unix)]
    {
        if let Err(e) = fileio::set_executable(&tmp_path) {
            log_println!(
                "{}",
                app_state
                    .get_formatted_translation("url_script.failed_executable", &[&e.to_string()])
            );
        }
    }

    // 5. 执行
    log_println!("{}", app_state.get_translation("url_script.executing"));
    match execute_script(&tmp_path) {
        Ok(status) if status.success() => {
            log_println!("{}", app_state.get_translation("url_script.success"));
        }
        Ok(status) => log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_status", &[&status.to_string()])
        ),
        Err(e) => log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_execute", &[&e.to_string()])
        ),
    }

    // 6. 清理
    if let Err(e) = fileio::remove_file(&tmp_path) {
        log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_remove_temp", &[&e.to_string()])
        );
    }
}

// ──────────────────────────────── 手动输入脚本 URL ─────────────────────────
fn run_script_from_url(app_state: &AppState) {
    log_print!("{}", app_state.get_translation("url_script.enter_url"));
    let _ = io::stdout().flush();

    let mut url = String::new();
    if io::stdin().read_line(&mut url).is_err() {
        log_println!("{}", app_state.get_translation("main.invalid_choice"));
        return;
    }
    let url_trimmed = url.trim();
    if url_trimmed.eq_ignore_ascii_case("exit") {
        log_println!(
            "{}",
            app_state.get_translation("script_execution.returning")
        );
        return;
    }

    match reqwest::blocking::get(url_trimmed) {
        Ok(response) => match response.text() {
            Ok(script_content) => {
                log_println!(
                    "{}",
                    app_state.get_formatted_translation(
                        "url_script.script_content",
                        &[url_trimmed, &script_content]
                    )
                );

                // 落盘 → chmod → 执行
                let mut tmp_path = env::temp_dir();
                let file_name = format!("script_{}.sh", rand::random::<u64>());
                tmp_path.push(file_name);
                if let Err(e) = fileio::write(&tmp_path, &script_content) {
                    log_println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "url_script.failed_write",
                            &[&e.to_string()]
                        )
                    );
                    return;
                }
                #[cfg(unix)]
                {
                    let _ = fileio::set_executable(&tmp_path);
                }

                let status = execute_script(&tmp_path);
                match status {
                    Ok(s) if s.success() => {
                        log_println!("{}", app_state.get_translation("url_script.success"))
                    }
                    Ok(s) => log_println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "url_script.failed_status",
                            &[&s.to_string()]
                        )
                    ),
                    Err(e) => log_println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "url_script.failed_execute",
                            &[&e.to_string()]
                        )
                    ),
                }

                let _ = fileio::remove_file(&tmp_path);
            }
            Err(e) => log_println!(
                "{}",
                app_state
                    .get_formatted_translation("url_script.failed_read_content", &[&e.to_string()])
            ),
        },
        Err(e) => log_println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_fetch", &[&e.to_string()])
        ),
    }
}

// ─────────────────────────────────── 主函数 ───────────────────────────────

fn main() {
    let mut app_state = match AppState::new() {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to initialize application: {}", e);
            std::process::exit(1);
        }
    };
    log_println!("{}", app_state.get_translation("main.welcome"));

    log_println!(
        "{}",
        app_state.get_formatted_translation(
            "main.version_msg",
            &[
                env!("CARGO_PKG_VERSION"),
                format!("https://github.com/{}", env!("CARGO_PKG_REPOSITORY")).as_str()
            ]
        )
    );

    log_println!(
        "{}",
        app_state.get_formatted_translation(
            "main.buildtag_msg",
            &[
                BUILD_TAG,
                format!(
                    "https://github.com/{}/Buildtag.md",
                    env!("CARGO_PKG_REPOSITORY")
                )
                .as_str()
            ]
        )
    );
    loop {
        log_print!("{}", app_state.get_menu_text());
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }

        match choice.trim() {
            "1" => run_existing_script(&app_state),
            "2" => run_script_from_url(&app_state),
            "3" => show_custom_scripts_menu(&app_state),
            "4" => show_plugin_menu(&app_state),
            "5" => show_settings_menu(&mut app_state),
            "6" => {
                log_println!("{}", app_state.get_translation("main.exit_message"));
                process::exit(0);
            }
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }

        log_println!(); // 空行，美观
    }
}

// 显示设置菜单
fn show_settings_menu(app_state: &mut AppState) {
    loop {
        log_print!("{}", app_state.get_settings_menu_text());
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }

        match choice.trim() {
            "1" => {
                // 语言设置
                log_print!("{}", app_state.get_language_menu_text());
                let _ = io::stdout().flush();

                let mut lang_choice = String::new();
                if io::stdin().read_line(&mut lang_choice).is_err() {
                    log_println!("{}", app_state.get_translation("main.invalid_choice"));
                    continue;
                }
                match lang_choice.trim() {
                    "1" => {
                        app_state.current_language = Language::English;
                        let _ = save_language_to_config(app_state.current_language);
                    }
                    "2" => {
                        app_state.current_language = Language::Chinese;
                        let _ = save_language_to_config(app_state.current_language);
                    }
                    _ => log_println!("{}", app_state.get_translation("main.invalid_language")),
                }
            }
            "2" => change_version(app_state),
            "3" => {
                // 清理个性化设置
                if let Err(e) = fileio::remove_file(&*CONFIG_PATH) {
                    // Only show error if it's not a "file not found" error
                    match &e {
                        GeekToolsError::FileOperationError { source, .. } 
                            if source.kind() == io::ErrorKind::NotFound => {
                            // Ignore file not found errors
                        }
                        _ => {
                            log_println!("Failed to clear personalization: {}", e);
                        }
                    }
                }
                log_println!(
                    "{}",
                    app_state.get_translation("settings_menu.clear_success")
                );
                exit(0);
            }
            "4" => return, // 返回主菜单
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }

        log_println!(); // 空行，美观
    }
}

// 从 Cargo.toml 读取 repository 信息
fn repo_path_from_cargo() -> std::result::Result<String, GeekToolsError> {
    // 在编译时直接获取 repository 字段
    Ok(env!("CARGO_PKG_REPOSITORY").to_string())
}

// ─────────────────────────────── 自定义脚本管理 ───────────────────────────

/// 显示安全警告
fn show_security_warning(app_state: &AppState) -> bool {
    log_println!("\n⚠️  {}", app_state.get_translation("security.warning_title"));
    log_println!("{}", app_state.get_translation("security.warning_content"));
    log_println!("{}", app_state.get_translation("security.disclaimer"));
    log_println!("{}", app_state.get_translation("security.responsibility"));
    
    loop {
        log_print!("\n{}", app_state.get_translation("security.confirm_prompt"));
        let _ = io::stdout().flush();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "是" | "确认" => return true,
            "n" | "no" | "否" | "取消" => return false,
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }
    }
}

/// 从URL下载脚本内容
fn download_script_content(url: &str) -> std::result::Result<String, GeekToolsError> {
    let resp = reqwest::blocking::get(url)?;
    
    if !resp.status().is_success() {
        return Err(GeekToolsError::ConfigError {
            message: format!("HTTP error: {}", resp.status()),
        });
    }
    
    resp.text().map_err(GeekToolsError::from)
}

/// 解析脚本内容获取描述信息
fn parse_script_info(content: &str, default_name: &str) -> (String, String) {
    let mut name = default_name.to_string();
    let mut description = "无描述".to_string();
    
    for line in content.lines().take(20) { // 只检查前20行
        let line = line.trim();
        if line.starts_with("# Name:") || line.starts_with("#Name:") {
            name = line.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if line.starts_with("# Description:") || line.starts_with("#Description:") {
            description = line.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if line.starts_with("# 名称:") || line.starts_with("#名称:") {
            name = line.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if line.starts_with("# 描述:") || line.starts_with("#描述:") {
            description = line.split(':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    
    (name, description)
}

/// 添加自定义脚本
fn add_custom_script(app_state: &AppState) {
    if !show_security_warning(app_state) {
        log_println!("{}", app_state.get_translation("custom_script.cancelled"));
        return;
    }
    
    log_print!("{}", app_state.get_translation("custom_script.enter_url"));
    let _ = io::stdout().flush();
    
    let mut url = String::new();
    if io::stdin().read_line(&mut url).is_err() {
        log_println!("{}", app_state.get_translation("main.invalid_choice"));
        return;
    }
    
    let url = url.trim();
    if url.is_empty() || url.eq_ignore_ascii_case("exit") {
        return;
    }
    
    log_println!("{}", app_state.get_translation("custom_script.downloading"));
    
    match download_script_content(url) {
        Ok(content) => {
            let script_id = format!("custom_{}", rand::random::<u64>());
            let (name, description) = parse_script_info(&content, &script_id);
            
            log_println!("📝 检测到脚本信息:");
            log_println!("   名称: {}", name);
            log_println!("   描述: {}", description);
            
            log_print!("\n是否要编辑脚本信息? (y/N): ");
            let _ = io::stdout().flush();
            
            let mut edit_choice = String::new();
            let _ = io::stdin().read_line(&mut edit_choice);
            
            let (final_name, final_desc) = if edit_choice.trim().to_lowercase().starts_with("y") {
                // 编辑名称
                log_print!("输入脚本名称 (留空保持'{}'): ", name);
                let _ = io::stdout().flush();
                let mut new_name = String::new();
                let _ = io::stdin().read_line(&mut new_name);
                let new_name = new_name.trim();
                let final_name = if new_name.is_empty() { name } else { new_name.to_string() };
                
                // 编辑描述
                log_print!("输入脚本描述 (留空保持'{}'): ", description);
                let _ = io::stdout().flush();
                let mut new_desc = String::new();
                let _ = io::stdin().read_line(&mut new_desc);
                let new_desc = new_desc.trim();
                let final_desc = if new_desc.is_empty() { description } else { new_desc.to_string() };
                
                (final_name, final_desc)
            } else {
                (name, description)
            };
            
            // 创建自定义脚本目录
            if !CUSTOM_SCRIPTS_DIR.exists() {
                if let Err(e) = fileio::create_dir(&*CUSTOM_SCRIPTS_DIR) {
                    log_println!("❌ 创建脚本目录失败: {}", e);
                    return;
                }
            }
            
            // 保存脚本内容到文件
            let script_file_name = format!("{}.sh", script_id);
            let script_file_path = CUSTOM_SCRIPTS_DIR.join(&script_file_name);
            
            if let Err(e) = fileio::write(&script_file_path, &content) {
                log_println!("❌ 保存脚本文件失败: {}", e);
                return;
            }
            
            // 设置可执行权限
            #[cfg(unix)]
            {
                if let Err(e) = fileio::set_executable(&script_file_path) {
                    log_println!("⚠️  设置脚本可执行权限失败: {}", e);
                }
            }
            
            let custom_script = CustomScript {
                name: final_name.clone(),
                description: Some(final_desc.clone()),
                url: Some(url.to_string()),
                file_path: Some(script_file_path.to_string_lossy().to_string()),
                enabled: true,
                last_updated: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            };
            
            let mut config = load_user_config();
            config.custom_scripts.push(custom_script);
            
            match save_user_config(&config) {
                Ok(_) => {
                    log_println!("✅ 自定义脚本 '{}' 添加成功！", final_name);
                    log_println!("   ID: {}", script_id);
                }
                Err(e) => {
                    log_println!("❌ 保存配置失败: {}", e);
                }
            }
        }
        Err(e) => {
            log_println!("❌ {}", e);
        }
    }
}

/// 列出自定义脚本
fn list_custom_scripts(app_state: &AppState) {
    let config = load_user_config();
    
    if config.custom_scripts.is_empty() {
        log_println!("{}", app_state.get_translation("custom_script.no_scripts"));
        return;
    }
    
    log_println!("{}", app_state.get_translation("custom_script.list_title"));
    for (idx, script) in config.custom_scripts.iter().enumerate() {
        log_println!("📜 {} ({})", script.name, idx + 1);
        log_println!("   描述: {}", script.description.as_deref().unwrap_or("无描述"));
        log_println!("   URL: {}", script.url.as_deref().unwrap_or("本地文件"));
        log_println!("   更新时间: {}", script.last_updated.as_deref().unwrap_or("未知"));
        log_println!();
    }
}

/// 删除自定义脚本
fn remove_custom_script(app_state: &AppState) {
    let mut config = load_user_config();
    
    if config.custom_scripts.is_empty() {
        log_println!("{}", app_state.get_translation("custom_script.no_scripts"));
        return;
    }
    
    log_println!("{}", app_state.get_translation("custom_script.list_for_removal"));
    let scripts: Vec<(usize, &CustomScript)> = config.custom_scripts.iter().enumerate().collect();
    
    for (i, (idx, script)) in scripts.iter().enumerate() {
        log_println!("{}. {}", i + 1, script.name);
    }
    
    log_print!("选择要删除的脚本编号 (1-{}, 或输入 exit 退出): ", scripts.len());
    let _ = io::stdout().flush();
    
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return;
    }
    
    let input = input.trim();
    if input.eq_ignore_ascii_case("exit") {
        return;
    }
    
    if let Ok(idx) = input.parse::<usize>() {
        if (1..=scripts.len()).contains(&idx) {
            let (script_idx, script) = scripts[idx - 1];
            let script_name = script.name.clone();  // 克隆名称避免生命周期问题
            
            log_print!("确认删除脚本 '{}' 吗? (y/N): ", script_name);
            let _ = io::stdout().flush();
            
            let mut confirm = String::new();
            let _ = io::stdin().read_line(&mut confirm);
            
            if confirm.trim().to_lowercase().starts_with("y") {
                // 删除脚本文件（如果存在）
                if let Some(file_path) = &script.file_path {
                    let script_path = Path::new(file_path);
                    if script_path.exists() {
                        if let Err(e) = fileio::remove_file(script_path) {
                            log_println!("⚠️  删除脚本文件失败: {}", e);
                        } else {
                            log_println!("✅ 已删除脚本文件: {}", file_path);
                        }
                    }
                }
                
                config.custom_scripts.remove(script_idx);
                match save_user_config(&config) {
                    Ok(_) => log_println!("✅ 脚本 '{}' 已删除", script_name),
                    Err(e) => log_println!("❌ 删除失败: {}", e),
                }
            }
        } else {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
        }
    }
}

// 显示自定义脚本管理菜单
fn show_custom_scripts_menu(app_state: &AppState) {
    loop {
        log_print!("{}", app_state.get_custom_scripts_menu_text());
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }

        match choice.trim() {
            "1" => add_custom_script(app_state),
            "2" => list_custom_scripts(app_state),
            "3" => remove_custom_script(app_state),
            "4" => return, // 返回主菜单
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }

        log_println!(); // 空行，美观
    }
}

// 显示插件管理菜单
fn show_plugin_menu(app_state: &AppState) {
    let mut plugin_manager = PluginManager::new();
    
    loop {
        log_print!("{}", app_state.get_plugin_menu_text());
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }

        match choice.trim() {
            "1" => {
                // 插件市场管理
                show_marketplace_menu(app_state, &mut plugin_manager);
            }
            "2" => {
                // 本地插件扫描和导入
                show_local_scan_menu(app_state, &mut plugin_manager);
            }
            "3" => {
                // 安装插件
                log_print!("请输入插件包路径 (.tar.gz 文件): ");
                let _ = io::stdout().flush();
                
                let mut path_input = String::new();
                if io::stdin().read_line(&mut path_input).is_err() {
                    log_println!("{}", app_state.get_translation("main.invalid_choice"));
                    continue;
                }
                
                let plugin_path = path_input.trim();
                if plugin_path.is_empty() || plugin_path.eq_ignore_ascii_case("exit") {
                    continue;
                }
                
                match plugin_manager.install_plugin(Path::new(plugin_path)) {
                    Ok(plugin_id) => {
                        log_println!("✅ 插件安装成功！插件 ID: {}", plugin_id);
                    }
                    Err(e) => {
                        log_println!("❌ 插件安装失败: {}", e);
                    }
                }
            }
            "4" => {
                // 列出插件
                let plugins = plugin_manager.list_installed_plugins();
                if plugins.is_empty() {
                    log_println!("📋 暂无已安装的插件");
                } else {
                    log_println!("📋 已安装的插件:");
                    for plugin in plugins {
                        let status = if plugin.enabled { "✅ 已启用" } else { "❌ 已禁用" };
                        log_println!("  📦 {} ({})", plugin.info.name, plugin.info.id);
                        log_println!("     版本: {} | 状态: {}", plugin.info.version, status);
                        log_println!("     描述: {}", plugin.info.description);
                        log_println!("     作者: {} | 安装时间: {}", plugin.info.author, plugin.installed_at);
                        if !plugin.info.scripts.is_empty() {
                            log_println!("     脚本 ({} 个):", plugin.info.scripts.len());
                            for script in &plugin.info.scripts {
                                log_println!("       - {} ({})", script.name, script.file);
                            }
                        }
                        log_println!();
                    }
                }
            }
            "5" => {
                // 卸载插件
                let plugins = plugin_manager.list_installed_plugins();
                if plugins.is_empty() {
                    log_println!("📋 暂无已安装的插件");
                    continue;
                }
                
                log_println!("📋 选择要卸载的插件:");
                for (i, plugin) in plugins.iter().enumerate() {
                    log_println!("{}. {} ({})", i + 1, plugin.info.name, plugin.info.id);
                }
                
                log_print!("输入插件编号 (1-{}, 或输入 exit 退出): ", plugins.len());
                let _ = io::stdout().flush();
                
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    continue;
                }
                
                let input = input.trim();
                if input.eq_ignore_ascii_case("exit") {
                    continue;
                }
                
                if let Ok(idx) = input.parse::<usize>() {
                    if (1..=plugins.len()).contains(&idx) {
                        let plugin = &plugins[idx - 1];
                        let plugin_name = plugin.info.name.clone();
                        let plugin_id = plugin.info.id.clone();
                        
                        log_print!("确认卸载插件 '{}' 吗? (y/N): ", plugin_name);
                        let _ = io::stdout().flush();
                        
                        let mut confirm = String::new();
                        let _ = io::stdin().read_line(&mut confirm);
                        
                        if confirm.trim().to_lowercase().starts_with("y") {
                            match plugin_manager.uninstall_plugin(&plugin_id) {
                                Ok(_) => log_println!("✅ 插件 '{}' 卸载成功", plugin_name),
                                Err(e) => log_println!("❌ 卸载失败: {}", e),
                            }
                        }
                    } else {
                        log_println!("{}", app_state.get_translation("main.invalid_choice"));
                    }
                }
            }
            "6" => {
                // 启用/禁用插件
                let plugins = plugin_manager.list_installed_plugins();
                if plugins.is_empty() {
                    log_println!("📋 暂无已安装的插件");
                    continue;
                }
                
                log_println!("📋 选择要切换状态的插件:");
                for (i, plugin) in plugins.iter().enumerate() {
                    let status = if plugin.enabled { "✅ 已启用" } else { "❌ 已禁用" };
                    log_println!("{}. {} ({}) - {}", i + 1, plugin.info.name, plugin.info.id, status);
                }
                
                log_print!("输入插件编号 (1-{}, 或输入 exit 退出): ", plugins.len());
                let _ = io::stdout().flush();
                
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    continue;
                }
                
                let input = input.trim();
                if input.eq_ignore_ascii_case("exit") {
                    continue;
                }
                
                if let Ok(idx) = input.parse::<usize>() {
                    if (1..=plugins.len()).contains(&idx) {
                        let plugin = &plugins[idx - 1];
                        let plugin_id = plugin.info.id.clone();
                        let plugin_name = plugin.info.name.clone();
                        let new_status = !plugin.enabled;
                        let status_text = if new_status { "启用" } else { "禁用" };
                        
                        match plugin_manager.toggle_plugin(&plugin_id, new_status) {
                            Ok(_) => log_println!("✅ 插件 '{}' 已{}", plugin_name, status_text),
                            Err(e) => log_println!("❌ 操作失败: {}", e),
                        }
                    } else {
                        log_println!("{}", app_state.get_translation("main.invalid_choice"));
                    }
                }
            }
            "7" => return, // 返回主菜单
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }

        log_println!(); // 空行，美观
    }
}

// 显示插件市场管理菜单
fn show_marketplace_menu(app_state: &AppState, plugin_manager: &mut PluginManager) {
    loop {
        log_println!("\n=== 插件市场管理 ===");
        log_println!("1. 配置市场URL和端口");
        log_println!("2. 浏览插件市场");
        log_println!("3. 搜索插件");
        log_println!("4. 测试连接");
        log_println!("5. 返回");
        log_print!("请输入您的选择 (1-5): ");
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            log_println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }

        match choice.trim() {
            "1" => configure_marketplace(app_state),
            "2" => browse_marketplace(app_state, plugin_manager),
            "3" => search_marketplace(app_state, plugin_manager),
            "4" => test_marketplace_connection(app_state),
            "5" => return,
            _ => log_println!("{}", app_state.get_translation("main.invalid_choice")),
        }
        
        log_println!();
    }
}

// 配置插件市场URL和端口
fn configure_marketplace(_app_state: &AppState) {
    let mut config = load_user_config();
    
    log_println!("\n=== 配置插件市场 ===");
    log_println!("当前配置:");
    log_println!("  URL: {}", config.marketplace_config.api_url);
    log_println!("  端口: {}", config.marketplace_config.api_port);
    log_println!("  超时: {}秒", config.marketplace_config.timeout_seconds);
    
    // 配置URL
    log_print!("\n输入市场URL (留空保持当前值): ");
    let _ = io::stdout().flush();
    let mut url_input = String::new();
    if io::stdin().read_line(&mut url_input).is_ok() {
        let url_input = url_input.trim();
        if !url_input.is_empty() && !url_input.eq_ignore_ascii_case("exit") {
            config.marketplace_config.api_url = url_input.to_string();
        }
    }
    
    // 配置端口
    log_print!("输入API端口 (留空保持当前值，默认3000): ");
    let _ = io::stdout().flush();
    let mut port_input = String::new();
    if io::stdin().read_line(&mut port_input).is_ok() {
        let port_input = port_input.trim();
        if !port_input.is_empty() && !port_input.eq_ignore_ascii_case("exit") {
            if let Ok(port) = port_input.parse::<u16>() {
                config.marketplace_config.api_port = port;
            } else {
                log_println!("❌ 无效的端口号，保持原值");
            }
        }
    }
    
    // 保存配置
    match save_user_config(&config) {
        Ok(_) => {
            log_println!("✅ 市场配置已保存");
            log_println!("新配置: {}:{}", 
                config.marketplace_config.api_url, 
                config.marketplace_config.api_port);
        }
        Err(e) => log_println!("❌ 保存配置失败: {}", e),
    }
}

// 测试市场连接
fn test_marketplace_connection(_app_state: &AppState) {
    let config = load_user_config();
    log_println!("\n正在测试连接到 {}:{}...", 
        config.marketplace_config.api_url, 
        config.marketplace_config.api_port);
    
    match plugins::MarketplaceClient::new(config.marketplace_config.clone()) {
        Ok(client) => {
            match client.test_connection() {
                Ok(_) => log_println!("✅ 连接成功！插件市场服务正常运行"),
                Err(e) => log_println!("❌ 连接失败: {}", e),
            }
        }
        Err(e) => log_println!("❌ 创建客户端失败: {}", e),
    }
}

// 浏览插件市场
fn browse_marketplace(_app_state: &AppState, plugin_manager: &mut PluginManager) {
    let config = load_user_config();
    let client = match plugins::MarketplaceClient::new(config.marketplace_config.clone()) {
        Ok(client) => client,
        Err(e) => {
            log_println!("❌ 创建市场客户端失败: {}", e);
            return;
        }
    };

    let mut current_page = 1;
    let per_page = 10;
    let mut current_sort = plugins::SortBy::Rating;

    loop {
        log_println!("\n=== 插件市场浏览 (第{}页) ===", current_page);
        
        match client.get_plugins(current_page, per_page, Some(current_sort)) {
            Ok(response) => {
                if response.plugins.is_empty() {
                    log_println!("📋 当前页面没有插件");
                } else {
                    log_println!("找到 {} 个插件 (共 {} 个，第 {}/{} 页)", 
                        response.plugins.len(), response.total, 
                        response.page, response.total_pages);
                    log_println!();

                    for (i, plugin) in response.plugins.iter().enumerate() {
                        log_println!("{}. {} v{}", i + 1, plugin.name, plugin.version);
                        log_println!("   作者: {} | 评分: {:.1}/5.0 | 下载: {}", 
                            plugin.author, plugin.rating, plugin.download_count);
                        log_println!("   描述: {}", plugin.description);
                        if !plugin.tags.is_empty() {
                            log_println!("   标签: {}", plugin.tags.join(", "));
                        }
                        log_println!();
                    }

                    log_println!("操作选项:");
                    log_println!("  n - 下一页 | p - 上一页 | s - 排序 | i - 安装插件");
                    log_println!("  数字 - 查看详情 | exit - 返回");
                    log_print!("请输入选择: ");
                    let _ = io::stdout().flush();

                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        let input = input.trim();
                        match input {
                            "n" if current_page < response.total_pages => current_page += 1,
                            "p" if current_page > 1 => current_page -= 1,
                            "s" => current_sort = select_sort_method(),
                            "i" | "d" => download_plugin_from_market(&client, &response.plugins, plugin_manager),
                            "exit" => return,
                            num_str => {
                                if let Ok(num) = num_str.parse::<usize>() {
                                    if (1..=response.plugins.len()).contains(&num) {
                                        show_plugin_details(&response.plugins[num - 1]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log_println!("❌ 获取插件列表失败: {}", e);
                return;
            }
        }
    }
}

// 选择排序方式
fn select_sort_method() -> plugins::SortBy {
    log_println!("\n选择排序方式:");
    log_println!("1. 按名称排序");
    log_println!("2. 按评分排序");
    log_println!("3. 按下载量排序");
    log_println!("4. 按创建时间排序");
    log_println!("5. 按更新时间排序");
    log_print!("请选择 (1-5): ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(choice) = input.trim().parse::<usize>() {
            return plugins::SortBy::from_choice(choice).unwrap_or(plugins::SortBy::Rating);
        }
    }
    plugins::SortBy::Rating
}

// 显示插件详情
fn show_plugin_details(plugin: &plugins::MarketplacePlugin) {
    log_println!("\n=== 插件详情 ===");
    log_println!("名称: {}", plugin.name);
    log_println!("版本: {}", plugin.version);
    log_println!("作者: {}", plugin.author);
    log_println!("描述: {}", plugin.description);
    log_println!("评分: {:.1}/5.0", plugin.rating);
    log_println!("下载量: {}", plugin.download_count);
    log_println!("文件大小: {} 字节", plugin.file_size);
    log_println!("创建时间: {}", plugin.created_at);
    log_println!("更新时间: {}", plugin.updated_at);
    if !plugin.tags.is_empty() {
        log_println!("标签: {}", plugin.tags.join(", "));
    }
    log_println!("下载URL: {}", plugin.file_url);
}

// 从市场下载并安装插件
fn download_plugin_from_market(client: &plugins::MarketplaceClient, plugins_list: &[plugins::MarketplacePlugin], plugin_manager: &mut PluginManager) {
    log_print!("输入要下载的插件编号: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(num) = input.trim().parse::<usize>() {
            if (1..=plugins_list.len()).contains(&num) {
                let plugin = &plugins_list[num - 1];
                
                // 显示插件信息和免责声明
                log_println!("\n📦 准备安装插件：");
                log_println!("   名称: {}", plugin.name);
                log_println!("   版本: {}", plugin.version);
                log_println!("   作者: {}", plugin.author);
                log_println!("   描述: {}", plugin.description);
                log_println!("   评分: {:.1}/5.0 | 下载量: {}", plugin.rating, plugin.download_count);
                
                // 显示安全免责声明
                if !show_plugin_marketplace_disclaimer() {
                    log_println!("❌ 安装已取消");
                    return;
                }
                
                let download_path = env::temp_dir().join(format!("{}-{}.tar.gz", plugin.name, plugin.version));
                
                log_println!("正在下载 {} v{}...", plugin.name, plugin.version);
                
                // 如果没有file_url，尝试构建下载URL
                let download_url = if plugin.file_url.is_empty() {
                    let config = load_user_config();
                    format!("{}:{}/api/v1/plugins/{}/download", 
                        config.marketplace_config.api_url, 
                        config.marketplace_config.api_port,
                        plugin.id)
                } else {
                    plugin.file_url.clone()
                };
                
                match client.download_plugin(&download_url, &download_path) {
                    Ok(_) => {
                        log_println!("✅ 下载完成，正在安装...");
                        
                        // 直接安装下载的插件
                        match plugin_manager.install_plugin(&download_path) {
                            Ok(plugin_id) => {
                                log_println!("🎉 插件安装成功！");
                                log_println!("   插件ID: {}", plugin_id);
                                log_println!("   插件已启用，可在脚本列表中使用");
                                
                                // 清理临时文件
                                let _ = std::fs::remove_file(&download_path);
                            }
                            Err(e) => {
                                log_println!("❌ 插件安装失败: {}", e);
                                log_println!("   下载文件保留在: {:?}", download_path);
                                log_println!("   您可以稍后手动安装");
                            }
                        }
                    }
                    Err(e) => log_println!("❌ 下载失败: {}", e),
                }
            }
        }
    }
}

// 显示插件市场安装免责声明
fn show_plugin_marketplace_disclaimer() -> bool {
    log_println!("\n⚠️  插件安装免责声明");
    log_println!("════════════════════════════════════════");
    log_println!("您即将从插件市场安装第三方插件，请注意：");
    log_println!("• 插件来自第三方开发者，非GeekTools官方提供");
    log_println!("• 我们无法保证第三方插件的安全性和稳定性");
    log_println!("• 插件可能包含恶意代码或损坏您的系统");
    log_println!("• 插件执行可能会访问您的文件和系统资源");
    log_println!("• 安装和使用插件的风险由您自行承担");
    log_println!("• 建议仅安装来自可信开发者的插件");
    log_println!("════════════════════════════════════════");
    
    loop {
        log_print!("您确认理解上述风险并继续安装吗？(y/N): ");
        let _ = io::stdout().flush();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "是" | "确认" => return true,
            "n" | "no" | "否" | "取消" | "" => return false,
            _ => log_println!("请输入 y(是) 或 n(否)"),
        }
    }
}

// 搜索插件市场
fn search_marketplace(_app_state: &AppState, plugin_manager: &mut PluginManager) {
    let config = load_user_config();
    let client = match plugins::MarketplaceClient::new(config.marketplace_config.clone()) {
        Ok(client) => client,
        Err(e) => {
            log_println!("❌ 创建市场客户端失败: {}", e);
            return;
        }
    };

    log_print!("输入搜索关键词: ");
    let _ = io::stdout().flush();

    let mut query = String::new();
    if io::stdin().read_line(&mut query).is_err() {
        return;
    }

    let query = query.trim();
    if query.is_empty() || query.eq_ignore_ascii_case("exit") {
        return;
    }

    log_println!("正在搜索 '{}'...", query);
    match client.search_plugins(query) {
        Ok(response) => {
            if response.plugins.is_empty() {
                log_println!("❌ 没有找到匹配的插件");
            } else {
                log_println!("🔍 找到 {} 个匹配的插件:", response.total);
                log_println!();

                for (i, plugin) in response.plugins.iter().enumerate() {
                    log_println!("{}. {} v{}", i + 1, plugin.name, plugin.version);
                    log_println!("   作者: {} | 评分: {:.1}/5.0 | 下载: {}", 
                        plugin.author, plugin.rating, plugin.download_count);
                    log_println!("   描述: {}", plugin.description);
                    log_println!();
                }

                log_println!("操作选项:");
                log_println!("  数字 - 查看详情 | i - 安装插件 | exit - 返回");
                log_print!("请输入选择: ");
                let _ = io::stdout().flush();

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_ok() {
                    let input = input.trim();
                    match input {
                        "i" => download_plugin_from_market(&client, &response.plugins, plugin_manager),
                        "exit" | "" => return,
                        num_str => {
                            if let Ok(num) = num_str.parse::<usize>() {
                                if (1..=response.plugins.len()).contains(&num) {
                                    show_plugin_details(&response.plugins[num - 1]);
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => log_println!("❌ 搜索失败: {}", e),
    }
}

// 显示本地扫描菜单
fn show_local_scan_menu(_app_state: &AppState, plugin_manager: &mut PluginManager) {
    let scanner = plugins::LocalPluginScanner::new();
    
    log_println!("\n=== 本地插件扫描 ===");
    log_println!("正在扫描本地目录中的插件文件...");
    
    let local_plugins = scanner.scan_plugins();
    
    if local_plugins.is_empty() {
        log_println!("❌ 未找到任何插件文件");
        log_println!("扫描目录包括: ~/Downloads, ~/Desktop, ~/Documents, 当前目录");
        log_println!("请确保插件文件为 .tar.gz 格式");
        return;
    }
    
    log_println!("🔍 找到 {} 个潜在的插件文件:", local_plugins.len());
    log_println!();
    
    for (i, plugin) in local_plugins.iter().enumerate() {
        log_println!("{}. {}", i + 1, plugin.file_name);
        log_println!("   路径: {:?}", plugin.file_path);
        log_println!("   大小: {} 字节", plugin.file_size);
        log_println!("   修改时间: {}", plugin.modified_time);
        log_println!("   推测名称: {}", plugin.estimated_name);
        log_println!("   推测版本: {}", plugin.estimated_version);
        log_println!();
    }
    
    loop {
        log_print!("输入要安装的插件编号 (1-{}), 或输入 'exit' 返回: ", local_plugins.len());
        let _ = io::stdout().flush();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") {
            return;
        }
        
        if let Ok(num) = input.parse::<usize>() {
            if (1..=local_plugins.len()).contains(&num) {
                let plugin = &local_plugins[num - 1];
                
                log_println!("正在安装插件: {}", plugin.file_name);
                match plugin_manager.install_plugin(&plugin.file_path) {
                    Ok(plugin_id) => {
                        log_println!("✅ 插件安装成功！插件 ID: {}", plugin_id);
                        return;
                    }
                    Err(e) => {
                        log_println!("❌ 插件安装失败: {}", e);
                    }
                }
            } else {
                log_println!("❌ 无效的选择");
            }
        } else {
            log_println!("❌ 无效的输入");
        }
    }
}

// Legacy compatibility functions for backward compatibility with older code
fn load_user_config() -> Config {
    let config_path = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".geektools")
        .join("config.json");
    
    match ConfigManager::new(config_path) {
        Ok(manager) => {
            let config = manager.get_config();
            config.read().unwrap().clone()
        }
        Err(_) => Config::default(),
    }
}

fn save_user_config(config: &Config) -> std::result::Result<(), GeekToolsError> {
    let config_path = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".geektools")
        .join("config.json");
    
    let manager = ConfigManager::new(config_path)?;
    manager.update_config(|cfg| {
        *cfg = config.clone();
        Ok(())
    })
}

fn save_language_to_config(language: Language) -> std::result::Result<(), GeekToolsError> {
    let config_path = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".geektools")
        .join("config.json");
    
    let manager = ConfigManager::new(config_path)?;
    manager.update_config(|cfg| {
        cfg.language = match language {
            Language::Chinese => "zh".to_string(),
            Language::English => "en".to_string(),
        };
        Ok(())
    })
}
