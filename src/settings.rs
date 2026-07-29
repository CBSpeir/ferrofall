use eframe::egui;

use crate::audio::{DEFAULT_EFFECTS_VOLUME, DEFAULT_MUSIC_VOLUME};
use crate::theme;

const EFFECTS_VOLUME_KEY: &str = "oxidefall.audio-volume.v1";
const MUSIC_VOLUME_KEY: &str = "oxidefall.music-volume.v1";
const MUTED_KEY: &str = "oxidefall.audio-muted.v1";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings {
    pub(crate) effects_volume: f32,
    pub(crate) music_volume: f32,
    pub(crate) muted: bool,
    pub(crate) theme_preference: egui::ThemePreference,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            effects_volume: DEFAULT_EFFECTS_VOLUME,
            music_volume: DEFAULT_MUSIC_VOLUME,
            muted: false,
            theme_preference: egui::ThemePreference::System,
        }
    }
}

impl Settings {
    pub(crate) fn load() -> Self {
        #[cfg(test)]
        return Self::default();

        #[cfg(not(test))]
        imp::load().normalized()
    }

    pub(crate) fn save(self) {
        #[cfg(not(test))]
        imp::save(self.normalized());
    }

    fn normalized(self) -> Self {
        let defaults = Self::default();
        Self {
            effects_volume: normalize_volume(self.effects_volume, defaults.effects_volume),
            music_volume: normalize_volume(self.music_volume, defaults.music_volume),
            ..self
        }
    }
}

fn normalize_volume(volume: f32, default: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        default
    }
}

fn parse_settings<'a>(mut value: impl FnMut(&str) -> Option<&'a str>) -> Settings {
    let defaults = Settings::default();
    Settings {
        effects_volume: value(EFFECTS_VOLUME_KEY)
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.effects_volume),
        music_volume: value(MUSIC_VOLUME_KEY)
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.music_volume),
        muted: value(MUTED_KEY).is_some_and(|value| value == "true"),
        theme_preference: value(theme::PREFERENCE_KEY)
            .and_then(theme::parse_preference)
            .unwrap_or(defaults.theme_preference),
    }
    .normalized()
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_native_settings(settings: Settings) -> String {
    format!(
        "version=1\n{EFFECTS_VOLUME_KEY}={}\n{MUSIC_VOLUME_KEY}={}\n{MUTED_KEY}={}\n{}={}\n",
        settings.effects_volume,
        settings.music_volume,
        settings.muted,
        theme::PREFERENCE_KEY,
        theme::preference_value(settings.theme_preference),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_native_settings(contents: &str) -> Settings {
    let values = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect::<std::collections::HashMap<_, _>>();
    parse_settings(|key| values.get(key).copied())
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_legacy_settings(contents: &str) -> Settings {
    let values = [
        EFFECTS_VOLUME_KEY,
        MUSIC_VOLUME_KEY,
        MUTED_KEY,
        theme::PREFERENCE_KEY,
    ]
    .into_iter()
    .filter_map(|key| legacy_value(contents, key).map(|value| (key, value)))
    .collect::<std::collections::HashMap<_, _>>();
    parse_settings(|key| values.get(key).copied())
}

#[cfg(not(target_arch = "wasm32"))]
fn legacy_value<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    let key_start = contents.find(&format!("\"{key}\""))?;
    let after_key = &contents[key_start + key.len() + 2..];
    let value_start = after_key.find('"')? + 1;
    let value = &after_key[value_start..];
    Some(&value[..value.find('"')?])
}

#[cfg(all(not(test), target_arch = "wasm32"))]
mod imp {
    use std::collections::HashMap;

    use super::*;

    pub(super) fn load() -> Settings {
        let storage = web_sys::window().and_then(|window| window.local_storage().ok().flatten());
        let values = [
            EFFECTS_VOLUME_KEY,
            MUSIC_VOLUME_KEY,
            MUTED_KEY,
            theme::PREFERENCE_KEY,
        ]
        .into_iter()
        .filter_map(|key| {
            storage
                .as_ref()
                .and_then(|storage| storage.get_item(key).ok().flatten())
                .map(|value| (key, value))
        })
        .collect::<HashMap<_, _>>();
        parse_settings(|key| values.get(key).map(String::as_str))
    }

    pub(super) fn save(settings: Settings) {
        let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        else {
            return;
        };
        let values = [
            (EFFECTS_VOLUME_KEY, settings.effects_volume.to_string()),
            (MUSIC_VOLUME_KEY, settings.music_volume.to_string()),
            (MUTED_KEY, settings.muted.to_string()),
            (
                theme::PREFERENCE_KEY,
                theme::preference_value(settings.theme_preference).to_owned(),
            ),
        ];
        for (key, value) in values {
            let _ = storage.set_item(key, &value);
        }
    }
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
mod imp {
    use std::path::PathBuf;

    use super::*;

    const SETTINGS_FILE: &str = "settings-v1.txt";
    const LEGACY_FILE: &str = "app.ron";

    pub(super) fn load() -> Settings {
        let Some(directory) = settings_directory() else {
            return Settings::default();
        };
        let settings_path = directory.join(SETTINGS_FILE);
        if let Ok(contents) = std::fs::read_to_string(&settings_path) {
            return decode_native_settings(&contents);
        }

        let legacy_path = directory.join(LEGACY_FILE);
        let Ok(contents) = std::fs::read_to_string(legacy_path) else {
            return Settings::default();
        };
        let settings = decode_legacy_settings(&contents);
        save(settings);
        settings
    }

    pub(super) fn save(settings: Settings) {
        let Some(directory) = settings_directory() else {
            return;
        };
        if std::fs::create_dir_all(&directory).is_ok() {
            let _ = std::fs::write(
                directory.join(SETTINGS_FILE),
                encode_native_settings(settings),
            );
        }
    }

    fn settings_directory() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        return environment_path("HOME").map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Oxidefall")
        });

        #[cfg(target_os = "windows")]
        return environment_path("APPDATA").map(|path| path.join("Oxidefall").join("data"));

        #[cfg(target_os = "linux")]
        return environment_path("XDG_DATA_HOME")
            .filter(|path| path.is_absolute())
            .or_else(|| environment_path("HOME").map(|home| home.join(".local").join("share")))
            .map(|path| path.join("oxidefall"));

        #[allow(unreachable_code)]
        None
    }

    fn environment_path(name: &str) -> Option<PathBuf> {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_normalized() {
        let settings = parse_settings(|key| match key {
            EFFECTS_VOLUME_KEY => Some("1.7"),
            MUSIC_VOLUME_KEY => Some("not-a-number"),
            MUTED_KEY => Some("true"),
            theme::PREFERENCE_KEY => Some("light"),
            _ => None,
        });
        assert_eq!(settings.effects_volume, 1.0);
        assert_eq!(settings.music_volume, DEFAULT_MUSIC_VOLUME);
        assert!(settings.muted);
        assert_eq!(settings.theme_preference, egui::ThemePreference::Light);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_settings_round_trip() {
        let settings = Settings {
            effects_volume: 0.25,
            music_volume: 0.75,
            muted: true,
            theme_preference: egui::ThemePreference::Dark,
        };
        assert_eq!(
            decode_native_settings(&encode_native_settings(settings)),
            settings
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn migrates_legacy_eframe_values() {
        let legacy = r#"({"oxidefall.audio-volume.v1":"0.4","oxidefall.music-volume.v1":"0.2","oxidefall.audio-muted.v1":"true","oxidefall.theme.v1":"light"})"#;
        let settings = decode_legacy_settings(legacy);
        assert_eq!(settings.effects_volume, 0.4);
        assert_eq!(settings.music_volume, 0.2);
        assert!(settings.muted);
        assert_eq!(settings.theme_preference, egui::ThemePreference::Light);
    }
}
