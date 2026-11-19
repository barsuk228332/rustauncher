use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Путь к конфигу: ~/.config/rustauncher/config.toml
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rustauncher")
        .join("config.toml")
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    /// Имя раннера из ~/.local/share/rustauncher/runners, None = системный wine
    #[serde(default)]
    pub active_runner: Option<String>,
    #[serde(default)]
    pub prefixes: Vec<Prefix>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Prefix {
    pub name: String,
    pub path: String,
    /// Переопределение раннера для этого префикса (None = как в глобальных настройках)
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default = "default_winver")]
    pub windows_version: String,
    #[serde(default)]
    pub dll_overrides: Vec<String>,
    /// Переменные окружения префикса, формат "KEY=VALUE"
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub apps: Vec<AppEntry>,
}

fn default_winver() -> String {
    "win10".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppEntry {
    pub name: String,
    pub exe: String,
    #[serde(default)]
    pub args: String,
    /// Пустая строка = папка exe
    #[serde(default)]
    pub workdir: String,
    /// "KEY=VALUE"
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub dll_overrides: Vec<String>,
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("Не смог прочитать {}: {e}", path.display());
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl Prefix {
    /// DLL-оверрайды одной строкой для WINEDLLOVERRIDES
    pub fn overrides_env(&self) -> Option<String> {
        join_overrides(&self.dll_overrides)
    }
}

impl AppEntry {
    pub fn overrides_env(&self) -> Option<String> {
        join_overrides(&self.dll_overrides)
    }
}

fn join_overrides(items: &[String]) -> Option<String> {
    let items: Vec<&str> = items
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items.join(";"))
    }
}

/// Парсит строки "KEY=VALUE" в пары, игнорируя пустые и мусор.
pub fn parse_env(lines: &[String]) -> Vec<(String, String)> {
    lines
        .iter()
        .filter_map(|line| {
            let (k, v) = line.split_once('=')?;
            let k = k.trim();
            if k.is_empty() {
                None
            } else {
                Some((k.to_string(), v.trim().to_string()))
            }
        })
        .collect()
}
