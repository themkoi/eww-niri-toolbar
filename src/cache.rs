use serde::{Serialize, Deserialize};
use std::{collections::HashMap, fs, path::PathBuf};
use dirs::cache_dir;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CacheDate {
    pub icon_path: String,
}

pub type CacheMap = HashMap<String, CacheDate>;

pub fn get_cache_file() -> PathBuf {
    let mut path = cache_dir().unwrap();
    path.push("eww-niri-toolbar");
    fs::create_dir_all(&path).unwrap();
    path.push("cache.toml");
    path
}

pub fn load_history() -> CacheMap {
    let path = get_cache_file();
    if let Ok(data) = fs::read_to_string(&path) {
        toml::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn save_history(history: &CacheMap) {
    let toml_str = toml::to_string(history).unwrap();
    fs::write(get_cache_file(), toml_str).unwrap();
}

pub fn set_path(history: &mut CacheMap, appid: &str, icon_path: &str) {
    let entry = history.entry(appid.to_string()).or_default();
    entry.icon_path = icon_path.to_string();
}