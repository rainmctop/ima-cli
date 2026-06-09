//! Configuration management for IMA CLI
//! 
//! Credentials are loaded from (in priority order):
//! 1. Command-line options (passed explicitly)
//! 2. Environment variables (IMA_CLIENT_ID, IMA_API_KEY)
//! 3. Config file (~/.config/ima/config.toml or legacy ~/.config/ima/client_id, ~/.config/ima/api_key)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;

use crate::error::{self, Result, Error};

/// Default config directory path
const CONFIG_DIR: &str = ".config/ima";

/// Default config file name
const CONFIG_FILE: &str = "config.toml";

/// Legacy client ID file name
const LEGACY_CLIENT_ID_FILE: &str = "client_id";

/// Legacy API key file name
const LEGACY_API_KEY_FILE: &str = "api_key";

/// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Client ID for IMA API authentication
    pub client_id: Option<String>,

    /// API Key for IMA API authentication
    pub api_key: Option<String>,

    /// Base URL for IMA API
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// Skill version (from meta.json or default)
    #[serde(default = "default_skill_version")]
    pub skill_version: String,

    /// Force update check
    #[serde(default)]
    pub force_update_check: bool,

    /// Last check file path (for update checking)
    #[serde(skip)]
    pub last_check_file: Option<PathBuf>,

    /// Config file path
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

fn default_base_url() -> String {
    "https://ima.qq.com".to_string()
}

fn default_skill_version() -> String {
    // Try to read from meta.json first
    if let Some(meta_path) = find_meta_json() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(version) = meta.get("version").and_then(|v| v.as_str()) {
                    return version.to_string();
                }
            }
        }
    }
    "1.1.7".to_string()
}

fn find_meta_json() -> Option<PathBuf> {
    // Try current directory first
    let current_meta = PathBuf::from("meta.json");
    if current_meta.exists() {
        return Some(current_meta);
    }

    // Try executable directory
    if let Ok(exe_path) = std::env::current_exe() {
        let mut meta_path = exe_path;
        meta_path.pop(); // Remove executable name
        meta_path.push("meta.json");
        if meta_path.exists() {
            return Some(meta_path);
        }
    }

    None
}

impl Config {
    /// Load configuration from file, environment, or defaults
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self> {
        let mut config = Self::load_from_file(config_path)?;

        // Override with environment variables if present
        if let Ok(env_client_id) = std::env::var("IMA_CLIENT_ID") {
            if !env_client_id.is_empty() {
                config.client_id = Some(env_client_id);
            }
        }
        if let Ok(env_api_key) = std::env::var("IMA_API_KEY") {
            if !env_api_key.is_empty() {
                config.api_key = Some(env_api_key);
            }
        }
        // Also check alternative env var names (legacy compatibility)
        if config.client_id.is_none() {
            if let Ok(env_client_id) = std::env::var("IMA_OPENAPI_CLIENTID") {
                if !env_client_id.is_empty() {
                    config.client_id = Some(env_client_id);
                }
            }
        }
        if config.api_key.is_none() {
            if let Ok(env_api_key) = std::env::var("IMA_OPENAPI_APIKEY") {
                if !env_api_key.is_empty() {
                    config.api_key = Some(env_api_key);
                }
            }
        }

        // Set up last check file path
        config.last_check_file = Some(get_default_last_check_file());

        Ok(config)
    }

    /// Load configuration from TOML file or legacy files
    fn load_from_file(config_path: Option<&PathBuf>) -> Result<Self> {
        let config_path = config_path.map(|p| p.clone()).unwrap_or_else(get_default_config_file);

        // Try to load from main config file first
        if config_path.exists() {
            return Self::load_toml_config(&config_path);
        }

        // Fall back to legacy file format
        Self::load_legacy_config()
    }

    /// Load from TOML config file
    fn load_toml_config(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| error::invalid_config(format!("Failed to read config file {}: {}", path.display(), e)))?;

        let mut config: Config = toml::from_str(&content)
            .map_err(|e| error::invalid_config(format!("Failed to parse config file: {}", e)))?;

        config.config_path = Some(path.to_path_buf());
        Ok(config)
    }

    /// Load from legacy separate files
    fn load_legacy_config() -> Result<Self> {
        let config_dir = get_config_dir();
        let mut config = Config {
            client_id: None,
            api_key: None,
            base_url: default_base_url(),
            skill_version: default_skill_version(),
            force_update_check: false,
            last_check_file: None,
            config_path: None,
        };

        // Try to read legacy client_id file
        let client_id_path = config_dir.join(LEGACY_CLIENT_ID_FILE);
        if client_id_path.exists() {
            if let Ok(content) = fs::read_to_string(&client_id_path) {
                let trimmed = content.trim().to_string();
                if !trimmed.is_empty() {
                    config.client_id = Some(trimmed);
                }
            }
        }

        // Try to read legacy api_key file
        let api_key_path = config_dir.join(LEGACY_API_KEY_FILE);
        if api_key_path.exists() {
            if let Ok(content) = fs::read_to_string(&api_key_path) {
                let trimmed = content.trim().to_string();
                if !trimmed.is_empty() {
                    config.api_key = Some(trimmed);
                }
            }
        }

        config.config_path = Some(config_dir.join(CONFIG_FILE));
        Ok(config)
    }

    /// Validate that credentials are present
    pub fn validate_credentials(&self) -> Result<()> {
        match (&self.client_id, &self.api_key) {
            (Some(client_id), Some(api_key)) if !client_id.is_empty() && !api_key.is_empty() => Ok(()),
            _ => Err(error::missing_credentials(
                "请设置 IMA_CLIENT_ID 和 IMA_API_KEY 环境变量，或将凭证放置在 ~/.config/ima/ 目录下。"
            )),
        }
    }

    /// Get the client ID
    pub fn client_id(&self) -> Result<&str> {
        Ok(self.client_id
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .ok_or_else(|| error::missing_credentials("Client ID is missing"))?)
    }

    /// Get the API key
    pub fn api_key(&self) -> Result<&str> {
        Ok(self.api_key
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .ok_or_else(|| error::missing_credentials("API Key is missing"))?)
    }
}

/// Get the default config directory path
fn get_config_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(CONFIG_DIR)
    } else {
        // Fallback to current directory
        PathBuf::from(CONFIG_DIR)
    }
}

/// Get the default config file path
fn get_default_config_file() -> PathBuf {
    get_config_dir().join(CONFIG_FILE)
}

/// Get the default last check file path
fn get_default_last_check_file() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(CONFIG_DIR).join("last_update_check")
    } else {
        PathBuf::from(CONFIG_DIR).join("last_update_check")
    }
}

/// Read a file safely, returning empty string on error
pub fn read_file_safe<P: AsRef<Path>>(path: P) -> String {
    fs::read_to_string(path.as_ref())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Ensure directory exists
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    Ok(fs::create_dir_all(path.as_ref())
        .map_err(|e| error::invalid_config(format!("Failed to create directory {}: {}", path.as_ref().display(), e)))?)
}

/// Save today's date to the last check file
pub fn save_today_checked(last_check_file: &Path) -> Result<()> {
    ensure_dir(last_check_file.parent().unwrap())?;
    
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    Ok(fs::write(last_check_file, &today)
        .map_err(|e| error::io_error(e))?)
}

/// Read the last checked date
pub fn read_last_checked(last_check_file: &Path) -> String {
    read_file_safe(last_check_file)
}

/// Check if update should be checked
pub fn should_check_update(last_check_file: &Path, force: bool) -> bool {
    if force {
        return true;
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let last_checked = read_last_checked(last_check_file);

    last_checked != today
}

// Optional TOML support - if not available, use JSON config instead
// For now, we'll use a simple JSON-based config as fallback
impl Config {
    /// Load from JSON config file (alternative to TOML)
    pub fn load_from_json(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| error::invalid_config(format!("Failed to read config file {}: {}", path.display(), e)))?;

        let mut config: Config = serde_json::from_str(&content)
            .map_err(|e| error::invalid_config(format!("Failed to parse config file: {}", e)))?;

        config.config_path = Some(path.to_path_buf());
        Ok(config)
    }
}

// Re-export with conditional compilation for TOML
#[cfg(feature = "toml")]
mod toml_support {
    use super::*;

    impl Config {
        pub fn load_toml(path: &Path) -> Result<Self> {
            Self::load_toml_config(path)
        }
    }
}
