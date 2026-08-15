use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleStorageBlobMetadata {
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "clientTimestamp", default)]
    pub client_timestamp: Option<String>,
    #[serde(rename = "etag", default)]
    pub etag: Option<String>,
    #[serde(rename = "length", default)]
    pub length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleStorageBlobList {
    #[serde(default)]
    pub blobs: Vec<TitleStorageBlobMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleStorageQuota {
    #[serde(rename = "usedBytes", default)]
    pub used_bytes: u64,
    #[serde(rename = "quotaBytes", default)]
    pub quota_bytes: u64,
}

pub struct TitleStorageClient<'a> {
    client: &'a reqwest::Client,
    base_url: String,
}

impl<'a> TitleStorageClient<'a> {
    pub fn new(client: &'a reqwest::Client) -> Self {
        Self {
            client,
            base_url: "https://titlestorage.xboxlive.com".to_string(),
        }
    }

    pub fn with_base_url(client: &'a reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }

    /// List blobs in a container path for a user
    pub async fn list_user_blobs(
        &self,
        auth_header: &str,
        scid: &str,
        xuid: &str,
        path: &str,
    ) -> reqwest::Result<TitleStorageBlobList> {
        let clean_path = path.trim_matches('/');
        let url = format!(
            "{}/trustedplatform/users/xuid({})/scids/{}/data/{}",
            self.base_url, xuid, scid, clean_path
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "1")
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            log::warn!("TitleStorage list failed with status: {}", resp.status());
            return Ok(TitleStorageBlobList { blobs: Vec::new() });
        }
        resp.json().await
    }

    /// Download a blob
    pub async fn download_blob(
        &self,
        auth_header: &str,
        scid: &str,
        xuid: &str,
        path_and_name: &str,
        blob_type: &str,
    ) -> reqwest::Result<Option<Vec<u8>>> {
        let clean_name = path_and_name.trim_matches('/');
        let url = format!(
            "{}/trustedplatform/users/xuid({})/scids/{}/data/{},{}",
            self.base_url, xuid, scid, clean_name, blob_type
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            log::warn!("TitleStorage download failed with status: {}", resp.status());
            return Ok(None);
        }
        let bytes = resp.bytes().await?.to_vec();
        Ok(Some(bytes))
    }

    /// Upload a blob
    pub async fn upload_blob(
        &self,
        auth_header: &str,
        scid: &str,
        xuid: &str,
        path_and_name: &str,
        blob_type: &str,
        data: Vec<u8>,
    ) -> reqwest::Result<bool> {
        let clean_name = path_and_name.trim_matches('/');
        let url = format!(
            "{}/trustedplatform/users/xuid({})/scids/{}/data/{},{}",
            self.base_url, xuid, scid, clean_name, blob_type
        );
        let resp = self
            .client
            .put(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "1")
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;

        if resp.status().is_success()
            || resp.status() == reqwest::StatusCode::CREATED
            || resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            Ok(true)
        } else {
            log::warn!("TitleStorage upload failed with status: {}", resp.status());
            Ok(false)
        }
    }

    /// Delete a blob
    pub async fn delete_blob(
        &self,
        auth_header: &str,
        scid: &str,
        xuid: &str,
        path_and_name: &str,
        blob_type: &str,
    ) -> reqwest::Result<bool> {
        let clean_name = path_and_name.trim_matches('/');
        let url = format!(
            "{}/trustedplatform/users/xuid({})/scids/{}/data/{},{}",
            self.base_url, xuid, scid, clean_name, blob_type
        );
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "1")
            .send()
            .await?;

        Ok(resp.status().is_success()
            || resp.status() == reqwest::StatusCode::NO_CONTENT
            || resp.status() == reqwest::StatusCode::NOT_FOUND)
    }
}
