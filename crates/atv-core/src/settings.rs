//! User configuration and settings persistence.
//!
//! Handles local user preferences (mouse speed, trackpad mode, acceleration,
//! logging level) stored in `~/.config/atv/settings.json` outside the git repository,
//! ensuring preferences persist across runs and are never touched by git changes.

use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_MOUSE_SPEED: f64 = 0.5;
pub const MIN_MOUSE_SPEED: f64 = 0.1;
pub const MAX_MOUSE_SPEED: f64 = 10.0;

/// Persistent user settings.
#[derive(Debug, Clone, PartialEq)]
pub struct UserSettings {
    pub mouse_speed: f64,
    pub mouse_mode: Option<bool>,
    pub trackpad_mode: Option<String>,
    pub mouse_accel: Option<bool>,
    pub verbose_events: Option<bool>,
    /// Debug Web UI port override (default 38765).
    pub ui_port: Option<u16>,
    /// Swift host diagnostics port override (default 38766).
    pub web_port: Option<u16>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            mouse_speed: DEFAULT_MOUSE_SPEED,
            mouse_mode: None,
            trackpad_mode: None,
            mouse_accel: None,
            verbose_events: None,
            ui_port: None,
            web_port: None,
        }
    }
}

/// Resolve the path to the user settings file.
///
/// Priority:
/// 1. `.atv-settings.json` in the current working directory if it exists.
/// 2. `$XDG_CONFIG_HOME/atv/settings.json` if configured.
/// 3. `~/.config/atv/settings.json` (standard user config on macOS/Linux).
/// 4. Fallback to `atv-settings.json`.
pub fn get_settings_file_path() -> PathBuf {
    let local = PathBuf::from(".atv-settings.json");
    if local.exists() {
        return local;
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("atv").join("settings.json");
        }
    }

    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join(".config").join("atv").join("settings.json");
        }
    }

    PathBuf::from("atv-settings.json")
}

impl UserSettings {
    /// Load user settings from disk. If the file does not exist, initialize it
    /// with default settings (`mouse_speed: 0.5`) and persist it.
    pub fn load() -> Self {
        let path = get_settings_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                return Self::from_json(&content);
            }
        }

        let default_settings = Self::default();
        let _ = default_settings.save_to(&path);
        default_settings
    }

    /// Parse settings from JSON string.
    pub fn from_json(json: &str) -> Self {
        let mut settings = Self::default();

        if let Some(sp) = extract_float(json, "mouse_speed").or_else(|| extract_float(json, "speed")) {
            settings.mouse_speed = sp.clamp(MIN_MOUSE_SPEED, MAX_MOUSE_SPEED);
        }

        let mode_str = extract_string(json, "trackpad_mode").or_else(|| extract_string(json, "mode"));
        if let Some(ref m) = mode_str {
            settings.trackpad_mode = Some(m.clone());
            if m == "mouse" {
                settings.mouse_mode = Some(true);
            } else if m == "direction" || m == "idle" {
                settings.mouse_mode = Some(false);
            }
        }

        if let Some(mode) = extract_bool(json, "mouse_mode") {
            settings.mouse_mode = Some(mode);
            if settings.trackpad_mode.is_none() {
                settings.trackpad_mode = Some(if mode { "mouse".into() } else { "direction".into() });
            }
        }

        if let Some(accel) = extract_bool(json, "mouse_accel").or_else(|| extract_bool(json, "accel")) {
            settings.mouse_accel = Some(accel);
        }

        if let Some(verbose) = extract_bool(json, "verbose_events").or_else(|| extract_bool(json, "verbose")) {
            settings.verbose_events = Some(verbose);
        }

        if let Some(ui_port) = extract_u16(json, "ui_port") {
            settings.ui_port = Some(ui_port);
        }

        if let Some(web_port) = extract_u16(json, "web_port") {
            settings.web_port = Some(web_port);
        }

        settings
    }

    /// Serialize settings into clean JSON.
    pub fn to_json(&self) -> String {
        let mut fields = Vec::new();
        fields.push(format!("  \"mouse_speed\": {:.2}", self.mouse_speed));
        if let Some(ref mode) = self.trackpad_mode {
            fields.push(format!("  \"trackpad_mode\": \"{mode}\""));
        }
        if let Some(mode) = self.mouse_mode {
            fields.push(format!("  \"mouse_mode\": {mode}"));
        }
        if let Some(accel) = self.mouse_accel {
            fields.push(format!("  \"mouse_accel\": {accel}"));
        }
        if let Some(verbose) = self.verbose_events {
            fields.push(format!("  \"verbose_events\": {verbose}"));
        }
        if let Some(ui_port) = self.ui_port {
            fields.push(format!("  \"ui_port\": {ui_port}"));
        }
        if let Some(web_port) = self.web_port {
            fields.push(format!("  \"web_port\": {web_port}"));
        }

        format!("{{\n{}\n}}\n", fields.join(",\n"))
    }

    /// Save settings to default file location.
    pub fn save(&self) -> std::io::Result<()> {
        let path = get_settings_file_path();
        self.save_to(&path)
    }

    /// Save settings to a specific file path.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = self.to_json();
        fs::write(path, json)
    }

    /// Update mouse speed and persist to disk.
    pub fn update_speed(new_speed: f64) {
        let mut s = Self::load();
        s.mouse_speed = new_speed.clamp(MIN_MOUSE_SPEED, MAX_MOUSE_SPEED);
        let _ = s.save();
    }

    /// Update mouse mode and persist to disk.
    pub fn update_mouse_mode(new_mode: bool) {
        let mut s = Self::load();
        s.mouse_mode = Some(new_mode);
        s.trackpad_mode = Some(if new_mode { "mouse".into() } else { "direction".into() });
        let _ = s.save();
    }

    /// Update trackpad mode (mouse, direction, idle) and persist to disk.
    pub fn update_trackpad_mode(new_mode: &str) {
        let mut s = Self::load();
        s.trackpad_mode = Some(new_mode.to_string());
        s.mouse_mode = Some(new_mode == "mouse");
        let _ = s.save();
    }

    /// Update all touchpad settings and persist to disk.
    pub fn update_all(speed: f64, accel: bool, verbose: bool, mouse_mode: Option<bool>) {
        Self::update_all_with_mode(speed, accel, verbose, None, mouse_mode);
    }

    /// Update all touchpad settings including explicit trackpad mode string.
    pub fn update_all_with_mode(
        speed: f64,
        accel: bool,
        verbose: bool,
        trackpad_mode: Option<&str>,
        mouse_mode: Option<bool>,
    ) {
        let mut s = Self::load();
        s.mouse_speed = speed.clamp(MIN_MOUSE_SPEED, MAX_MOUSE_SPEED);
        s.mouse_accel = Some(accel);
        s.verbose_events = Some(verbose);
        if let Some(tm) = trackpad_mode {
            s.trackpad_mode = Some(tm.to_string());
            s.mouse_mode = Some(tm == "mouse");
        } else if let Some(mode) = mouse_mode {
            s.mouse_mode = Some(mode);
            s.trackpad_mode = Some(if mode { "mouse".into() } else { "direction".into() });
        }
        let _ = s.save();
    }
}

fn extract_float(json: &str, key: &str) -> Option<f64> {
    let quoted = format!("\"{key}\"");
    let idx = json.find(&quoted)?;
    let rest = &json[idx + quoted.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let num_str: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    num_str.parse().ok()
}

/// Parse an integer port value; accepts either a JSON number (`38765`) or a
/// quoted string (`"38765"`). Values outside 1..=65535 are ignored.
fn extract_u16(json: &str, key: &str) -> Option<u16> {
    let value = extract_float(json, key)
        .or_else(|| extract_string(json, key).and_then(|s| s.parse::<f64>().ok()))?;
    if value.fract() != 0.0 || value < 1.0 || value > u16::MAX as f64 {
        return None;
    }
    Some(value as u16)
}

fn extract_bool(json: &str, key: &str) -> Option<bool> {
    let quoted = format!("\"{key}\"");
    let idx = json.find(&quoted)?;
    let rest = &json[idx + quoted.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_string(json: &str, key: &str) -> Option<String> {
    let quoted = format!("\"{key}\"");
    let idx = json.find(&quoted)?;
    let rest = &json[idx + quoted.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let quote_end = rest.find('"')?;
    Some(rest[..quote_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mouse_speed_is_half() {
        let s = UserSettings::default();
        assert_eq!(s.mouse_speed, 0.5);
    }

    #[test]
    fn test_parse_json_custom_speed() {
        let json = r#"{ "mouse_speed": 0.75, "mouse_accel": false, "verbose_events": true }"#;
        let s = UserSettings::from_json(json);
        assert!((s.mouse_speed - 0.75).abs() < 1e-6);
        assert_eq!(s.mouse_accel, Some(false));
        assert_eq!(s.verbose_events, Some(true));
    }

    #[test]
    fn test_parse_json_clamp_speed() {
        let json_low = r#"{ "mouse_speed": 0.01 }"#;
        let s_low = UserSettings::from_json(json_low);
        assert_eq!(s_low.mouse_speed, MIN_MOUSE_SPEED);

        let json_high = r#"{ "mouse_speed": 50.0 }"#;
        let s_high = UserSettings::from_json(json_high);
        assert_eq!(s_high.mouse_speed, MAX_MOUSE_SPEED);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp_dir = std::env::temp_dir().join(format!("atv_test_{}", std::process::id()));
        let tmp_file = tmp_dir.join("test_settings.json");

        let settings = UserSettings {
            mouse_speed: 0.65,
            mouse_mode: Some(true),
            trackpad_mode: Some("mouse".into()),
            mouse_accel: Some(false),
            verbose_events: Some(true),
            ui_port: Some(38765),
            web_port: Some(38766),
        };

        settings.save_to(&tmp_file).expect("save should succeed");
        let content = fs::read_to_string(&tmp_file).expect("read should succeed");
        let loaded = UserSettings::from_json(&content);

        assert!((loaded.mouse_speed - 0.65).abs() < 1e-6);
        assert_eq!(loaded.mouse_mode, Some(true));
        assert_eq!(loaded.trackpad_mode, Some("mouse".into()));
        assert_eq!(loaded.mouse_accel, Some(false));
        assert_eq!(loaded.verbose_events, Some(true));
        assert_eq!(loaded.ui_port, Some(38765));
        assert_eq!(loaded.web_port, Some(38766));

        let _ = fs::remove_file(&tmp_file);
        let _ = fs::remove_dir(&tmp_dir);
    }

    #[test]
    fn test_parse_port_overrides() {
        let json = r#"{ "mouse_speed": 0.5, "ui_port": 38765, "web_port": "38766" }"#;
        let s = UserSettings::from_json(json);
        assert_eq!(s.ui_port, Some(38765));
        assert_eq!(s.web_port, Some(38766));
    }

    #[test]
    fn test_invalid_ports_are_ignored() {
        for json in [
            r#"{ "ui_port": 0 }"#,
            r#"{ "ui_port": 70000 }"#,
            r#"{ "web_port": -1 }"#,
            r#"{ "web_port": 1.5 }"#,
            r#"{ "web_port": "abc" }"#,
        ] {
            let s = UserSettings::from_json(json);
            assert_eq!(s.ui_port, None, "{json}");
            assert_eq!(s.web_port, None, "{json}");
        }
    }

    #[test]
    fn test_parse_json_idle_mode() {
        let json = r#"{ "trackpad_mode": "idle", "mouse_speed": 1.2 }"#;
        let s = UserSettings::from_json(json);
        assert_eq!(s.trackpad_mode, Some("idle".into()));
        assert_eq!(s.mouse_mode, Some(false));
    }
}
