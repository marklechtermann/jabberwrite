//! User configuration stored as JSON in the per-user config directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Global shortcut that toggles recording, e.g. "Ctrl+Super".
    pub shortcut: String,
    /// Keystroke sent to paste the text, e.g. "Ctrl+V" or "Ctrl+Shift+V".
    pub paste_shortcut: String,
    /// Directory with the model files; defaults to the downloaded model.
    pub model_path: Option<PathBuf>,
    /// Delete the recognized-text history whenever Jabberwrite quits.
    pub clear_history_on_exit: bool,
}

impl Default for Config {
    fn default() -> Self {
        if cfg!(target_os = "macos") {
            Config {
                shortcut: "Ctrl+Cmd".into(),
                paste_shortcut: "Cmd+V".into(),
                model_path: None,
                clear_history_on_exit: false,
            }
        } else {
            Config {
                shortcut: "Ctrl+Super".into(),
                paste_shortcut: "Ctrl+V".into(),
                model_path: None,
                clear_history_on_exit: false,
            }
        }
    }
}

/// Result of loading: the config to use plus an error if the file was invalid.
pub struct Loaded {
    pub config: Config,
    pub error: Option<String>,
}

/// Load the config, creating it with defaults if missing.
/// An invalid file is never overwritten; defaults are used instead.
pub fn load(path: &Path) -> Loaded {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(config) => Loaded {
                config,
                error: None,
            },
            Err(e) => Loaded {
                config: Config::default(),
                error: Some(format!("{}: {e}", path.display())),
            },
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            let error = save(path, &config).err();
            Loaded { config, error }
        }
        Err(e) => Loaded {
            config: Config::default(),
            error: Some(format!("{}: {e}", path.display())),
        },
    }
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, text + "\n").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("jabberwrite-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("config.json")
    }

    #[test]
    fn creates_defaults_when_missing() {
        let path = temp_path("missing");
        let loaded = load(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config, Config::default());
        assert!(path.exists());
    }

    #[test]
    fn invalid_file_is_kept() {
        let path = temp_path("invalid");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        let loaded = load(&path);
        assert!(loaded.error.is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
    }

    #[test]
    fn missing_fields_use_defaults() {
        let path = temp_path("partial");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{ "shortcut": "Ctrl+Alt+Space" }"#).unwrap();
        let loaded = load(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config.shortcut, "Ctrl+Alt+Space");
        assert_eq!(
            loaded.config.paste_shortcut,
            Config::default().paste_shortcut
        );
    }
}
