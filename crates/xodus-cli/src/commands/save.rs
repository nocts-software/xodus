use std::path::PathBuf;
use std::process::ExitCode;
use xodus::api::xbox::auth::get_xsts_auth_header;
use xodus::api::xbox::get_or_request_xsts;
use xodus::api::xbox::titlestorage::TitleStorageClient;
use xodus::tokens::TokenManager;

pub async fn pull(client: &reqwest::Client, tokens: &TokenManager, source: String) -> ExitCode {
    println!("Pulling Xbox Live cloud saves for: {source}");
    let (scid, xuid) = match resolve_scid_and_xuid(tokens, &source).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to resolve title metadata: {e}");
            return ExitCode::FAILURE;
        }
    };

    let xsts = match get_or_request_xsts(client, tokens, "http://xboxlive.com").await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get XSTS auth token: {e}");
            return ExitCode::FAILURE;
        }
    };

    let auth_header = get_xsts_auth_header(xsts);
    let storage = TitleStorageClient::new(client);

    println!("Querying cloud save containers for SCID {scid} (XUID: {xuid})...");
    match storage.list_user_blobs(&auth_header, &scid, &xuid, "").await {
        Ok(list) => {
            if list.blobs.is_empty() {
                println!("No remote cloud save blobs found for this title.");
                return ExitCode::SUCCESS;
            }
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let save_dir = PathBuf::from(home)
                .join(".local/share/xodus/saves")
                .join(&scid)
                .join("1");
            let _ = tokio::fs::create_dir_all(&save_dir).await;

            for blob in list.blobs {
                let blob_type = if blob.file_name.ends_with(".json") {
                    "json"
                } else {
                    "binary"
                };
                print!("Downloading {}... ", blob.file_name);
                match storage
                    .download_blob(&auth_header, &scid, &xuid, &blob.file_name, blob_type)
                    .await
                {
                    Ok(Some(data)) => {
                        let local_path = save_dir.join(&blob.file_name);
                        if let Some(parent) = local_path.parent() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                        if let Err(e) = tokio::fs::write(&local_path, &data).await {
                            println!("FAILED to write: {e}");
                        } else {
                            println!("OK ({} bytes)", data.len());
                        }
                    }
                    Ok(None) => println!("NOT FOUND"),
                    Err(e) => println!("ERROR: {e}"),
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to query title storage: {e}");
            ExitCode::FAILURE
        }
    }
}

pub async fn push(client: &reqwest::Client, tokens: &TokenManager, source: String) -> ExitCode {
    println!("Pushing local saves to Xbox Live cloud for: {source}");
    let (scid, xuid) = match resolve_scid_and_xuid(tokens, &source).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to resolve title metadata: {e}");
            return ExitCode::FAILURE;
        }
    };

    let xsts = match get_or_request_xsts(client, tokens, "http://xboxlive.com").await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get XSTS auth token: {e}");
            return ExitCode::FAILURE;
        }
    };

    let auth_header = get_xsts_auth_header(xsts);
    let storage = TitleStorageClient::new(client);

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let save_dir = PathBuf::from(home)
        .join(".local/share/xodus/saves")
        .join(&scid)
        .join("1");

    if !save_dir.exists() {
        println!("No local saves found at {save_dir:?}");
        return ExitCode::SUCCESS;
    }

    let mut entries = match tokio::fs::read_dir(&save_dir).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to read local save directory: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut count = 0;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            let blob_type = if name.ends_with(".json") {
                "json"
            } else {
                "binary"
            };
            if let Ok(data) = tokio::fs::read(&path).await {
                print!("Uploading {name} ({} bytes)... ", data.len());
                match storage
                    .upload_blob(&auth_header, &scid, &xuid, &name, blob_type, data)
                    .await
                {
                    Ok(true) => {
                        println!("OK");
                        count += 1;
                    }
                    Ok(false) => println!("REJECTED"),
                    Err(e) => println!("ERROR: {e}"),
                }
            }
        }
    }

    println!("Uploaded {count} save blob(s) successfully.");
    ExitCode::SUCCESS
}

pub async fn status(client: &reqwest::Client, tokens: &TokenManager, source: String) -> ExitCode {
    println!("Checking Xbox Live cloud save status for: {source}");
    let (scid, xuid) = match resolve_scid_and_xuid(tokens, &source).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to resolve title metadata: {e}");
            return ExitCode::FAILURE;
        }
    };

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let save_dir = PathBuf::from(home)
        .join(".local/share/xodus/saves")
        .join(&scid)
        .join("1");

    println!("Local Save Path: {save_dir:?}");
    if save_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&save_dir).await {
            println!("Local Blobs:");
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(meta) = entry.metadata().await {
                    println!("  - {:<24} ({} bytes, modified: {:?})", entry.file_name().to_string_lossy(), meta.len(), meta.modified().ok());
                }
            }
        }
    } else {
        println!("  (No local saves found)");
    }

    if let Ok(xsts) = get_or_request_xsts(client, tokens, "http://xboxlive.com").await {
        let auth_header = get_xsts_auth_header(xsts);
        let storage = TitleStorageClient::new(client);
        if let Ok(list) = storage.list_user_blobs(&auth_header, &scid, &xuid, "").await {
            println!("\nRemote Cloud Blobs (titlestorage.xboxlive.com):");
            if list.blobs.is_empty() {
                println!("  (No remote blobs found)");
            } else {
                for blob in list.blobs {
                    println!("  - {:<24} ({} bytes, etag: {:?})", blob.file_name, blob.length, blob.etag);
                }
            }
        }
    }

    ExitCode::SUCCESS
}

async fn resolve_scid_and_xuid(
    tokens: &TokenManager,
    source: &str,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let p = PathBuf::from(source);
    let mut title_id = "77BB5AFB".to_string();

    let cfg_path = if p.is_dir() {
        p.join("MicrosoftGame.config")
    } else {
        p.parent().unwrap().join("MicrosoftGame.config")
    };

    if cfg_path.exists() {
        if let Ok(xml) = tokio::fs::read_to_string(&cfg_path).await {
            if let Some(pos) = xml.find("TitleId=\"") {
                let rest = &xml[pos + 9..];
                if let Some(end) = rest.find('"') {
                    title_id = rest[..end].to_string();
                }
            }
        }
    }

    let scid = format!("00000000-0000-0000-0000-0000{}", title_id.to_lowercase());
    let user = tokens.get_user().ok();
    let xuid = user
        .map(|u| u.puid)
        .unwrap_or_else(|| "2533274839201029".to_string());

    Ok((scid, xuid))
}

