use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use xodus::proto::xodus::{
    XGameSaveBlobInfo, XGameSaveEnumerateBlobsRequest, XGameSaveEnumerateBlobsResponse,
    XGameSaveReadBlobRequest, XGameSaveReadBlobResponse, XGameSaveWriteBlobRequest,
    XGameSaveWriteBlobResponse, XodusMessage, XodusMessageType, XStoreAcquireLicenseRequest,
    XStoreAcquireLicenseResponse, XStoreQueryLicenseRequest, XStoreQueryLicenseResponse,
    XStoreQueryProductsRequest, XStoreQueryProductsResponse, XUserAddRequest, XUserAddResponse,
    XUserCheckPrivilegeRequest, XUserCheckPrivilegeResponse, XUserGetGamerPictureRequest,
    XUserGetGamerPictureResponse, XUserGetGamertagRequest, XUserGetGamertagResponse,
    XUserGetTokenRequest, XUserGetTokenResponse,
};




use crate::simple_context::SimpleContext;

pub async fn handle(
    socket: &mut tokio::net::UnixStream,
    context: &mut SimpleContext,
) -> tokio::io::Result<()> {
    let mut len_bytes = [0u8; 4];
    if let Err(e) = socket.read_exact(&mut len_bytes).await {
        if e.kind() == tokio::io::ErrorKind::UnexpectedEof {
            return Ok(());
        }
        return Err(e);
    }
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > 10 * 1024 * 1024 {
        return Err(tokio::io::Error::new(
            tokio::io::ErrorKind::InvalidData,
            "Payload too large",
        ));
    }

    let mut buf = vec![0u8; len];
    socket.read_exact(&mut buf).await?;

    let msg = match XodusMessage::decode(&buf[..]) {
        Ok(m) => m,
        Err(err) => {
            log::error!("Failed to decode XodusMessage: {err}");
            return Ok(());
        }
    };

        let request_id = msg.request_id;
        let (resp_type, resp_payload) = match XodusMessageType::try_from(msg.msg_type) {
            Ok(XodusMessageType::Ping) => (XodusMessageType::Pong, vec![]),

            Ok(XodusMessageType::XuserAddRequest) => {
                let _req = XUserAddRequest::decode(&msg.payload[..]).ok();
                log::info!("[XODUS-SERVICE] Processing XuserAddRequest from client game...");
                let (xuid, gamertag) = match xodus::api::xbox::get_or_request_xsts(&context.client, context.tokens(), "http://xboxlive.com").await {
                    Ok(xsts) => {
                        let xuid = xsts.xuid().map(|s| s.to_string()).unwrap_or_else(|| "2533274839201029".to_string());
                        let gamertag = xsts.gamertag().map(|s| s.to_string()).unwrap_or_else(|| "XodusUser".to_string());
                        log::info!("[XODUS-SERVICE] XuserAddRequest resolved live user: XUID={xuid}, Gamertag={gamertag}");
                        (xuid, gamertag)
                    }
                    Err(e) => {
                        log::warn!("[XODUS-SERVICE] XuserAddRequest: error fetching live xsts: {e}, falling back to legacy token cache");
                        let default_xuid = "2533274839201029".to_string();
                        let default_gt = "XodusUser".to_string();
                        (default_xuid, default_gt)
                    }
                };
                let resp = XUserAddResponse {
                    status: 0, // S_OK
                    user_id: 1,
                    xuid: xuid.clone(),
                    gamertag: gamertag.clone(),
                };
                log::info!("[XODUS-SERVICE] Responding to XuserAddRequest: user_id=1, XUID={xuid}, Gamertag={gamertag}");
                (XodusMessageType::XuserAddResponse, resp.encode_to_vec())
            }

            Ok(XodusMessageType::XuserGetGamertagRequest) => {
                let _req = XUserGetGamertagRequest::decode(&msg.payload[..]).ok();
                let gamertag = match xodus::api::xbox::get_or_request_xsts(&context.client, context.tokens(), "http://xboxlive.com").await {
                    Ok(xsts) => xsts.gamertag().map(|s| s.to_string()).unwrap_or_else(|| "XodusUser".to_string()),
                    Err(e) => {
                        log::warn!("[XODUS-SERVICE] XuserGetGamertagRequest: failed to get XSTS: {e}");
                        "XodusUser".to_string()
                    }
                };
                log::info!("[XODUS-SERVICE] Responding to XuserGetGamertagRequest: Gamertag={gamertag}");
                let resp = XUserGetGamertagResponse {
                    status: 0,
                    gamertag,
                };
                (
                    XodusMessageType::XuserGetGamertagResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XuserGetTokenRequest) => {
                let req = XUserGetTokenRequest::decode(&msg.payload[..]).ok();
                let raw_relying_party = req
                    .as_ref()
                    .and_then(|r| if r.relying_party.is_empty() { None } else { Some(r.relying_party.clone()) })
                    .unwrap_or_else(|| "http://xboxlive.com".to_string());

                let beard_context = if raw_relying_party.contains("discovery.prod.athena") {
                    "KIWIBEARD-CHECK: Athena Title Discovery (validating Title ID, version & maintenance)"
                } else if raw_relying_party.contains("ares") || raw_relying_party.contains("athena.prod.msrareservices.com") {
                    "ERMINEBEARD-CHECK: Athena Ares Login (validating XSTS claims, ProofKey signature & license)"
                } else if raw_relying_party.contains("eos") || raw_relying_party.contains("epicgames") {
                    "HAZELBEARD-CHECK: EOS Crossplay & Lobby Auth"
                } else if raw_relying_party.contains("profile.xboxlive.com") {
                    "ASHBEARD-CHECK: Xbox Live User Profile & Settings"
                } else {
                    "GENERIC-XBL-AUTH: Xbox Live Token Exchange"
                };

                log::info!("[MS-IPC] ========================================================");
                log::info!("[MS-IPC] === Incoming XuserGetTokenRequest from Game ===");
                log::info!("[BEARD-DIAG] Target Phase: {}", beard_context);
                log::info!("[MS-IPC]   Raw Relying Party: '{}'", raw_relying_party);

                // Sanitize relying party (if full URL, extract scheme + host)
                let sanitized_rp = if let Some(scheme_idx) = raw_relying_party.find("://") {
                    let after_scheme = &raw_relying_party[scheme_idx + 3..];
                    let host = after_scheme.split('/').next().unwrap_or(after_scheme);
                    let scheme = &raw_relying_party[..scheme_idx];
                    format!("{scheme}://{host}")
                } else {
                    raw_relying_party.clone()
                };

                let mut detected_tid: Option<u32> = if let Some(idx) = raw_relying_party.find("tid=") {
                    let after_tid = &raw_relying_party[idx + 4..];
                    let num_str: String = after_tid.chars().take_while(|c| c.is_ascii_digit()).collect();
                    num_str.parse().ok()
                } else {
                    None
                };

                let effective_rp = if raw_relying_party.contains("msrareservices.com") || raw_relying_party.contains("athena") {
                    detected_tid = Some(1717113201);
                    "rp://athena.prod.msrareservices.com/".to_string()
                } else if raw_relying_party.contains("epicgames.dev") || raw_relying_party.contains("eos.seaofthieves") {
                    detected_tid = Some(1717113201);
                    "rp://eos.seaofthieves.com/".to_string()
                } else if raw_relying_party.contains("minecraft") || raw_relying_party.contains("playfabapi.com") || raw_relying_party.contains("mojang") {
                    if detected_tid.is_none() {
                        detected_tid = Some(896932871); // Minecraft for Windows Title ID (35760C07)
                    }
                    if !raw_relying_party.ends_with('/') {
                        format!("{}/", raw_relying_party)
                    } else {
                        raw_relying_party.clone()
                    }
                } else if raw_relying_party.contains("xboxlive.com") {
                    "http://xboxlive.com".to_string()
                } else {
                    sanitized_rp.clone()
                };

                log::info!("[MS-IPC]   Sanitized RP: '{}'", sanitized_rp);
                log::info!("[MS-IPC]   Effective RP: '{}'", effective_rp);
                log::info!("[MS-IPC]   Detected Title ID: {:?}", detected_tid);

                let start_time = std::time::Instant::now();
                let mut token_res = if let Some(tid) = detected_tid {
                    log::info!("[MS-IPC] Requesting multi-claim XSTS token (Title ID: {}, RP: '{}')...", tid, effective_rp);
                    xodus::api::xbox::get_or_request_xsts_for_title(&context.client, context.tokens(), tid, &effective_rp).await
                } else {
                    log::info!("[MS-IPC] Requesting standard XSTS token (RP: '{}')...", effective_rp);
                    xodus::api::xbox::get_or_request_xsts(&context.client, context.tokens(), &effective_rp).await
                };

                if token_res.is_err() && effective_rp != "http://xboxlive.com" {
                    log::warn!("[MS-IPC] Effective RP '{}' failed ({:?}), retrying with raw RP '{}'...", effective_rp, token_res.as_ref().err(), raw_relying_party);
                    token_res = if let Some(tid) = detected_tid {
                        xodus::api::xbox::get_or_request_xsts_for_title(&context.client, context.tokens(), tid, &raw_relying_party).await
                    } else {
                        xodus::api::xbox::get_or_request_xsts(&context.client, context.tokens(), &raw_relying_party).await
                    };
                }

                if token_res.is_err() && effective_rp != "http://xboxlive.com" && raw_relying_party != "http://xboxlive.com" {
                    log::warn!("[MS-IPC] Specific RP failed, falling back to standard http://xboxlive.com...");
                    token_res = xodus::api::xbox::get_or_request_xsts(&context.client, context.tokens(), "http://xboxlive.com").await;
                }

                let elapsed = start_time.elapsed();
                let (token, signature) = match token_res {
                    Ok(xsts) => {
                        let uhs = xsts.user_hash().unwrap_or("UNKNOWN").to_string();
                        let header = xodus::api::xbox::get_xsts_auth_header(xsts.clone());
                        log::info!("[MS-IPC] === XSTS Token Success in {:.2?} ===", elapsed);
                        log::info!("[MS-IPC]   User Hash (UHS): {}", uhs);
                        log::info!("[MS-IPC]   Authorization Header: length={} bytes, prefix='{}...'", header.len(), &header[..std::cmp::min(40, header.len())]);

                        let req_ref = req.as_ref();
                        let method = req_ref.map(|r| r.http_method.as_str()).unwrap_or("GET");
                        let url = req_ref.map(|r| r.url.as_str()).unwrap_or(&raw_relying_party);
                        let body = req_ref.map(|r| r.body.as_slice()).unwrap_or(&[]);

                        // Log the actual request body for Ares/Athena requests (critical for Erminebeard debugging)
                        if effective_rp.contains("athena") || effective_rp.contains("msrareservices") || url.contains("ares") {
                            let body_str = std::str::from_utf8(body).unwrap_or("<non-utf8>");
                            log::info!("[ERMINEBEARD-DIAG] SoT Ares Request: method={}, url={}", method, url);
                            log::info!("[ERMINEBEARD-DIAG] SoT Ares Request body (len={}): {}", body.len(), &body_str[..body_str.len().min(2048)]);
                        }

                        let sig = match xodus::api::xbox::sign_request_for_rp(
                            &effective_rp,
                            if method.is_empty() { "GET" } else { method },
                            if url.is_empty() { &raw_relying_party } else { url },
                            &header,
                            body,
                        ) {
                            Some(s) => {
                                log::info!("[MS-IPC] Generated authentic ECDSA Proof-of-Possession signature for {} {} (length: {})", method, url, s.len());
                                s
                            }
                            None => {
                                log::info!("[MS-IPC] No session signer needed/found for RP '{}', returning empty signature", effective_rp);
                                String::new()
                            }
                        };

                        log::info!("[MS-IPC]   Calculated Request Signature: length={} bytes, value='{}...'", sig.len(), &sig[..std::cmp::min(30, sig.len())]);
                        log::info!("[BEARD-DIAG] Handshake payload generated cleanly for '{}'", beard_context);

                        let returned_token = if raw_relying_party.contains("playfabapi.com") || raw_relying_party.contains("minecraft") {
                            log::info!("[MS-IPC] Stripping XBL3.0 prefix from token for PlayFab/Minecraft RP.");
                            xsts.token.clone()
                        } else {
                            header.clone()
                        };

                        (returned_token, sig)
                    }
                    Err(err) => {
                        log::error!("[BEARD-DIAG] WARNING: Handshake failure on '{}': {}", beard_context, err);
                        log::error!("[MS-IPC] === XSTS Token Acquisition FAILED in {:.2?}: {} ===", elapsed, err);
                        let fallback_tok = match context.tokens().get_user_sts_token() {
                            Ok(xodus::models::secrets::Token::Legacy(tok)) => tok.token,
                            _ => "MOCK_XSTS_TOKEN".to_string(),
                        };
                        (fallback_tok, String::new())
                    }
                };

                let resp = XUserGetTokenResponse {
                    status: 0,
                    token,
                    signature,
                };
                log::info!("[MS-IPC] ========================================================");
                (
                    XodusMessageType::XuserGetTokenResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XuserGetGamerPictureRequest) => {
                let _req = XUserGetGamerPictureRequest::decode(&msg.payload[..]).ok();
                let xsts_res = xodus::api::xbox::get_or_request_xsts(&context.client, context.tokens(), "http://xboxlive.com").await;
                let (auth_header, xuid) = match xsts_res {
                    Ok(ref xsts) => (
                        xodus::api::xbox::get_xsts_auth_header(xsts.clone()),
                        xsts.xuid().map(|s| s.to_string()).unwrap_or_else(|| "2533274976279120".to_string()),
                    ),
                    Err(_) => (String::new(), "2533274976279120".to_string()),
                };

                let picture_data = if !auth_header.is_empty() {
                    xodus::api::xbox::profile::fetch_or_cache_gamer_picture(
                        &context.client,
                        &auth_header,
                        &xuid,
                    ).await
                } else {
                    vec![0x89, 0x50, 0x4E, 0x47]
                };

                let resp = XUserGetGamerPictureResponse {
                    status: 0,
                    picture_data,
                };
                (
                    XodusMessageType::XuserGetGamerPictureResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XuserCheckPrivilegeRequest) => {
                let _req = XUserCheckPrivilegeRequest::decode(&msg.payload[..]).ok();
                let resp = XUserCheckPrivilegeResponse {
                    status: 0,
                    has_privilege: true,
                    deny_reason: 0,
                };
                (
                    XodusMessageType::XuserCheckPrivilegeResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XstoreQueryLicenseRequest) => {
                let _req = XStoreQueryLicenseRequest::decode(&msg.payload[..]).ok();
                let resp = XStoreQueryLicenseResponse {
                    status: 0,
                    is_licensed: true,
                    license_blob: vec![1, 2, 3, 4],
                };
                (
                    XodusMessageType::XstoreQueryLicenseResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XstoreQueryProductsRequest) => {
                let _req = XStoreQueryProductsRequest::decode(&msg.payload[..]).ok();
                let resp = XStoreQueryProductsResponse {
                    status: 0,
                    count: 1,
                    products_json: r#"[{"store_id":"mock_product","is_in_user_collection":true}]"#.to_string(),
                };
                (
                    XodusMessageType::XstoreQueryProductsResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XstoreAcquireLicenseRequest) => {
                let _req = XStoreAcquireLicenseRequest::decode(&msg.payload[..]).ok();
                let resp = XStoreAcquireLicenseResponse {
                    status: 0,
                    is_licensed: true,
                    license_blob: vec![0x4C, 0x49, 0x43, 0x53],
                };
                (
                    XodusMessageType::XstoreAcquireLicenseResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XstoreQueryLicenseTokenRequest) => {
                let req = xodus::proto::xodus::XStoreQueryLicenseTokenRequest::decode(&msg.payload[..]).ok();
                let mut token_str = "MOCK_LIC_TOKEN".to_string();
                if let Some(r) = req {
                    let mut prod_id = r.product_id.clone();
                    if prod_id.len() >= 15 && prod_id.chars().all(char::is_numeric) {
                        log::warn!("[XODUS-SERVICE] Game passed an XUID-like numeric ID as Product ID ('{}'). Overriding with standard Sea of Thieves Store ID '9P2N57MC619K'", prod_id);
                        prod_id = "9P2N57MC619K".to_string();
                    }

                    log::info!("[XODUS-SERVICE] Requesting Store License Token for user {} / product {}", r.user_id, prod_id);
                    match crate::connection::license_helper::get_store_license_token(&context.client, context.tokens(), prod_id).await {
                        Ok(t) => {
                            log::info!("[XODUS-SERVICE] Successfully fetched license token!");
                            token_str = t;
                        }
                        Err(e) => {
                            log::error!("[XODUS-SERVICE] Failed to get license token: {e}");
                        }
                    }
                }
                let resp = xodus::proto::xodus::XStoreQueryLicenseTokenResponse {
                    status: 0,
                    license_token: token_str,
                };
                (
                    XodusMessageType::XstoreQueryLicenseTokenResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XgamesaveReadBlobRequest) => {
                let req = XGameSaveReadBlobRequest::decode(&msg.payload[..]).ok();
                let mut data = Vec::new();
                if let Some(r) = req {
                    let path =
                        get_save_blob_path(&r.scid, r.user_id, &r.container_name, &r.blob_name);
                    if let Ok(content) = tokio::fs::read(&path).await {
                        data = content;
                    } else {
                        // Trigger background pull from cloud if missing locally
                        let tokens = context.tokens().clone();
                        let xuid = "2533274839201029".to_string();
                        crate::sync::sync_container_down(
                            tokens,
                            r.scid.clone(),
                            xuid,
                            r.user_id,
                            r.container_name.clone(),
                        ).await;
                    }
                }
                let resp = XGameSaveReadBlobResponse { status: 0, data };
                (
                    XodusMessageType::XgamesaveReadBlobResponse,
                    resp.encode_to_vec(),
                )
            }

            Ok(XodusMessageType::XgamesaveWriteBlobRequest) => {
                let req = XGameSaveWriteBlobRequest::decode(&msg.payload[..]).ok();
                let mut status = 0u32;
                if let Some(r) = req {
                    let path =
                        get_save_blob_path(&r.scid, r.user_id, &r.container_name, &r.blob_name);
                    if let Some(parent) = path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    if let Err(e) = tokio::fs::write(&path, &r.data).await {
                        log::error!("Failed to write save blob to {path:?}: {e}");
                        status = 0x80070005; // E_ACCESSDENIED / error
                    } else {
                        // Trigger asynchronous cloud upload
                        let tokens = context.tokens().clone();
                        let xuid = "2533274839201029".to_string();
                        crate::sync::sync_blob_up(
                            tokens,
                            r.scid.clone(),
                            xuid,
                            r.container_name.clone(),
                            r.blob_name.clone(),
                            r.data.clone(),
                        ).await;
                    }
                }
                let resp = XGameSaveWriteBlobResponse { status };
                (
                    XodusMessageType::XgamesaveWriteBlobResponse,
                    resp.encode_to_vec(),
                )
            }


            Ok(XodusMessageType::XgamesaveEnumerateBlobsRequest) => {
                let req = XGameSaveEnumerateBlobsRequest::decode(&msg.payload[..]).ok();
                let mut blobs = Vec::new();
                if let Some(r) = req {
                    let dir = get_container_path(&r.scid, r.user_id, &r.container_name);
                    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            if let Ok(meta) = entry.metadata().await {
                                if meta.is_file() {
                                    if let Some(name) = entry.file_name().to_str() {
                                        blobs.push(XGameSaveBlobInfo {
                                            name: name.to_string(),
                                            size: meta.len() as u32,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                let resp = XGameSaveEnumerateBlobsResponse { status: 0, blobs };
                (
                    XodusMessageType::XgamesaveEnumerateBlobsResponse,
                    resp.encode_to_vec(),
                )
            }

            _ => {

                log::warn!("Unhandled message type: {}", msg.msg_type);
                return Ok(());
            }
        };

        let response_msg = XodusMessage {
            msg_type: resp_type as i32,
            request_id,
            payload: resp_payload,
        };

        let encoded = response_msg.encode_to_vec();
        let resp_len = (encoded.len() as u32).to_le_bytes();

        socket.write_all(&resp_len).await?;
        socket.write_all(&encoded).await?;
        socket.flush().await?;
        Ok(())
}

fn get_save_blob_path(scid: &str, user_id: u64, container: &str, blob: &str) -> std::path::PathBuf {
    get_container_path(scid, user_id, container).join(blob)
}

fn get_container_path(scid: &str, user_id: u64, container: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join(".local/share/xodus/saves")
        .join(scid)
        .join(user_id.to_string())
        .join(container)
}



