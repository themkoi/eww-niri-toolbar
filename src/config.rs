use config::{Config as ConfigLoader, File};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SortingMode {
    Default,
    AZ,
    Id,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    pub icon_theme: String,
    pub icon_size: u16,
    pub seperate_workspaces: bool,
    pub sorting_mode: SortingMode,
    pub check_cache_validity: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub general: GeneralConfig,
}

fn default_config() -> Config {
    Config {
        general: GeneralConfig {
            icon_theme: "Papirus-Dark".to_string(),
            icon_size: 16,
            seperate_workspaces: true,
            sorting_mode: SortingMode::Default,
            check_cache_validity: true,
        }
    }
}

fn get_config_file() -> PathBuf {
    let mut path = config_dir().unwrap();
    path.push("eww-niri-toolbar");
    fs::create_dir_all(&path).unwrap();
    path.push("config.toml");
    path
}

fn write_config<P: AsRef<Path>>(path: P, config: &Config) -> std::io::Result<()> {
    let toml_string = toml::to_string_pretty(config).expect("Failed to serialize config");
    fs::write(path, toml_string)
}

pub fn load_or_create_config() -> Config {
    let path = get_config_file();

    if !path.exists() {
        let default = default_config();
        let _ = write_config(&path, &default);
        return default;
    }

    let loaded = ConfigLoader::builder()
        .add_source(File::with_name(path.to_str().unwrap()))
        .build();

    match loaded {
        Ok(cfg) => match cfg.try_deserialize::<Config>() {
            Ok(conf) => conf,
            Err(_) => {
                let default = default_config();
                let _ = write_config(&path, &default);
                default
            }
        },
        Err(_) => {
            let default = default_config();
            let _ = write_config(&path, &default);
            default
        }
    }
}
