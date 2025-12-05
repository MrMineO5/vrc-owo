use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const CONFIG_FILE: &str = "muscle_config.json";

#[derive(Serialize, Deserialize)]
pub struct MuscleConfig {
    pub name: String,
    pub muscle: String,
    pub intensity_touch: u8,
    pub intensity_impact: u8,
    pub intensity_stab: u8,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub muscles: Vec<MuscleConfig>,
    pub ip_address: Option<String>,
}

pub fn load_config() -> Option<Config> {
    let config_path = get_config_path();
    if !config_path.exists() {
        return None;
    }

    match fs::read_to_string(config_path) {
        Ok(contents) => serde_json::from_str(&contents).ok(),
        Err(e) => {
            println!("Error reading config file: {}", e);
            None
        }
    }
}

pub fn save_config(config: &Config) -> std::io::Result<()> {
    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(config)?;
    fs::write(config_path, json)
}

pub fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("vrc-owo");
    fs::create_dir_all(&path).ok();
    path.push(CONFIG_FILE);
    path
}
