use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeConfig {
    #[serde(default)]
    pub flake_path: Option<String>,
    #[serde(default)]
    pub terminal_command: Option<String>,
    #[serde(default)]
    pub rebuild_command: Option<String>,
    #[serde(default)]
    pub setup_completed: bool,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub last_page: Option<String>,
}

impl Default for WelcomeConfig {
    fn default() -> Self {
        Self {
            flake_path: None,
            terminal_command: None,
            rebuild_command: None,
            setup_completed: false,
            completed_at: None,
            last_page: None,
        }
    }
}

impl WelcomeConfig {
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"));
        base.join("ojuny-welcome").join("config.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create config dir: {error}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(&path, json).map_err(|error| format!("cannot write config: {error}"))
    }

    pub fn mark_completed(&mut self) {
        self.setup_completed = true;
        self.completed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs().to_string());
    }

    pub fn reset(&mut self) {
        self.setup_completed = false;
        self.completed_at = None;
        self.last_page = None;
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_reset_round_trip() {
        let mut config = WelcomeConfig::default();
        assert!(!config.setup_completed);
        config.mark_completed();
        assert!(config.setup_completed);
        assert!(config.completed_at.is_some());
        config.last_page = Some("audio".into());
        config.reset();
        assert!(!config.setup_completed);
        assert!(config.completed_at.is_none());
        assert!(config.last_page.is_none());
    }
}

