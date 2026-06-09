//! Notes command handlers

use crate::api::{ApiClient, ImportDocParams, AppendDocParams};
use crate::config::Config;
use crate::error::{self, Result};
use crate::commands::NotesCommands;

/// Handle notes commands
pub async fn handle_notes_command(action: NotesCommands, config: &Config) -> Result<String> {
    let client = ApiClient::new(config)?;

    match action {
        NotesCommands::ListDocs { notebook_id, cursor, limit } => {
            let response = client.list_docs(&notebook_id, &cursor, limit).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        NotesCommands::GetDoc { doc_id } => {
            let response = client.get_doc_content(&doc_id).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        NotesCommands::ImportDoc { notebook_id, title, content, format } => {
            // Validate UTF-8 encoding
            let content = validate_utf8(&content)?;
            let title = validate_utf8(&title)?;

            let params = ImportDocParams {
                notebook_id,
                title,
                content,
                content_format: format as i32,
            };
            let response = client.import_doc(&params).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        NotesCommands::AppendDoc { doc_id, content, format } => {
            // Validate UTF-8 encoding
            let content = validate_utf8(&content)?;

            let params = AppendDocParams {
                doc_id,
                content,
                content_format: format as i32,
            };
            let response = client.append_doc(&params).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }

        NotesCommands::Search { notebook_id, query, cursor, limit } => {
            let response = client.search_docs(&notebook_id, &query, &cursor, limit).await?;
            Ok(serde_json::to_string_pretty(&response.data)?)
        }
    }
}

impl From<serde_json::Error> for crate::error::Result<String> {
    fn from(err: serde_json::Error) -> Self {
        Err(crate::error::json_error(err))
    }
}

/// Validate and clean UTF-8 string
fn validate_utf8(input: &str) -> Result<String> {
    // In Rust, strings are already valid UTF-8 by design.
    // However, we can clean any potential issues by re-encoding.
    // This is mainly for compatibility with data that might have come from external sources.
    
    // Convert to bytes and back to ensure clean UTF-8
    let bytes = input.as_bytes();
    let cleaned = String::from_utf8_lossy(bytes).to_string();
    
    Ok(cleaned)
}
