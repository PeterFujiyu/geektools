mod fileio;
mod i18n;
mod scripts;

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
// 1️⃣ 统一的调试宏：只在 DEBUG 文件开启时打印
// ────────────────────────────────────────────────────────────────────────────
static DEBUG_ENABLED: Lazy<bool> = Lazy::new(|| {
    fileio::read("DEBUG")
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

// ───────────────────────────────── 语言枚举 ────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Language {
    English,
    Chinese,
}

// ───────────────────────────────── 翻译加载 ────────────────────────────────
use i18n::{EN_US_JSON, ZH_CN_JSON};

static TRANSLATIONS: Lazy<Arc<RwLock<HashMap<Language, Value>>>> = Lazy::new(|| {
    let mut translations = HashMap::new();

    if let Ok(json) = serde_json::from_str(EN_US_JSON) {
        translations.insert(Language::English, json);
    }
    if let Ok(json) = serde_json::from_str(ZH_CN_JSON) {
        translations.insert(Language::Chinese, json);
    }

    Arc::new(RwLock::new(translations))
});

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

static LOG_FILE: Lazy<Mutex<File>> = Lazy::new(|| {
    let file = fileio::open_append(&*LOG_FILE_PATH).unwrap_or_else(|e| {
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

/// 自定义脚本信息
#[derive(Deserialize, serde::Serialize, Clone)]
struct CustomScript {
    url: String,
    name: String,
    description: String,
    added_time: String,
    #[serde(default)]
    file_path: Option<String>,
}

/// 存储在 ~/.geektools/config.json 中的配置
#[derive(Deserialize, serde::Serialize)]
struct UserConfig {
    language: String,
    #[serde(default)]
    custom_scripts: std::collections::HashMap<String, CustomScript>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language: "English".into(),
            custom_scripts: std::collections::HashMap::new(),
        }
    }
}

/// 查询 IP-API 的返回结构
#[derive(Deserialize)]
struct IpApiResp {
    #[serde(rename = "countryCode")]
    country_code: String,
}

/// 加载或初始化用户语言
fn load_or_init_language() -> Language {
    // 1. 尝试读取现有配置
    match fileio::read(&*CONFIG_PATH) {
        Ok(text) => {
            if let Ok(cfg) = serde_json::from_str::<UserConfig>(&text) {
                return match cfg.language.as_str() {
                    "Chinese" => Language::Chinese,
                    _ => Language::English,
                };
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => { /* 文件不存在，走初始化 */ }
        Err(e) => {
            log_eprintln!("Failed to read config: {e}");
        }
    }

    // 2. 文件不存在或解析失败 → 调用 IP API 决定默认语言
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

    // 3. 保存到新配置文件
    let _ = save_language_to_config(default_lang);

    default_lang
}

/// 加载完整用户配置
fn load_user_config() -> UserConfig {
    match fileio::read(&*CONFIG_PATH) {
        Ok(text) => {
            serde_json::from_str::<UserConfig>(&text).unwrap_or_default()
        }
        Err(_) => UserConfig::default(),
    }
}

/// 保存完整用户配置
fn save_user_config(config: &UserConfig) -> io::Result<()> {
    let json = serde_json::to_string_pretty(config).unwrap_or_else(|_| "{}".into());
    fileio::write(&*CONFIG_PATH, &json)
}

/// 将语言写回 ~/.geektools/config.json
fn save_language_to_config(lang: Language) -> io::Result<()> {
    let mut config = load_user_config();
    config.language = match lang {
        Language::Chinese => "Chinese".into(),
        Language::English => "English".into(),
    };
    save_user_config(&config)
}

// ───────────────────────────────── 应用状态 ────────────────────────────────
struct AppState {
    language: Language,
}

impl AppState {
    fn new() -> Self {
        let lang = load_or_init_language();
        AppState { language: lang }
    }

    // 基础翻译
    fn get_translation(&self, key_path: &str) -> String {
        if let Ok(translations) = TRANSLATIONS.read() {
            if let Some(lang_translations) = translations.get(&self.language) {
                let mut current = lang_translations;
                for key in key_path.split('.') {
                    if let Some(value) = current.get(key) {
                        current = value;
                    } else {
                        return key_path.to_string(); // 未找到
                    }
                }
                if let Some(text) = current.as_str() {
                    return text.to_string();
                }
            }
        }
        key_path.to_string() // 回退
    }

    // 含占位符替换
    fn get_formatted_translation(&self, key_path: &str, args: &[&str]) -> String {
        let mut result = self.get_translation(key_path);
        for (i, arg) in args.iter().enumerate() {
            let numbered = format!("{{{}}}", i);
            if result.contains(&numbered) {
                result = result.replace(&numbered, arg);
            } else if result.contains("{}") {
                result = result.replacen("{}", arg, 1);
            }
        }
        result
    }

    // 主菜单文本
    fn get_menu_text(&self) -> String {
        format!(
            "\n{}\n1. {}\n2. {}\n3. {}\n4. {}\n5. {}\n{}",
            self.get_translation("menu.title"),
            self.get_translation("menu.run_existing_script"),
            self.get_translation("menu.run_script_from_network"),
            self.get_translation("menu.custom_scripts"),
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
fn fetch_releases() -> Result<Vec<GhRelease>, String> {
    let repo = repo_path_from_cargo()?;
    let url = format!("https://api.github.com/repos/{repo}/releases");
    debug_log!("[DEBUG] 即将请求 GitHub API: {url}");

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!(
            "geektools/{} (+{})",
            env!("CARGO_PKG_VERSION"),
            "PeterFujiyu/geektools"
        ))
        .build()
        .map_err(|e| format!("构建 client 失败: {e}"))?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("请求失败: {e}"))?;
    debug_log!("[DEBUG] 收到响应，状态码: {}", resp.status());

    if !resp.status().is_success() {
        return Err(format!("HTTP 非成功状态: {}", resp.status()));
    }

    let text = resp.text().map_err(|e| format!("读取响应正文失败: {e}"))?;
    debug_log!("[DEBUG] 响应体长度: {}", text.len());

    let releases: Vec<GhRelease> =
        serde_json::from_str(&text).map_err(|e| format!("JSON 解析失败: {e}"))?;
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

fn download_and_replace(url: &str) -> Result<(), String> {
    let resp = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut tmp = exe.clone();
    tmp.set_extension("tmp");
    fileio::write_bytes(&tmp, &bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        let _ = fileio::set_executable(&tmp);
    }
    fileio::rename(&tmp, &exe).map_err(|e| e.to_string())?;
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
            app_state.get_formatted_translation("update_menu.replace_failed", &[&e])
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
            app_state.get_formatted_translation("update_menu.download_failed", &[&e])
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
                app_state.get_formatted_translation("update_menu.download_failed", &[&e])
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
    let custom_scripts: Vec<(&String, &CustomScript)> = config.custom_scripts.iter().collect();

    // 3. 计算总脚本数量
    let total_scripts = map.len() + custom_scripts.len();
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
                v.get(match app_state.language {
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
        log_println!("{}. {} - {} [自定义]", names.len() + i + 1, script.name, script.description);
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
                } else {
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
                            log_println!("⚠️  脚本没有保存的文件路径，正在从URL重新下载...");
                            run_custom_script_from_url(&custom_script.url, app_state);
                        }
                    }
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
    let mut app_state = AppState::new();
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
            "4" => show_settings_menu(&mut app_state),
            "5" => {
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
                        app_state.language = Language::English;
                        let _ = save_language_to_config(app_state.language);
                    }
                    "2" => {
                        app_state.language = Language::Chinese;
                        let _ = save_language_to_config(app_state.language);
                    }
                    _ => log_println!("{}", app_state.get_translation("main.invalid_language")),
                }
            }
            "2" => change_version(app_state),
            "3" => {
                // 清理个性化设置
                if let Err(e) = fileio::remove_file(&*CONFIG_PATH) {
                    if e.kind() != io::ErrorKind::NotFound {
                        log_println!("Failed to clear personalization: {}", e);
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
fn repo_path_from_cargo() -> Result<String, String> {
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
fn download_script_content(url: &str) -> Result<String, String> {
    let resp = reqwest::blocking::get(url)
        .map_err(|e| format!("下载失败: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(format!("HTTP错误: {}", resp.status()));
    }
    
    resp.text().map_err(|e| format!("读取内容失败: {}", e))
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
                url: url.to_string(),
                name: final_name.clone(),
                description: final_desc.clone(),
                added_time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                file_path: Some(script_file_path.to_string_lossy().to_string()),
            };
            
            let mut config = load_user_config();
            config.custom_scripts.insert(script_id.clone(), custom_script);
            
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
    for (id, script) in &config.custom_scripts {
        log_println!("📜 {} ({})", script.name, id);
        log_println!("   描述: {}", script.description);
        log_println!("   URL: {}", script.url);
        log_println!("   添加时间: {}", script.added_time);
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
    let scripts: Vec<(&String, &CustomScript)> = config.custom_scripts.iter().collect();
    
    for (i, (id, script)) in scripts.iter().enumerate() {
        log_println!("{}. {} ({})", i + 1, script.name, id);
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
            let (id, script) = scripts[idx - 1];
            let script_name = script.name.clone();  // 克隆名称避免生命周期问题
            let script_id = id.clone();  // 克隆ID
            
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
                
                config.custom_scripts.remove(&script_id);
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
