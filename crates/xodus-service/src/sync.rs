use std::sync::Arc;
use xodus::api::xbox::auth::get_xsts_auth_header;
use xodus::api::xbox::get_or_request_xsts;
use xodus::api::xbox::titlestorage::TitleStorageClient;
use xodus::tokens::TokenManager;

pub async fn sync_container_down(
    tokens: Arc<TokenManager>,
    scid: String,
    xuid: String,
    user_id: u64,
    container_name: String,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let xsts = match get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Cloud save sync: Failed to acquire XSTS token: {e}");
                return;
            }
        };

        let auth_header = get_xsts_auth_header(xsts);
        let storage = TitleStorageClient::new(&client);

        let list = match storage
            .list_user_blobs(&auth_header, &scid, &xuid, &container_name)
            .await
        {
            Ok(l) => l,
            Err(e) => {
                log::warn!("Cloud save sync: Failed to list remote blobs for {container_name}: {e}");
                return;
            }
        };

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let container_dir = std::path::PathBuf::from(home)
            .join(".local/share/xodus/saves")
            .join(&scid)
            .join(user_id.to_string())
            .join(&container_name);

        let _ = tokio::fs::create_dir_all(&container_dir).await;

        for blob in list.blobs {
            let blob_path = format!("{}/{}", container_name, blob.file_name);
            let blob_type = if blob.file_name.ends_with(".json") {
                "json"
            } else {
                "binary"
            };

            match storage
                .download_blob(&auth_header, &scid, &xuid, &blob_path, blob_type)
                .await
            {
                Ok(Some(data)) => {
                    let local_file = container_dir.join(&blob.file_name);
                    if let Err(e) = tokio::fs::write(&local_file, &data).await {
                        log::error!("Cloud save sync: Failed to write {local_file:?}: {e}");
                    } else {
                        log::info!("Cloud save sync: Downloaded {} ({} bytes)", blob.file_name, data.len());
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("Cloud save sync: Failed to download {blob_path}: {e}");
                }
            }
        }
    });
}

pub async fn sync_blob_up(
    tokens: Arc<TokenManager>,
    scid: String,
    xuid: String,
    container_name: String,
    blob_name: String,
    data: Vec<u8>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let xsts = match get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Cloud save sync: Failed to acquire XSTS token for upload: {e}");
                return;
            }
        };

        let auth_header = get_xsts_auth_header(xsts);
        let storage = TitleStorageClient::new(&client);

        let blob_path = format!("{}/{}", container_name, blob_name);
        let blob_type = if blob_name.ends_with(".json") {
            "json"
        } else {
            "binary"
        };

        match storage
            .upload_blob(&auth_header, &scid, &xuid, &blob_path, blob_type, data)
            .await
        {
            Ok(true) => {
                log::info!("Cloud save sync: Successfully uploaded {blob_path} to Xbox Live");
            }
            Ok(false) => {
                log::warn!("Cloud save sync: Upload returned failure for {blob_path}");
            }
            Err(e) => {
                log::warn!("Cloud save sync: Network error uploading {blob_path}: {e}");
            }
        }
    });
}
