//! Knowledge base command handlers

use crate::api::{ApiClient, CreateMediaParams, AddKnowledgeParams, FileInfo, ImportUrlsParams, CheckRepeatedNamesParams, CheckRepeatedNameParam};
use crate::config::Config;
use crate::cos;
use crate::error::{self, Result};
use crate::commands::KbCommands;
use std::path::PathBuf;
use mime_guess::from_path;

/// Handle knowledge base commands
pub async fn handle_kb_command(action: KbCommands, config: &Config) -> Result<String> {
    let client = ApiClient::new(config)?;

    match action {
        KbCommands::Info { ids } => {
            let id_list: Vec<String> = ids.split(',').map(|s: &str| s.trim().to_string()).collect();
            let response = client.get_knowledge_base(&id_list).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::List { kb_id, folder_id, cursor, limit } => {
            let response = client.get_knowledge_list(&kb_id, folder_id.as_deref(), &cursor, limit).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::Search { kb_id, query, cursor } => {
            let response = client.search_knowledge(&kb_id, &query, &cursor).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::SearchKb { query, cursor, limit } => {
            let response = client.search_knowledge_base(&query, &cursor, limit).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::Addable { cursor, limit } => {
            let response = client.get_addable_knowledge_base_list(&cursor, limit).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::Upload { file, kb_id, folder_id, content_type, title } => {
            upload_file(&client, config, &file, &kb_id, folder_id.as_deref(), content_type.as_deref(), title.as_deref()).await
        }

        KbCommands::ImportUrls { kb_id, folder_id, urls } => {
            let params = ImportUrlsParams {
                knowledge_base_id: kb_id,
                folder_id,
                urls,
            };
            let response = client.import_urls(&params).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::CheckRepeated { kb_id, folder_id, files } => {
            let mut params_list = Vec::new();
            for file_spec in files {
                let parts: Vec<&str> = file_spec.split(':').collect();
                if parts.len() != 2 {
                    return Err(error::invalid_argument(format!(
                        "Invalid file spec '{}'. Expected format: name:media_type",
                        file_spec
                    )).into());
                }
                let media_type: i32 = parts[1].parse()
                    .map_err(|_| error::invalid_argument(format!("Invalid media type '{}'", parts[1])))?;
                params_list.push(CheckRepeatedNameParam {
                    name: parts[0].to_string(),
                    media_type,
                });
            }

            let params = CheckRepeatedNamesParams {
                params: params_list,
                knowledge_base_id: kb_id,
                folder_id,
            };
            let response = client.check_repeated_names(&params).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        KbCommands::MediaInfo { media_id } => {
            let response = client.get_media_info(&media_id).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }
    }
}

/// Upload a file to knowledge base
async fn upload_file(
    client: &ApiClient,
    config: &Config,
    file_path: &PathBuf,
    kb_id: &str,
    folder_id: Option<&str>,
    content_type: Option<&str>,
    title: Option<&str>,
) -> Result<String> {
    use std::fs;

    // Validate file exists
    if !file_path.exists() {
        return Err(error::file_validation_failed(format!("File not found: {}", file_path.display())).into());
    }

    // Read file content
    let file_content = fs::read(file_path)
        .map_err(|e| error::io_error(e))?;

    let file_size = file_content.len() as u64;
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| error::invalid_argument("Invalid file path"))?
        .to_string();

    // Determine content type
    let content_type = content_type.map(|s| s.to_string())
        .or_else(|| {
            from_path(file_path)
                .first()
                .map(|m| m.to_string())
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());

    // Determine file extension
    let file_ext = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Determine media type based on extension or content type
    let media_type = determine_media_type(&file_ext, &content_type)?;

    // Check file size limits
    check_file_size_limit(media_type, file_size)?;

    // Step 1: Create media to get COS credentials
    let create_params = CreateMediaParams {
        file_name: file_name.clone(),
        file_size,
        content_type: content_type.clone(),
        knowledge_base_id: kb_id.to_string(),
        file_ext: file_ext.clone(),
    };

    let create_response = client.create_media(&create_params).await?;
    let media_id = create_response.data.media_id;
    let credential = create_response.data.cos_credential;

    // Step 2: Upload file to COS (save to temp file first if needed for large files)
    // For now, we upload directly. For very large files, we could implement chunked upload.
    cos::upload_file(&credential, file_path, &file_content).await?;

    // Step 3: Add knowledge entry
    let add_params = AddKnowledgeParams {
        media_type,
        media_id: Some(media_id.clone()),
        title: title.unwrap_or(&file_name).to_string(),
        knowledge_base_id: kb_id.to_string(),
        folder_id: folder_id.map(|s| s.to_string()),
        note_info: None,
        web_info: None,
        session_info: None,
        file_info: Some(FileInfo {
            cos_key: credential.cos_key,
            file_size,
            last_modify_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            password: None,
            file_name: file_name.clone(),
        }),
    };

    let add_response = client.add_knowledge(&add_params).await?;

    let result = serde_json::json!({
        "success": true,
        "media_id": add_response.data.media_id,
        "file_name": file_name,
        "file_size": file_size,
    });
    Ok(serde_json::to_string_pretty(&result)?)
}

/// Determine media type from file extension and content type
fn determine_media_type(ext: &str, content_type: &str) -> Result<i32> {
    // Map based on extension first
    let media_type = match ext.to_lowercase().as_str() {
        "pdf" => 1,
        "doc" | "docx" => 3,
        "ppt" | "pptx" => 4,
        "xls" | "xlsx" | "csv" => 5,
        "md" | "markdown" => 7,
        "png" | "jpg" | "jpeg" | "webp" | "gif" => 9,
        "txt" => 13,
        "xmind" => 14,
        "mp3" | "m4a" | "wav" | "aac" => 15,
        _ => {
            // Fall back to content type
            match content_type {
                ct if ct.starts_with("application/pdf") => 1,
                ct if ct.contains("word") || ct.contains("document") => 3,
                ct if ct.contains("powerpoint") || ct.contains("presentation") => 4,
                ct if ct.contains("excel") || ct.contains("spreadsheet") || ct == "text/csv" => 5,
                ct if ct.contains("markdown") => 7,
                ct if ct.starts_with("image/") => 9,
                ct if ct == "text/plain" => 13,
                ct if ct.contains("xmind") || ct.contains("zip") => 14,
                ct if ct.starts_with("audio/") => 15,
                _ => 0, // Unknown
            }
        }
    };

    if media_type == 0 {
        return Err(error::file_validation_failed(format!(
            "Unsupported file type: extension={}, content_type={}",
            ext, content_type
        )).into());
    }

    Ok(media_type)
}

/// Check file size limits by media type
fn check_file_size_limit(media_type: i32, size: u64) -> Result<()> {
    const MB: u64 = 1024 * 1024;
    
    let limit = match media_type {
        5 | 7 | 13 | 14 => 10 * MB,  // Excel, Markdown, TXT, Xmind
        9 => 30 * MB,                 // Image
        _ => 200 * MB,                // PDF, Word, PPT, Audio, etc.
    };

    if size > limit {
        return Err(error::file_validation_failed(format!(
            "File size {} exceeds the {} limit for this file type",
            format_size(size),
            format_size(limit)
        )).into());
    }

    Ok(())
}

/// Format file size for display
fn format_size(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
