//! COS (Cloud Object Storage) upload module for IMA CLI
//! 
//! Implements Tencent Cloud COS upload using temporary credentials from IMA API

use reqwest::{Client, StatusCode};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use hex::ToHex;

use crate::api::CosCredential;
use crate::error::{self, Result};

type HmacSha1 = Hmac<Sha1>;

/// Upload file to COS
pub async fn upload_file(
    credential: &CosCredential,
    file_path: &Path,
    file_content: &[u8],
) -> Result<()> {
    let bucket = &credential.bucket_name;
    let region = &credential.region;
    let cos_key = &credential.cos_key;
    let appid = &credential.appid;

    // Build COS endpoint
    let hostname = format!("{}.cos.{}.myqcloud.com", bucket, region);
    let pathname = format!("/{}", cos_key);

    // Get current timestamp
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expired_time = start_time + 3600; // 1 hour validity

    // Build authorization header
    let authorization = build_authorization(
        &credential.secret_id,
        &credential.secret_key,
        "PUT",
        &pathname,
        file_content.len(),
        &hostname,
        start_time,
        expired_time,
    );

    // Build request
    let url = format!("https://{}{}", hostname, pathname);
    
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10 minutes for large files
        .build()
        .map_err(|e| error::network_error(e))?;

    let response = client
        .put(&url)
        .header("Authorization", authorization)
        .header("x-cos-security-token", &credential.token)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", file_content.len())
        .header("Host", &hostname)
        .body(file_content.to_vec())
        .send()
        .await
        .map_err(|e| error::network_error(e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(error::cos_upload_failed(format!(
            "COS upload failed (HTTP {}): {}",
            status, body
        )).into());
    }

    Ok(())
}

/// Build COS Authorization header
/// Reference: https://cloud.tencent.com/document/product/436/7778
fn build_authorization(
    secret_id: &str,
    secret_key: &str,
    method: &str,
    pathname: &str,
    content_length: usize,
    host: &str,
    start_time: i64,
    expired_time: i64,
) -> String {
    let key_time = format!("{};{}", start_time, expired_time);

    // 1. SignKey = HMAC-SHA1(SecretKey, KeyTime)
    let sign_key = hmac_sha1(secret_key.as_bytes(), key_time.as_bytes());

    // 2. Build signed headers
    // For PUT, we sign: host, content-length
    let header_list = vec!["content-length", "host"];
    let http_headers = format!(
        "content-length={}&host={}",
        url_encode(&content_length.to_string()),
        url_encode(host)
    );

    // 3. HttpString = method\npathname\nparams\nheaders\n
    let http_string = format!(
        "{}\n{}\n\n{}\n",
        method.to_lowercase(),
        pathname,
        http_headers
    );

    // 4. StringToSign = sha1\nKeyTime\nSHA1(HttpString)\n
    let string_to_sign = format!(
        "sha1\n{}\n{}\n",
        key_time,
        sha1_hex(http_string.as_bytes())
    );

    // 5. Signature = HMAC-SHA1(SignKey, StringToSign)
    let signature = hmac_sha1(&sign_key, string_to_sign.as_bytes());

    // 6. Build Authorization header
    let signature_hex = signature.encode_hex::<String>();
    format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list=&q-signature={}",
        secret_id,
        key_time,
        key_time,
        header_list.join(";"),
        signature_hex
    )
}

/// Compute HMAC-SHA1
fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Compute SHA1 hash in hex format
fn sha1_hex(data: &[u8]) -> String {
    use sha1::Digest;
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize().encode_hex()
}

/// URL encode a string
fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha1() {
        let key = b"secret_key";
        let data = b"hello";
        let result = hmac_sha1(key, data);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_sha1_hex() {
        let data = b"hello";
        let result = sha1_hex(data);
        assert_eq!(result, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert!(url_encode("测试").contains("%"));
    }
}
