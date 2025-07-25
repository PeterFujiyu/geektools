use crate::fileio;
use std::{collections::{HashMap, HashSet}, env, io, path::PathBuf};

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
    let _ = fileio::create_dir(&dir);
    dir
});

/// 把指定脚本写到临时目录并返回可执行路径
pub fn materialize(name: &str) -> io::Result<PathBuf> {
    // 1) 从 embed 中取二进制内容
    let data = Assets::get(name).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name))?;

    // 2) 写入 <tmp>/name
    let dest = TMP_DIR.join(name);
    if !dest.exists() {
        if let Some(parent) = dest.parent() {
            fileio::create_dir(parent)?;
        }
        fileio::write_bytes(&dest, data.data.as_ref())?;
        // 3) chmod +x （Unix；Windows 会忽略）
        #[cfg(unix)]
        {
            fileio::set_executable(&dest)?;
        }
    }
    Ok(dest)
}
pub fn get_string(name: &str) -> Option<String> {
    Assets::get(name).map(|data| String::from_utf8_lossy(data.data.as_ref()).into_owned())
}

/// 解析脚本中的导入声明
fn parse_imports(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("#@import ") {
                Some(trimmed[9..].trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

/// 检测循环依赖
fn detect_cycles(deps: &HashMap<String, Vec<String>>) -> Result<(), String> {
    fn visit(
        node: &str,
        deps: &HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), String> {
        if visiting.contains(node) {
            return Err(format!("Circular dependency detected involving: {}", node));
        }
        if visited.contains(node) {
            return Ok(());
        }

        visiting.insert(node.to_string());
        
        if let Some(children) = deps.get(node) {
            for child in children {
                visit(child, deps, visiting, visited)?;
            }
        }
        
        visiting.remove(node);
        visited.insert(node.to_string());
        Ok(())
    }

    let mut visited = HashSet::new();
    for node in deps.keys() {
        if !visited.contains(node) {
            let mut visiting = HashSet::new();
            visit(node, deps, &mut visiting, &mut visited)?;
        }
    }
    Ok(())
}

/// 拓扑排序，返回执行顺序
fn topological_sort(deps: &HashMap<String, Vec<String>>) -> Result<Vec<String>, String> {
    detect_cycles(deps)?;
    
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut all_nodes = HashSet::new();
    
    // 初始化所有节点的入度
    for (node, children) in deps {
        all_nodes.insert(node.clone());
        in_degree.entry(node.clone()).or_insert(0);
        for child in children {
            all_nodes.insert(child.clone());
            *in_degree.entry(child.clone()).or_insert(0) += 1;
        }
    }
    
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|&(_, &degree)| degree == 0)
        .map(|(node, _)| node.clone())
        .collect();
    
    let mut result = Vec::new();
    
    while let Some(node) = queue.pop() {
        result.push(node.clone());
        
        if let Some(children) = deps.get(&node) {
            for child in children {
                if let Some(degree) = in_degree.get_mut(child) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(child.clone());
                    }
                }
            }
        }
    }
    
    if result.len() != all_nodes.len() {
        return Err("Failed to resolve all dependencies".to_string());
    }
    
    Ok(result)
}

/// 递归解析脚本及其依赖
fn resolve_dependencies(script_name: &str) -> Result<Vec<String>, String> {
    let mut deps = HashMap::new();
    let mut to_process = vec![script_name.to_string()];
    let mut processed = HashSet::new();
    
    while let Some(current) = to_process.pop() {
        if processed.contains(&current) {
            continue;
        }
        
        let content = get_string(&current)
            .ok_or_else(|| format!("Script not found: {}", current))?;
        
        let imports = parse_imports(&content);
        deps.insert(current.clone(), imports.clone());
        
        for import in imports {
            if !processed.contains(&import) {
                to_process.push(import);
            }
        }
        
        processed.insert(current);
    }
    
    topological_sort(&deps)
}

/// 把脚本及其依赖按顺序写到临时目录并返回执行顺序
pub fn materialize_with_deps(name: &str) -> io::Result<Vec<PathBuf>> {
    let execution_order = resolve_dependencies(name)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    
    let mut paths = Vec::new();
    
    for script_name in execution_order {
        // 只包含 .sh 脚本，跳过 .link 和其他文件
        if script_name.ends_with(".sh") {
            let path = materialize(&script_name)?;
            paths.push(path);
        }
    }
    
    Ok(paths)
}
