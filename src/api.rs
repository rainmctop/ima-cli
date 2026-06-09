//! IMA API client module
//! 
//! Provides HTTP client for making requests to the IMA OpenAPI

use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

use crate::config::Config;
use crate::error::{self, Result};

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// IMA API Client
pub struct ApiClient {
    client: Client,
    base_url: String,
    config: Config,
}

/// Standard API response structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub msg: String,
    #[serde(default)]
    pub data: T,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| error::network_error(e))?;

        Ok(Self {
            client,
            base_url: config.base_url.clone(),
            config: config.clone(),
        })
    }

    /// Build authentication headers
    fn build_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        
        headers.insert(
            "ima-openapi-clientid".to_string(),
            self.config.client_id()?.to_string(),
        );
        headers.insert(
            "ima-openapi-apikey".to_string(),
            self.config.api_key()?.to_string(),
        );
        headers.insert(
            "ima-openapi-ctx".to_string(),
            format!("skill_version={}", self.config.skill_version),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Ok(headers)
    }

    /// Make a POST request to the IMA API
    pub async fn post<P, R>(&self, path: &str, params: &P) -> Result<R>
    where
        P: serde::Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, path);
        let headers = self.build_headers()?;

        let response = self
            .client
            .post(&url)
            .headers(headers.into_iter().map(|(k, v)| {
                (reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(), 
                 reqwest::header::HeaderValue::from_str(&v).unwrap())
            }).collect::<std::collections::HashMap<_, _>>().into_iter().collect())
            .json(params)
            .send()
            .await
            .map_err(|e| error::network_error(e))?;

        self.handle_response(response).await
    }

    /// Handle API response
    async fn handle_response<R: DeserializeOwned>(&self, response: Response) -> Result<R> {
        let status = response.status();
        
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(error::api_request_failed(format!(
                "HTTP {}: {}",
                status, error_body
            )).into());
        }

        let text = response
            .text()
            .await
            .map_err(|e| error::network_error(e))?;

        // Try to parse as generic Value first to check for error codes
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| error::json_error(e))?;

        // Check if this is an error response
        if let Some(code) = value.get("code").and_then(|v| v.as_i64()) {
            if code != 0 {
                let msg = value
                    .get("msg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                return Err(error::api_error(code as i32, msg).into());
            }
        }

        // Parse into the expected type
        let result: R = serde_json::from_value(value)
            .map_err(|e| error::json_error(e))?;

        Ok(result)
    }

    /// Make a POST request and get raw response text
    pub async fn post_raw<P>(&self, path: &str, params: &P) -> Result<String>
    where
        P: serde::Serialize + ?Sized,
    {
        let url = format!("{}/{}", self.base_url, path);
        let headers = self.build_headers()?;

        let response = self
            .client
            .post(&url)
            .headers(headers.into_iter().map(|(k, v)| {
                (reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(), 
                 reqwest::header::HeaderValue::from_str(&v).unwrap())
            }).collect::<std::collections::HashMap<_, _>>().into_iter().collect())
            .json(params)
            .send()
            .await
            .map_err(|e| error::network_error(e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| error::network_error(e))?;

        if !status.is_success() {
            return Err(error::api_request_failed(format!("HTTP {}: {}", status, text)));
        }

        Ok(text)
    }
}

// ==================== Knowledge Base API Methods ====================

impl ApiClient {
    /// Get knowledge base information
    pub async fn get_knowledge_base(&self, ids: &[String]) -> Result<KnowledgeBaseResponse> {
        #[derive(serde::Serialize)]
        struct Params {
            ids: Vec<String>,
        }

        self.post("openapi/wiki/v1/get_knowledge_base", &Params { ids: ids.to_vec() })
            .await
    }

    /// List knowledge in a knowledge base
    pub async fn get_knowledge_list(
        &self,
        knowledge_base_id: &str,
        folder_id: Option<&str>,
        cursor: &str,
        limit: u64,
    ) -> Result<KnowledgeListResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            cursor: &'a str,
            limit: u64,
            knowledge_base_id: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            folder_id: Option<&'a str>,
        }

        self.post(
            "openapi/wiki/v1/get_knowledge_list",
            &Params {
                cursor,
                limit,
                knowledge_base_id,
                folder_id,
            },
        )
        .await
    }

    /// Search knowledge in a knowledge base
    pub async fn search_knowledge(
        &self,
        knowledge_base_id: &str,
        query: &str,
        cursor: &str,
    ) -> Result<SearchKnowledgeResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            query: &'a str,
            cursor: &'a str,
            knowledge_base_id: &'a str,
        }

        self.post(
            "openapi/wiki/v1/search_knowledge",
            &Params {
                query,
                cursor,
                knowledge_base_id,
            },
        )
        .await
    }

    /// Search knowledge bases
    pub async fn search_knowledge_base(
        &self,
        query: &str,
        cursor: &str,
        limit: u64,
    ) -> Result<SearchKnowledgeBaseResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            query: &'a str,
            cursor: &'a str,
            limit: u64,
        }

        self.post(
            "openapi/wiki/v1/search_knowledge_base",
            &Params { query, cursor, limit },
        )
        .await
    }

    /// Get list of addable knowledge bases
    pub async fn get_addable_knowledge_base_list(
        &self,
        cursor: &str,
        limit: u64,
    ) -> Result<AddableKnowledgeBaseResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            cursor: &'a str,
            limit: u64,
        }

        self.post(
            "openapi/wiki/v1/get_addable_knowledge_base_list",
            &Params { cursor, limit },
        )
        .await
    }

    /// Create media (get COS upload credentials)
    pub async fn create_media(&self, params: &CreateMediaParams) -> Result<CreateMediaResponse> {
        self.post("openapi/wiki/v1/create_media", params).await
    }

    /// Add knowledge to a knowledge base
    pub async fn add_knowledge(&self, params: &AddKnowledgeParams) -> Result<AddKnowledgeResponse> {
        self.post("openapi/wiki/v1/add_knowledge", params).await
    }

    /// Import URLs to a knowledge base
    pub async fn import_urls(&self, params: &ImportUrlsParams) -> Result<ImportUrlsResponse> {
        self.post("openapi/wiki/v1/import_urls", params).await
    }

    /// Check for repeated file names
    pub async fn check_repeated_names(
        &self,
        params: &CheckRepeatedNamesParams,
    ) -> Result<CheckRepeatedNamesResponse> {
        self.post("openapi/wiki/v1/check_repeated_names", params).await
    }

    /// Get media info
    pub async fn get_media_info(&self, media_id: &str) -> Result<GetMediaInfoResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            media_id: &'a str,
        }

        self.post("openapi/wiki/v1/get_media_info", &Params { media_id }).await
    }
}

// ==================== Notes API Methods ====================

impl ApiClient {
    /// List documents in a notebook
    pub async fn list_docs(
        &self,
        notebook_id: &str,
        cursor: &str,
        limit: u64,
    ) -> Result<ListDocsResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            notebook_id: &'a str,
            cursor: &'a str,
            limit: u64,
        }

        self.post("openapi/notes/v1/list_docs", &Params {
            notebook_id,
            cursor,
            limit,
        })
        .await
    }

    /// Get document content
    pub async fn get_doc_content(&self, doc_id: &str) -> Result<GetDocContentResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            doc_id: &'a str,
        }

        self.post("openapi/notes/v1/get_doc_content", &Params { doc_id }).await
    }

    /// Import/create a new document
    pub async fn import_doc(&self, params: &ImportDocParams) -> Result<ImportDocResponse> {
        self.post("openapi/notes/v1/import_doc", params).await
    }

    /// Append content to existing document
    pub async fn append_doc(&self, params: &AppendDocParams) -> Result<AppendDocResponse> {
        self.post("openapi/notes/v1/append_doc", params).await
    }

    /// Search documents
    pub async fn search_docs(
        &self,
        notebook_id: &str,
        query: &str,
        cursor: &str,
        limit: u64,
    ) -> Result<SearchDocsResponse> {
        #[derive(serde::Serialize)]
        struct Params<'a> {
            notebook_id: &'a str,
            query: &'a str,
            cursor: &'a str,
            limit: u64,
        }

        self.post("openapi/notes/v1/search_docs", &Params {
            notebook_id,
            query,
            cursor,
            limit,
        })
        .await
    }
}

// ==================== Data Structures ====================

// Knowledge Base structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeBaseResponse {
    pub code: i32,
    pub msg: String,
    pub data: KnowledgeBaseData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeBaseData {
    pub infos: HashMap<String, KnowledgeBaseInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeBaseInfo {
    pub id: String,
    pub name: String,
    pub cover_url: String,
    pub description: String,
    pub recommended_questions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeListResponse {
    pub code: i32,
    pub msg: String,
    pub data: KnowledgeListData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeListData {
    pub knowledge_list: Vec<KnowledgeInfo>,
    pub folder_list: Vec<FolderInfo>,
    pub is_end: bool,
    pub next_cursor: String,
    pub current_path: Vec<FolderInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeInfo {
    pub media_id: String,
    pub title: String,
    pub parent_folder_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FolderInfo {
    pub folder_id: String,
    pub name: String,
    pub file_number: i64,
    pub folder_number: i64,
    pub parent_folder_id: String,
    pub is_top: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchKnowledgeResponse {
    pub code: i32,
    pub msg: String,
    pub data: SearchKnowledgeData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchKnowledgeData {
    pub info_list: Vec<SearchedKnowledgeInfo>,
    pub is_end: bool,
    pub next_cursor: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchedKnowledgeInfo {
    pub media_id: String,
    pub title: String,
    pub parent_folder_id: String,
    pub highlight_content: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchKnowledgeBaseResponse {
    pub code: i32,
    pub msg: String,
    pub data: SearchKnowledgeBaseData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchKnowledgeBaseData {
    pub info_list: Vec<SearchedKnowledgeBaseInfo>,
    pub is_end: bool,
    pub next_cursor: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchedKnowledgeBaseInfo {
    pub id: String,
    pub name: String,
    pub cover_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddableKnowledgeBaseResponse {
    pub code: i32,
    pub msg: String,
    pub data: AddableKnowledgeBaseData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddableKnowledgeBaseData {
    pub addable_knowledge_base_list: Vec<AddableKnowledgeBaseInfo>,
    pub next_cursor: String,
    pub is_end: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddableKnowledgeBaseInfo {
    pub id: String,
    pub name: String,
}

// Create Media structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateMediaParams {
    pub file_name: String,
    pub file_size: u64,
    pub content_type: String,
    pub knowledge_base_id: String,
    pub file_ext: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateMediaResponse {
    pub code: i32,
    pub msg: String,
    pub data: CreateMediaData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateMediaData {
    pub media_id: String,
    pub cos_credential: CosCredential,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CosCredential {
    pub token: String,
    pub secret_id: String,
    pub secret_key: String,
    pub start_time: i64,
    pub expired_time: i64,
    pub appid: String,
    pub bucket_name: String,
    pub region: String,
    pub custom_domain: Option<String>,
    pub cos_key: String,
}

// Add Knowledge structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddKnowledgeParams {
    pub media_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_id: Option<String>,
    pub title: String,
    pub knowledge_base_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_info: Option<ContentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_info: Option<ContentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_info: Option<ContentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_info: Option<FileInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentInfo {
    pub content_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub cos_key: String,
    pub file_size: u64,
    pub last_modify_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub file_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddKnowledgeResponse {
    pub code: i32,
    pub msg: String,
    pub data: AddKnowledgeData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddKnowledgeData {
    pub media_id: String,
}

// Import URLs structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportUrlsParams {
    pub knowledge_base_id: String,
    pub folder_id: String,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportUrlsResponse {
    pub code: i32,
    pub msg: String,
    pub data: ImportUrlsData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportUrlsData {
    pub results: HashMap<String, ImportUrlResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportUrlResult {
    pub url: String,
    pub ret_code: i32,
    pub media_id: Option<String>,
}

// Check Repeated Names structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckRepeatedNamesParams {
    pub params: Vec<CheckRepeatedNameParam>,
    pub knowledge_base_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckRepeatedNameParam {
    pub name: String,
    pub media_type: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckRepeatedNamesResponse {
    pub code: i32,
    pub msg: String,
    pub data: CheckRepeatedNamesData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckRepeatedNamesData {
    pub results: Vec<CheckRepeatedNameResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckRepeatedNameResult {
    pub name: String,
    pub is_repeated: bool,
}

// Get Media Info structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetMediaInfoResponse {
    pub code: i32,
    pub msg: String,
    pub data: GetMediaInfoData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetMediaInfoData {
    pub media_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_info: Option<UrlInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notebook_ext_info: Option<NotebookExtInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UrlInfo {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotebookExtInfo {
    pub notebook_id: String,
}

// Notes structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListDocsResponse {
    pub code: i32,
    pub msg: String,
    pub data: ListDocsData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListDocsData {
    pub doc_list: Vec<DocInfo>,
    pub is_end: bool,
    pub next_cursor: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocInfo {
    pub doc_id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetDocContentResponse {
    pub code: i32,
    pub msg: String,
    pub data: DocContent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocContent {
    pub doc_id: String,
    pub title: String,
    pub content: String,
    pub content_format: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportDocParams {
    pub notebook_id: String,
    pub title: String,
    pub content: String,
    pub content_format: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportDocResponse {
    pub code: i32,
    pub msg: String,
    pub data: ImportDocData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportDocData {
    pub doc_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppendDocParams {
    pub doc_id: String,
    pub content: String,
    pub content_format: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppendDocResponse {
    pub code: i32,
    pub msg: String,
    pub data: AppendDocData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppendDocData {
    pub doc_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchDocsResponse {
    pub code: i32,
    pub msg: String,
    pub data: SearchDocsData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchDocsData {
    pub doc_list: Vec<SearchedDocInfo>,
    pub is_end: bool,
    pub next_cursor: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchedDocInfo {
    pub doc_id: String,
    pub title: String,
    pub highlight_content: Option<String>,
}
