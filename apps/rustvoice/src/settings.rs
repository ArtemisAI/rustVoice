use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub typing_speed_cpm: usize,
    pub dark_mode: bool,
    pub model_size: String, // "tiny_en", "base_en", "small_en", "tiny", "base", "small"
    pub opacity: f32,
    // Transcription options
    pub task: String,       // "transcribe" or "translate"
    pub timestamps: bool,
    pub verbose: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            typing_speed_cpm: 1200,
            dark_mode: true,
            model_size: "base_en".to_string(),
            opacity: 0.95,
            task: "transcribe".to_string(),
            timestamps: true,
            verbose: false,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        if let Some(path) = Self::get_config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, content);
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "AutoTyper", "AutoTyperV6")
            .map(|proj_dirs| proj_dirs.config_dir().join("settings.json"))
    }
}
