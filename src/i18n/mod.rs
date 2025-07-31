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

/// 翻译函数：根据 key 和参数获取翻译文本
pub fn t(key: &str, params: &[(&str, &str)], lang: Language) -> String {
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
