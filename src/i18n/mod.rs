use serde_json::Value;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

pub const EN_US_JSON: &str = include_str!("en_us.json");
pub const ZH_CN_JSON: &str = include_str!("zh_cn.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Chinese,
}

// 性能优化：按需延迟加载翻译，避免启动时解析所有JSON
static TRANSLATIONS: Lazy<Arc<RwLock<HashMap<Language, Value>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

/// 延迟加载指定语言的翻译
fn ensure_language_loaded(lang: Language) {
    let mut translations = TRANSLATIONS.write().unwrap();
    if !translations.contains_key(&lang) {
        let json_content = match lang {
            Language::English => EN_US_JSON,
            Language::Chinese => ZH_CN_JSON,
        };
        
        if let Ok(json) = serde_json::from_str(json_content) {
            translations.insert(lang, json);
        }
    }
}

/// 翻译函数：根据 key 和参数获取翻译文本，按需加载
pub fn t(key: &str, params: &[(&str, &str)], lang: Language) -> String {
    // 确保语言包已加载
    ensure_language_loaded(lang);
    
    let translations = TRANSLATIONS.read().unwrap();
    
    if let Some(lang_map) = translations.get(&lang) {
        if let Some(text) = get_nested_value(lang_map, key) {
            if let Some(text_str) = text.as_str() {
                let mut result = text_str.to_string();
                for (param_key, param_value) in params {
                    result = result.replace(&format!("{{{}}}", param_key), param_value);
                }
                return result;
            }
        }
    }
    
    // 如果翻译不存在，返回 key 本身
    key.to_string()
}

/// 从嵌套的 JSON 对象中获取值
fn get_nested_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;
    
    for part in parts {
        current = current.get(part)?;
    }
    
    Some(current)
}
