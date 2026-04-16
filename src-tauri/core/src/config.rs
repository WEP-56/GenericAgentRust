use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    #[serde(default)]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default)]
    pub is_native_anthropic: bool,
    #[serde(default)]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub providers: Vec<LlmProvider>,
    pub active_provider_id: String,
    pub workspace_dir: String,
    pub memory_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            providers: vec![LlmProvider {
                id: "default_oai".to_string(),
                name: "OpenAI Proxy".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "".to_string(),
                default_model: "gpt-4o".to_string(),
                max_tokens: 8192,
                temperature: 1.0,
                is_native_anthropic: false,
                max_retries: 3,
            }],
            active_provider_id: "default_oai".to_string(),
            workspace_dir: ".".to_string(),
            memory_dir: "./memory".to_string(),
        }
    }
}

pub struct ConfigState(pub Mutex<AppConfig>);

pub fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("GenericAgent");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("settings.json");
    path
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = serde_json::from_str(&content) {
            return config;
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}
