//! Command handlers for IMA CLI

pub mod kb;
pub mod notes;

use crate::config::Config;
use crate::error::{self, Result};
use crate::api::ApiClient;
use serde_json::Value;

/// Handle legacy API command (compatible with ima_api.cjs)
pub async fn legacy_api(api_path: &str, body: &str, options: &str, config: &Config) -> Result<String> {
    // Parse body JSON
    let body_value: Value = serde_json::from_str(body)
        .map_err(|e| error::invalid_argument(format!("请求 body 不是合法的 JSON: {}", e)))?;

    // Parse options JSON
    let mut options_value: Value = serde_json::from_str(options)
        .map_err(|e| error::invalid_argument(format!("options 参数不是合法的 JSON: {}", e)))?;

    // Merge options with config
    if let Value::Object(ref mut opts) = options_value {
        if !opts.contains_key("clientId") {
            if let Some(client_id) = &config.client_id {
                opts.insert("clientId".to_string(), Value::String(client_id.clone()));
            }
        }
        if !opts.contains_key("apiKey") {
            if let Some(api_key) = &config.api_key {
                opts.insert("apiKey".to_string(), Value::String(api_key.clone()));
            }
        }
    }

    // Create API client with merged config
    let api_config = Config {
        client_id: options_value.get("clientId").and_then(|v| v.as_str()).map(|s| s.to_string())
            .or_else(|| config.client_id.clone()),
        api_key: options_value.get("apiKey").and_then(|v| v.as_str()).map(|s| s.to_string())
            .or_else(|| config.api_key.clone()),
        base_url: options_value.get("baseUrl").and_then(|v| v.as_str()).map(|s| s.to_string())
            .unwrap_or_else(|| config.base_url.clone()),
        skill_version: config.skill_version.clone(),
        force_update_check: options_value.get("forceCheck").and_then(|v| v.as_bool()).unwrap_or(config.force_update_check),
        last_check_file: config.last_check_file.clone(),
        config_path: config.config_path.clone(),
    };

    // Check for update if not the check_skill_update endpoint
    if api_path != "openapi/check_skill_update" && api_config.force_update_check {
        let _ = check_update(&api_config).await;
    }

    // Make API call
    let client = ApiClient::new(&api_config)?;
    let response = client.post_raw(api_path, &body_value).await?;

    Ok(response)
}

/// Check for skill updates
pub async fn check_update(config: &Config) -> Result<String> {
    use std::fs;
    use chrono::Local;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let last_check_file = config.last_check_file.as_ref()
        .ok_or_else(|| error::invalid_config("Last check file path not set"))?;

    // Check if we already checked today
    let last_checked = fs::read_to_string(last_check_file)
        .unwrap_or_default();

    if last_checked.trim() == today && !config.force_update_check {
        return Ok(String::new());
    }

    // Call check_skill_update API
    let client = ApiClient::new(config)?;
    
    #[derive(serde::Serialize)]
    struct Params<'a> {
        version: &'a str,
    }

    let response = match client.post_raw(
        "openapi/check_skill_update",
        &Params { version: &config.skill_version },
    ).await {
        Ok(resp) => resp,
        Err(_) => {
            // Skip update check on network errors
            return Ok(String::new());
        }
    };

    // Save today's date
    if let Some(parent) = last_check_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(last_check_file, &today);

    // Parse response to check for updates
    let value: Value = serde_json::from_str(&response)
        .unwrap_or(Value::Null);

    if let Some(data) = value.get("data") {
        if let Some(latest_version) = data.get("latest_version").and_then(|v| v.as_str()) {
            if latest_version != config.skill_version {
                let release_desc = data.get("release_desc").and_then(|v| v.as_str()).unwrap_or("");
                let instruction = data.get("instruction").and_then(|v| v.as_str()).unwrap_or("请更新。");

                let _update_context = serde_json::json!({
                    "current_version": config.skill_version,
                    "latest_version": latest_version,
                    "release_desc": release_desc,
                    "instruction": instruction,
                    "checked_at": Local::now().to_rfc3339(),
                });

                let err_msg = format!(
                    "发现新版本 skill：{}（当前版本：{}）。{}",
                    latest_version, config.skill_version, instruction
                );

                return Err(error::update_available(err_msg).into());
            }
        }
    }

    Ok(String::new())
}
