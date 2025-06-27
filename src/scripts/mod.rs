use std::{env, fs, io, path::PathBuf};

use once_cell::sync::Lazy;
use rust_embed::RustEmbed;

/// 嵌入 scripts 目录下的全部文件
#[derive(RustEmbed)]
#[folder = "src/scripts/"]
// 若将来想排除临时文件，可加 exclude = ["*.tmp"]
struct Assets;

/// 临时目录 (每次程序启动只创建一次)
static TMP_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let dir = env::temp_dir().join("rustsimpin_scripts");
    // ignore error if exists
    let _ = fs::create_dir_all(&dir);
    dir
});

/// 把指定脚本写到临时目录并返回可执行路径
pub fn materialize(name: &str) -> io::Result<PathBuf> {
    // 1) 从 embed 中取二进制内容
    let data = Assets::get(name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name))?;

    // 2) 写入 <tmp>/name
    let dest = TMP_DIR.join(name);
    if !dest.exists() {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, data.data.as_ref())?;
        // 3) chmod +x （Unix；Windows 会忽略）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = dest.metadata()?.permissions();
            perm.set_mode(0o755);
            fs::set_permissions(&dest, perm)?;
        }
    }
    Ok(dest)
}
pub fn get_string(name: &str) -> Option<String> {
    Assets::get(name)
        .map(|data| String::from_utf8_lossy(data.data.as_ref()).into_owned())
}
