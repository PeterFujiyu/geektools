mod i18n;
mod scripts;

use std::{
    collections::HashMap,
    env,
    // fs::{self, File},
    fs::{self},
    io::{self, Write},
    // path::{Path, PathBuf},
    path::Path,
    process::{self, Command},
    sync::{Arc, RwLock},
};

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{self, Value};

// ────────────────────────────────────────────────────────────────────────────
// 1️⃣ 统一的调试宏：只在 DEBUG 文件开启时打印
// ────────────────────────────────────────────────────────────────────────────
static DEBUG_ENABLED: Lazy<bool> = Lazy::new(|| {
    fs::read_to_string("DEBUG")
        .map(|s| s.trim() == "DEBUG=true")
        .unwrap_or(false)
});

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if *DEBUG_ENABLED {
            // 直接写 stdout，避免被上层 logger 截断
            let _ = writeln!(std::io::stdout(), $($arg)*);
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

// ───────────────────────────────── 应用状态 ────────────────────────────────
struct AppState {
    language: Language,
}

impl AppState {
    fn new() -> Self {
        AppState {
            language: Language::English,
        }
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
            self.get_translation("menu.change_language"),
            self.get_translation("menu.change_version"),
            self.get_translation("menu.exit"),
            self.get_translation("menu.prompt")
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
    let url  = format!("https://api.github.com/repos/{repo}/releases");
    debug_log!("[DEBUG] 即将请求 GitHub API: {url}");

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!(
            "geektools/{} (+{})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY")
        ))
        .build()
        .map_err(|e| format!("构建 client 失败: {e}"))?;

    let resp = client.get(&url).send().map_err(|e| format!("请求失败: {e}"))?;
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
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755));
    }
    fs::rename(&tmp, &exe).map_err(|e| e.to_string())?;
    Ok(())
}

fn update_to_release(release: &GhRelease, app_state: &AppState) {
    let name = match asset_name() {
        Some(n) => n,
        None => {
            println!("{}", app_state.get_translation("update_menu.not_found"));
            return;
        }
    };
    let asset = match release.assets.iter().find(|a| a.name == name) {
        Some(a) => a,
        None => {
            println!("{}", app_state.get_translation("update_menu.not_found"));
            return;
        }
    };
    println!(
        "{}",
        app_state.get_formatted_translation("update_menu.downloading", &[&release.tag_name])
    );
    match download_and_replace(&asset.browser_download_url) {
        Ok(_) => println!("{}", app_state.get_translation("update_menu.success")),
        Err(e) => println!(
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
                println!("{}", app_state.get_translation("update_menu.no_release"));
            }
        }
        Err(e) => println!(
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
                println!("{}", app_state.get_translation("update_menu.no_release"));
                return;
            }

            // 根据 tag 名倒序，让最新的排在最前面（可按需要改成基于发布时间）
            releases.sort_by(|a, b| b.tag_name.cmp(&a.tag_name));

            // 输出版本列表，预发布版额外标记一下
            for (i, r) in releases.iter().enumerate() {
                if r.prerelease {
                    println!("{}. {} (prerelease)", i + 1, r.tag_name);
                } else {
                    println!("{}. {}", i + 1, r.tag_name);
                }
            }

            let prompt = app_state
                .get_formatted_translation("update_menu.select_prompt", &[&releases.len().to_string()]);

            loop {
                print!("{}", prompt);
                let _ = io::stdout().flush();

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    println!("{}", app_state.get_translation("main.invalid_choice"));
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

                println!("{}", app_state.get_translation("main.invalid_choice"));
            }
        }
        Err(e) => {
            eprintln!("[DEBUG] fetch_releases() 失败: {e}");
            println!(
                "{}",
                app_state.get_formatted_translation("update_menu.download_failed", &[&e])
            );
        }
    }
}

fn change_version(app_state: &AppState) {
    loop {
        println!(
            "\n{}\n1. {}\n2. {}\n3. {}",
            app_state.get_translation("update_menu.title"),
            app_state.get_translation("update_menu.latest"),
            app_state.get_translation("update_menu.latest_dev"),
            app_state.get_translation("update_menu.other")
        );
        print!("{}", app_state.get_translation("update_menu.prompt"));
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("{}", app_state.get_translation("main.invalid_choice"));
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
            _ => println!("{}", app_state.get_translation("main.invalid_choice")),
        }
    }
}

// ──────────────────────────────── 运行本地脚本 ─────────────────────────────
fn run_existing_script(app_state: &AppState) {
    // 0. 清理缓存
    use std::{env, fs};

    let mut tmp_path = env::temp_dir();
    tmp_path.push("geektools");

    // 如果缓存目录存在则递归删除
    if tmp_path.exists() {
        if let Err(e) = fs::remove_dir_all(&tmp_path) {
            eprintln!("⚠️  无法删除旧缓存目录 {:?}: {e}", tmp_path);
        }
    }

    // 重新创建空目录，忽略已存在的错误
    let _ = fs::create_dir_all(&tmp_path);
    // 1. 读取 info.json（已打包进二进制）
    let data = match scripts::get_string("info.json") {
        Some(s) => s,
        None => {
            println!(
                "{}",
                app_state.get_translation("script_execution.no_scripts")
            );
            return;
        }
    };

    let info: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            println!(
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
            println!(
                "{}",
                app_state.get_translation("script_execution.no_scripts")
            );
            return;
        }
    };

    // 2. 展示脚本列表
    println!(
        "{}",
        app_state.get_translation("script_execution.available_scripts")
    );
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
        println!("{}. {} - {}", i + 1, name, desc);
    }

    // 3. 处理用户选择
    let prompt = app_state
        .get_formatted_translation("script_execution.run_prompt", &[&names.len().to_string()]);
    loop {
        print!("{}", prompt);
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }
        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") {
            println!(
                "{}",
                app_state.get_translation("script_execution.returning")
            );
            return;
        }
        if let Ok(idx) = input.parse::<usize>() {
            if (1..=names.len()).contains(&idx) {
                let script_name = names[idx - 1];
                println!(
                    "{}",
                    app_state.get_formatted_translation(
                        "script_execution.running_script",
                        &[script_name]
                    )
                );

                // 将脚本释放到临时目录
                let script_path = match scripts::materialize(script_name) {
                    Ok(p) => p,
                    Err(e) => {
                        println!(
                            "{}",
                            app_state.get_formatted_translation(
                                "script_execution.failed_read_info",
                                &[&e.to_string()]
                            )
                        );
                        return;
                    }
                };

                if script_name.ends_with(".link") {
                    run_link_script(&script_path, app_state);
                } else {
                    run_sh_script(&script_path, app_state);
                }
                return;
            }
        }
        println!(
            "{}",
            app_state.get_formatted_translation(
                "script_execution.invalid_choice",
                &[&names.len().to_string()]
            )
        );
    }
}

// 根据脚本的 shebang 选择解释器执行脚本
fn execute_script(path: &Path) -> io::Result<process::ExitStatus> {
    if let Ok(content) = fs::read_to_string(path) {
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
        Ok(status) if !status.success() => println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_status", &[&status.to_string()])
        ),
        Err(e) => println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_execute", &[&e.to_string()])
        ),
        _ => {}
    }
}

// 处理 .link —— 下载远程脚本后执行
fn run_link_script(path: &Path, app_state: &AppState) {
    // 0. 清理缓存
    use std::{env, fs};

    let mut tmp_path = env::temp_dir();
    tmp_path.push("geektools");

    // 如果缓存目录存在则递归删除
    if tmp_path.exists() {
        if let Err(e) = fs::remove_dir_all(&tmp_path) {
            eprintln!("⚠️  无法删除旧缓存目录 {:?}: {e}", tmp_path);
        }
    }

    // 重新创建空目录，忽略已存在的错误
    let _ = fs::create_dir_all(&tmp_path);

    // 1. 读取 URL
    let url = match fs::read_to_string(path) {
        Ok(s) => s.trim().to_string(),
        Err(e) => {
            println!(
                "{}",
                app_state.get_formatted_translation("link_script.failed_read", &[&e.to_string()])
            );
            return;
        }
    };
    println!(
        "{}",
        app_state.get_formatted_translation("link_script.downloading", &[&url])
    );

    // 2. 下载
    let resp = match reqwest::blocking::get(&url) {
        Ok(r) => r,
        Err(e) => {
            println!(
                "{}",
                app_state.get_formatted_translation("url_script.failed_fetch", &[&e.to_string()])
            );
            return;
        }
    };
    let content = match resp.text() {
        Ok(t) => t,
        Err(e) => {
            println!(
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
    if let Err(e) = fs::write(&tmp_path, &content) {
        println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_write", &[&e.to_string()])
        );
        return;
    }
    // 4. 设置可执行
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755)) {
            println!(
                "{}",
                app_state
                    .get_formatted_translation("url_script.failed_executable", &[&e.to_string()])
            );
        }
    }

    // 5. 执行
    println!("{}", app_state.get_translation("url_script.executing"));
    match execute_script(&tmp_path) {
        Ok(status) if status.success() => {
            println!("{}", app_state.get_translation("url_script.success"));
        }
        Ok(status) => println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_status", &[&status.to_string()])
        ),
        Err(e) => println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_execute", &[&e.to_string()])
        ),
    }

    // 6. 清理
    if let Err(e) = fs::remove_file(&tmp_path) {
        println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_remove_temp", &[&e.to_string()])
        );
    }
}

// ──────────────────────────────── 手动输入脚本 URL ─────────────────────────
fn run_script_from_url(app_state: &AppState) {
    print!("{}", app_state.get_translation("url_script.enter_url"));
    let _ = io::stdout().flush();

    let mut url = String::new();
    if io::stdin().read_line(&mut url).is_err() {
        println!("{}", app_state.get_translation("main.invalid_choice"));
        return;
    }
    let url_trimmed = url.trim();
    if url_trimmed.eq_ignore_ascii_case("exit") {
        println!(
            "{}",
            app_state.get_translation("script_execution.returning")
        );
        return;
    }

    match reqwest::blocking::get(url_trimmed) {
        Ok(response) => match response.text() {
            Ok(script_content) => {
                println!(
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
                if let Err(e) = fs::write(&tmp_path, script_content.as_bytes()) {
                    println!(
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
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755));
                }

                let status = execute_script(&tmp_path);
                match status {
                    Ok(s) if s.success() => {
                        println!("{}", app_state.get_translation("url_script.success"))
                    }
                    Ok(s) => println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "url_script.failed_status",
                            &[&s.to_string()]
                        )
                    ),
                    Err(e) => println!(
                        "{}",
                        app_state.get_formatted_translation(
                            "url_script.failed_execute",
                            &[&e.to_string()]
                        )
                    ),
                }

                let _ = fs::remove_file(&tmp_path);
            }
            Err(e) => println!(
                "{}",
                app_state
                    .get_formatted_translation("url_script.failed_read_content", &[&e.to_string()])
            ),
        },
        Err(e) => println!(
            "{}",
            app_state.get_formatted_translation("url_script.failed_fetch", &[&e.to_string()])
        ),
    }
}

// ─────────────────────────────────── 主函数 ───────────────────────────────

fn main() {
    let mut app_state = AppState::new();
    println!("{}", app_state.get_translation("main.welcome"));

    println!("{}", app_state.get_formatted_translation("main.version_msg", &[env!("CARGO_PKG_VERSION"), format!("https://github.com/{}", env!("CARGO_PKG_REPOSITORY")).as_str()]));

    loop {
        print!("{}", app_state.get_menu_text());
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            println!("{}", app_state.get_translation("main.invalid_choice"));
            continue;
        }

        match choice.trim() {
            "1" => run_existing_script(&app_state),
            "2" => run_script_from_url(&app_state),
            "3" => {
                print!("{}", app_state.get_language_menu_text());
                let _ = io::stdout().flush();

                let mut lang_choice = String::new();
                if io::stdin().read_line(&mut lang_choice).is_err() {
                    println!("{}", app_state.get_translation("main.invalid_choice"));
                    continue;
                }
                match lang_choice.trim() {
                    "1" => app_state.language = Language::English,
                    "2" => app_state.language = Language::Chinese,
                    _ => println!("{}", app_state.get_translation("main.invalid_language")),
                }
            }
            "4" => change_version(&app_state),
            "5" => {
                println!("{}", app_state.get_translation("main.exit_message"));
                process::exit(0);
            }
            _ => println!("{}", app_state.get_translation("main.invalid_choice")),
        }

        println!(); // 空行，美观
    }
}

// 从 Cargo.toml 读取 repository 信息
fn repo_path_from_cargo() -> Result<String, String> {
    // 读取 Cargo.toml 文件
    let cargo_toml_path = Path::new("Cargo.toml");
    let toml_content = fs::read_to_string(cargo_toml_path).map_err(|e| format!("读取 Cargo.toml 失败: {}", e))?;

    // 解析 Cargo.toml
    let toml_data: toml::Value = toml::from_str(&toml_content).map_err(|e| format!("解析 Cargo.toml 失败: {}", e))?;

    // 提取 repository 字段
    let repository = toml_data
        .get("package")
        .and_then(|package| package.get("repository"))
        .and_then(|repository| repository.as_str())
        .ok_or("Cargo.toml 中缺少 'package.repository' 字段")?;

    // 提取 GitHub 用户名和仓库名
    let repo_path = repository
        .trim_start_matches("https://github.com/")
        .trim_end_matches(".git")
        .to_string();

    Ok(repo_path)
}