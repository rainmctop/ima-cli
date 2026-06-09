//! IMA CLI - A Rust-based command-line tool for IMA OpenAPI
//! 
//! This tool provides all functionality from the original Node.js implementation:
//! - Knowledge base management (upload files, add URLs, search, browse)
//! - Notes management (create, append, search, browse)
//! - Automatic credential loading from config file or environment variables
//! - Temporary file storage before upload for large files
//! 
//! Exit codes:
//! - 0: Success
//! - 1: Programmatic error (bad args, missing credentials, network, etc.)
//! - 2: Update available

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::{self, Write};

mod error;
mod config;
mod api;
mod cos;
mod commands;

use error::{Result, CliError};
use config::Config;

/// IMA CLI - Command-line tool for IMA knowledge base and notes management
#[derive(Parser)]
#[command(name = "ima")]
#[command(author = "IMA Skill Contributors")]
#[command(version = "1.1.7")]
#[command(about = "IMA OpenAPI CLI for knowledge base and notes management", long_about = None)]
struct Cli {
    /// Path to config file (default: ~/.config/ima/config.toml)
    #[arg(long, env = "IMA_CONFIG_FILE")]
    config: Option<PathBuf>,

    /// Force check for skill updates
    #[arg(long, env = "IMA_FORCE_UPDATE_CHECK")]
    force_update_check: bool,

    /// Base URL for IMA API (default: https://ima.qq.com)
    #[arg(long, env = "IMA_BASE_URL", default_value = "https://ima.qq.com")]
    base_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Knowledge base operations
    Kb {
        #[command(subcommand)]
        action: KbCommands,
    },

    /// Notes operations
    Notes {
        #[command(subcommand)]
        action: NotesCommands,
    },

    /// Check for skill updates
    CheckUpdate,

    /// Legacy API call interface (compatible with ima_api.cjs)
    Api {
        /// API path (e.g., openapi/wiki/v1/get_knowledge_base)
        api_path: String,

        /// Request body as JSON string
        #[arg(default_value = "{}")]
        body: String,

        /// Options as JSON string (clientId, apiKey, etc.)
        #[arg(default_value = "{}")]
        options: String,
    },
}

#[derive(Subcommand)]
enum KbCommands {
    /// Get knowledge base information
    Info {
        /// Knowledge base IDs (comma-separated)
        #[arg(short, long, required = true)]
        ids: String,
    },

    /// List knowledge in a knowledge base
    List {
        /// Knowledge base ID
        #[arg(short, long, required = true)]
        kb_id: String,

        /// Folder ID (optional, defaults to root)
        #[arg(short, long)]
        folder_id: Option<String>,

        /// Number of items to return (1-50)
        #[arg(short, long, default_value = "20")]
        limit: u64,

        /// Cursor for pagination
        #[arg(short, long, default_value = "")]
        cursor: String,
    },

    /// Search knowledge in a knowledge base
    Search {
        /// Knowledge base ID
        #[arg(short, long, required = true)]
        kb_id: String,

        /// Search query
        #[arg(short, long, required = true)]
        query: String,

        /// Cursor for pagination
        #[arg(short, long, default_value = "")]
        cursor: String,
    },

    /// Search knowledge bases
    SearchKb {
        /// Search query
        #[arg(short, long, required = true)]
        query: String,

        /// Number of results (1-20)
        #[arg(short, long, default_value = "20")]
        limit: u64,

        /// Cursor for pagination
        #[arg(short, long, default_value = "")]
        cursor: String,
    },

    /// Get list of addable knowledge bases
    Addable {
        /// Number of results (1-50)
        #[arg(short, long, default_value = "50")]
        limit: u64,

        /// Cursor for pagination
        #[arg(short, long, default_value = "")]
        cursor: String,
    },

    /// Upload a file to knowledge base
    Upload {
        /// Path to the file to upload
        #[arg(short, long, required = true)]
        file: PathBuf,

        /// Knowledge base ID
        #[arg(short, long, required = true)]
        kb_id: String,

        /// Folder ID (optional, defaults to root)
        #[arg(short, long)]
        folder_id: Option<String>,

        /// Content type (optional, auto-detected if not provided)
        #[arg(long)]
        content_type: Option<String>,

        /// Title for the knowledge (optional, defaults to filename)
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Import URLs to knowledge base
    ImportUrls {
        /// Knowledge base ID
        #[arg(short, long, required = true)]
        kb_id: String,

        /// Folder ID (required for import_urls)
        #[arg(short, long, required = true)]
        folder_id: String,

        /// URLs to import (1-10)
        #[arg(required = true)]
        urls: Vec<String>,
    },

    /// Check for repeated file names
    CheckRepeated {
        /// Knowledge base ID
        #[arg(short, long, required = true)]
        kb_id: String,

        /// Folder ID (optional, defaults to root)
        #[arg(short, long)]
        folder_id: Option<String>,

        /// File names to check (format: name:media_type, e.g., "report.pdf:1")
        #[arg(required = true)]
        files: Vec<String>,
    },

    /// Get media info
    MediaInfo {
        /// Media ID
        #[arg(short, long, required = true)]
        media_id: String,
    },
}

#[derive(Subcommand)]
enum NotesCommands {
    /// List documents in a notebook
    ListDocs {
        /// Notebook ID
        #[arg(short, long, required = true)]
        notebook_id: String,

        /// Cursor for pagination
        #[arg(short, long, default_value = "")]
        cursor: String,

        /// Number of results (1-50)
        #[arg(short, long, default_value = "20")]
        limit: u64,
    },

    /// Get document content
    GetDoc {
        /// Document ID
        #[arg(short, long, required = true)]
        doc_id: String,
    },

    /// Import/create a new document
    ImportDoc {
        /// Notebook ID
        #[arg(short, long, required = true)]
        notebook_id: String,

        /// Document title
        #[arg(short, long, required = true)]
        title: String,

        /// Document content
        #[arg(short, long, required = true)]
        content: String,

        /// Content format (1=markdown, 2=plain text)
        #[arg(short, long, default_value = "1")]
        format: u32,
    },

    /// Append content to existing document
    AppendDoc {
        /// Document ID
        #[arg(short, long, required = true)]
        doc_id: String,

        /// Content to append
        #[arg(short, long, required = true)]
        content: String,

        /// Content format (1=markdown, 2=plain text)
        #[arg(short, long, default_value = "1")]
        format: u32,
    },

    /// Search documents
    Search {
        /// Notebook ID
        #[arg(short, long, required = true)]
        notebook_id: String,

        /// Search query
        #[arg(short, long, required = true)]
        query: String,

        /// Cursor for pagination
        #[arg(short, long, default_value = "")]
        cursor: String,

        /// Number of results (1-50)
        #[arg(short, long, default_value = "20")]
        limit: u64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Load configuration
    let config = match Config::load(cli.config.as_ref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{}", serde_json::to_string(&CliError {
                code: error::ERR_PROGRAMMATIC,
                msg: format!("Failed to load config: {}", e),
            }).unwrap_or_default());
            std::process::exit(1);
        }
    };

    // Apply CLI overrides
    let mut config = config;
    if cli.force_update_check {
        config.force_update_check = true;
    }
    config.base_url = cli.base_url;

    let result = match cli.command {
        Commands::Kb { action } => commands::kb::handle_kb_command(action, &config).await,
        Commands::Notes { action } => commands::notes::handle_notes_command(action, &config).await,
        Commands::CheckUpdate => commands::check_update(&config).await,
        Commands::Api { api_path, body, options } => {
            commands::legacy_api(&api_path, &body, &options, &config).await
        }
    };

    match result {
        Ok(output) => {
            print!("{}", output);
            std::process::exit(0);
        }
        Err(e) => {
            let error_msg = serde_json::to_string(&CliError {
                code: e.code(),
                msg: e.message(),
            }).unwrap_or_default();
            
            eprintln!("{}", error_msg);
            std::process::exit(1);
        }
    }
}
