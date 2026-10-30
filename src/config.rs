use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::engine::TypingConfig;

fn config_path() -> Result<PathBuf, String> {
    let base = match env::var_os("XDG_CONFIG_HOME") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("XDG_CONFIG_HOME must be an absolute path".into());
            }
            path
        }
        _ => {
            let home = env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| "HOME is not set and XDG_CONFIG_HOME is unavailable".to_string())?;
            home.join(".config")
        }
    };

    Ok(base.join("ibus-bornona").join("config.json"))
}

pub fn load() -> Result<TypingConfig, String> {
    let path = config_path()?;
    match fs::read(&path) {
        Ok(contents) => serde_json::from_slice(&contents)
            .map_err(|error| format!("could not parse {}: {error}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(TypingConfig::default()),
        Err(error) => Err(format!("could not read {}: {error}", path.display())),
    }
}

pub fn save(config: &TypingConfig) -> Result<(), String> {
    let path = config_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "could not determine config directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;

    let temporary_path = path.with_extension(format!("json.tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("could not create {}: {error}", temporary_path.display()))?;
        serde_json::to_writer_pretty(&mut file, config)
            .map_err(|error| format!("could not serialize settings: {error}"))?;
        file.write_all(b"\n")
            .map_err(|error| format!("could not write settings: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("could not sync settings: {error}"))?;
        drop(file);
        fs::rename(&temporary_path, &path)
            .map_err(|error| format!("could not save {}: {error}", path.display()))
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    modified: SystemTime,
    length: u64,
}

fn file_stamp(path: &PathBuf) -> Result<Option<FileStamp>, String> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let modified = metadata
                .modified()
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            Ok(Some(FileStamp {
                modified,
                length: metadata.len(),
            }))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

pub struct ConfigWatcher {
    path: PathBuf,
    stamp: Option<FileStamp>,
    config: TypingConfig,
}

impl ConfigWatcher {
    pub fn new() -> Result<Self, String> {
        let path = config_path()?;
        let stamp = file_stamp(&path)?;
        let config = load()?;
        Ok(Self {
            path,
            stamp,
            config,
        })
    }

    pub fn config(&self) -> &TypingConfig {
        &self.config
    }

    pub fn refresh(&mut self) -> Result<Option<TypingConfig>, String> {
        let stamp = file_stamp(&self.path)?;
        if stamp == self.stamp {
            return Ok(None);
        }
        self.stamp = stamp;

        let config = match fs::read(&self.path) {
            Ok(contents) => serde_json::from_slice(&contents)
                .map_err(|error| format!("could not parse {}: {error}", self.path.display()))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => TypingConfig::default(),
            Err(error) => {
                return Err(format!(
                    "could not read {}: {error}",
                    self.path.display()
                ));
            }
        };

        self.config = config.clone();
        Ok(Some(config))
    }
}

pub fn feature_name(feature: &str) -> Option<&'static str> {
    match feature {
        "auto-vowel" => Some("Automatic Vowel Forming"),
        "auto-chandra" => Some("Automatic Chandrabindu Position Fix"),
        "old-reph" => Some("Old Style Reph"),
        "traditional-kar" => Some("Traditional Kar Joining"),
        "word-suggestion" | "suggestion" | "suggestions" => Some("Word Suggestions"),
        "suggestion-trailing-space" | "trailing-space" => Some("Space After Suggestion"),
        "suggestion-orientation" | "orientation" => Some("Suggestion Orientation"),
        "keyboard-switch-shortcut" => Some("Keyboard Switching Shortcut"),
        "learning" => Some("Suggestion Learning"),
        _ => None,
    }
}

pub fn set_feature(config: &mut TypingConfig, feature: &str, enabled: bool) -> bool {
    match feature {
        "auto-vowel" => config.auto_vowel = enabled,
        "auto-chandra" => config.auto_chandra = enabled,
        "old-reph" => config.old_reph = enabled,
        "traditional-kar" => config.traditional_kar = enabled,
        "word-suggestion" | "suggestion" | "suggestions" => config.word_suggestion = enabled,
        "suggestion-trailing-space" | "trailing-space" => {
            config.suggestion_trailing_space = enabled
        }
        "learning" => config.learning = enabled,
        _ => return false,
    }
    true
}

pub fn set_orientation(config: &mut TypingConfig, orientation: crate::engine::SuggestionOrientation) {
    config.suggestion_orientation = orientation;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SuggestionOrientation;

    #[test]
    fn older_settings_default_trailing_space_to_off() {
        let config: TypingConfig = serde_json::from_str(r#"{"auto_vowel":false}"#).unwrap();

        assert!(!config.auto_vowel);
        assert!(!config.suggestion_trailing_space);
    }

    #[test]
    fn feature_updates_change_only_the_requested_setting() {
        let mut config = TypingConfig::default();

        assert!(!config.word_suggestion);
        assert!(config.learning);
        assert_eq!(config.keyboard_switch_shortcut, "Alt+Space");

        assert!(set_feature(&mut config, "auto-vowel", false));
        assert!(!config.auto_vowel);
        assert!(config.auto_chandra);
        assert!(config.old_reph);
        assert!(!config.traditional_kar);
        assert!(!config.word_suggestion);
        assert!(!config.suggestion_trailing_space);
        assert_eq!(config.suggestion_orientation, SuggestionOrientation::Horizontal);

        assert!(set_feature(&mut config, "suggestion", false));
        assert!(!config.word_suggestion);
        assert!(set_feature(&mut config, "trailing-space", false));
        assert!(!config.suggestion_trailing_space);
        assert!(set_feature(&mut config, "learning", false));
        assert!(!config.learning);

        set_orientation(&mut config, SuggestionOrientation::Vertical);
        assert_eq!(config.suggestion_orientation, SuggestionOrientation::Vertical);
        set_orientation(&mut config, SuggestionOrientation::System);
        assert_eq!(config.suggestion_orientation, SuggestionOrientation::System);

        assert!(!set_feature(&mut config, "unknown", true));
    }

}
