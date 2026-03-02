use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::data::MAX_CHART_POINTS;

pub const AVAILABLE_PALETTES: &[&str] = &["gruvbox", "onedark"];

pub fn normalize_palette(value: &str) -> String {
    if AVAILABLE_PALETTES.iter().any(|name| *name == value) {
        value.to_string()
    } else {
        "gruvbox".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub palette: String,
    pub server_address: String,
    pub strict_table_file_binding: bool,
    pub max_chart_points: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            palette: "gruvbox".to_string(),
            server_address: "http://127.0.0.1:8080".to_string(),
            strict_table_file_binding: false,
            max_chart_points: MAX_CHART_POINTS,
        }
    }
}

pub fn config_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        return PathBuf::from(home)
            .join(".config")
            .join("rokku")
            .join("conf.toml");
    }

    #[cfg(target_os = "windows")]
    {
        let base = env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                env::var("USERPROFILE")
                    .ok()
                    .map(|p| PathBuf::from(p).join("AppData").join("Roaming"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        return base.join("Rokku").join("conf.toml");
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("conf.toml")
    }
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = config_path();
    if !path.exists() {
        log::debug!("Config file not found, using defaults: {}", path.display());
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Не удалось прочитать конфиг {}: {}", path.display(), e))?;
    let parsed = toml::from_str::<AppConfig>(&raw)
        .map_err(|e| format!("Некорректный формат конфига {}: {}", path.display(), e))?;
    log::debug!("Loaded config from {}", path.display());
    Ok(parsed)
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let Some(parent) = path.parent() else {
        return Err(format!("Некорректный путь конфига: {}", path.display()));
    };
    fs::create_dir_all(parent)
        .map_err(|e| format!("Не удалось создать каталог {}: {}", parent.display(), e))?;
    let raw = toml::to_string_pretty(config)
        .map_err(|e| format!("Не удалось сериализовать конфиг: {}", e))?;
    fs::write(&path, raw)
        .map_err(|e| format!("Не удалось сохранить конфиг {}: {}", path.display(), e))?;
    log::debug!("Saved config to {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_palette_keeps_known_value() {
        assert_eq!(normalize_palette("gruvbox"), "gruvbox");
        assert_eq!(normalize_palette("onedark"), "onedark");
    }

    #[test]
    fn normalize_palette_falls_back_for_unknown() {
        assert_eq!(normalize_palette("unknown-theme"), "gruvbox");
    }

    #[test]
    fn app_config_toml_roundtrip() {
        let cfg = AppConfig {
            palette: "onedark".to_string(),
            server_address: "http://localhost:9000".to_string(),
            strict_table_file_binding: true,
            max_chart_points: 12345,
        };

        let raw = toml::to_string_pretty(&cfg).expect("serialize config");
        let parsed: AppConfig = toml::from_str(&raw).expect("deserialize config");

        assert_eq!(parsed.palette, "onedark");
        assert_eq!(parsed.server_address, "http://localhost:9000");
        assert!(parsed.strict_table_file_binding);
        assert_eq!(parsed.max_chart_points, 12345);
    }
}
