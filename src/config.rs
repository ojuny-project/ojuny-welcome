use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use nix_settings_gui::config::ViewMode;

#[derive(Debug, Clone)]
pub struct GuiSettings {
    pub view_mode: ViewMode,
    pub live_apply: bool,
    pub expand_details_by_default: bool,
    pub auto_git_sync: bool,
    pub reload_mango_on_rebuild: bool,
    pub reload_dms_on_rebuild: bool,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::List,
            live_apply: true,
            expand_details_by_default: false,
            auto_git_sync: false,
            reload_mango_on_rebuild: false,
            reload_dms_on_rebuild: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NixSettingsGuiConfig {
    #[serde(default)]
    pub terminal_command: Option<String>,
    #[serde(default)]
    pub rebuild_command: Option<String>,
}

impl NixSettingsGuiConfig {
    pub fn load() -> Option<Self> {
        let path = gui_config_path();
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
    }
}

/// Read the settings GUI's own preferences, keeping the defaults of fields the
/// file does not contain.
pub fn load_gui_settings() -> GuiSettings {
    gui_settings_from_value(&gui_document())
}

/// Write only the preferences this page owns. Every other key in the settings
/// GUI config stays untouched.
pub fn save_gui_settings(settings: &GuiSettings) -> Result<(), String> {
    let mut value = gui_document();
    apply_gui_settings(&mut value, settings);
    let path = gui_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create config dir: {error}"))?;
    }
    let json = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| format!("cannot write config: {error}"))
}

fn gui_config_path() -> PathBuf {
    config_base().join("nix-settings-gui").join("config.json")
}

fn gui_document() -> Value {
    std::fs::read_to_string(gui_config_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

fn gui_settings_from_value(value: &Value) -> GuiSettings {
    let flag = |key: &str, default: bool| {
        value
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    };
    GuiSettings {
        view_mode: value
            .get("view_mode")
            .and_then(Value::as_str)
            .and_then(ViewMode::parse)
            .unwrap_or_default(),
        live_apply: flag("live_apply", true),
        expand_details_by_default: flag("expand_details_by_default", false),
        auto_git_sync: flag("auto_git_sync", false),
        reload_mango_on_rebuild: flag("reload_mango_on_rebuild", false),
        reload_dms_on_rebuild: flag("reload_dms_on_rebuild", false),
    }
}

fn apply_gui_settings(value: &mut Value, settings: &GuiSettings) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    object.insert(
        "view_mode".to_string(),
        Value::String(settings.view_mode.as_str().to_string()),
    );
    object.insert("live_apply".to_string(), Value::Bool(settings.live_apply));
    object.insert(
        "expand_details_by_default".to_string(),
        Value::Bool(settings.expand_details_by_default),
    );
    object.insert(
        "auto_git_sync".to_string(),
        Value::Bool(settings.auto_git_sync),
    );
    object.insert(
        "reload_mango_on_rebuild".to_string(),
        Value::Bool(settings.reload_mango_on_rebuild),
    );
    object.insert(
        "reload_dms_on_rebuild".to_string(),
        Value::Bool(settings.reload_dms_on_rebuild),
    );
}

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
        if let Some(path) = std::env::var_os("OJUNY_WELCOME_CONFIG") {
            return PathBuf::from(path);
        }
        config_base().join("ojuny-welcome").join("config.json")
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

fn config_base() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gui_settings_default_when_the_file_has_no_keys() {
        let settings = gui_settings_from_value(&json!({}));
        assert_eq!(settings.view_mode, ViewMode::List);
        assert!(settings.live_apply);
        assert!(!settings.expand_details_by_default);
        assert!(!settings.auto_git_sync);
        assert!(!settings.reload_mango_on_rebuild);
        assert!(!settings.reload_dms_on_rebuild);
    }

    #[test]
    fn gui_settings_round_trip_and_preserve_other_keys() {
        let mut value = json!({"flake_path": "/repo", "live_apply": false});
        let settings = GuiSettings {
            view_mode: ViewMode::Categories,
            live_apply: true,
            expand_details_by_default: true,
            auto_git_sync: true,
            reload_mango_on_rebuild: true,
            reload_dms_on_rebuild: true,
        };
        apply_gui_settings(&mut value, &settings);
        assert_eq!(value["flake_path"], "/repo");
        let back = gui_settings_from_value(&value);
        assert_eq!(back.view_mode, ViewMode::Categories);
        assert!(back.live_apply);
        assert!(back.expand_details_by_default);
        assert!(back.auto_git_sync);
        assert!(back.reload_mango_on_rebuild);
        assert!(back.reload_dms_on_rebuild);
    }

    #[test]
    fn unknown_style_falls_back_to_list() {
        let settings = gui_settings_from_value(&json!({"view_mode": "grid"}));
        assert_eq!(settings.view_mode, ViewMode::List);
    }

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

