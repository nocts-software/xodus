use crate::models::live::ExchangeUserTokenOutcome;
use crate::models::secrets::{LegacyToken, Token};
use crate::models::soap;
use crate::models::xbox::XstsResponse;

pub mod auth;
pub mod collections;
pub mod mpsd;
pub mod profile;
pub mod social;
pub mod title;
pub mod titlestorage;
pub use auth::{authenticate_xbox_user, get_xsts_auth_header, request_xsts_token};
pub use collections::{
    check_user_gamepass_subscription, enrich_products_catalog, get_gamepass_catalog_ids,
    get_gamepass_sigl_ids, get_user_collections, get_user_owned_catalog_items, CollectionItem,
    GameCatalogItem, DEFAULT_COVER_URL,
};


pub use mpsd::{MatchmakingTicketRequest, MatchmakingTicketResponse, MpsdClient, MultiplayerSession, SessionMember, SessionReference};
pub use profile::{get_user_profile, ProfileResponse, ProfileUser, UserProfile};
pub use social::{PeopleHubResponse, Person, PresenceDetail, SocialClient};
pub use titlestorage::{TitleStorageBlobList, TitleStorageBlobMetadata, TitleStorageClient};





pub async fn run(
    client: &reqwest::Client,
    dev_token: LegacyToken,
    legacy: LegacyToken,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    let user_token = crate::api::live::exchange_user_token(
        client,
        legacy,
        "USERNAME".to_string(),
        dev_token,
        None,
        Some("Silent".to_string()),
        "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
        &[(
            "user.auth.xboxlive.com".to_owned(),
            Some(soap::PolicyReference::mbi_ssl()),
        )],
    )
    .await?;

    let user_token: Token = match user_token {
        ExchangeUserTokenOutcome::Fault(f) => {
            return Err(format!("Failed to get exchange MS token: {f:?}").into());
        }
        ExchangeUserTokenOutcome::Issued(
            soap::BodyContent::RequestSecurityTokenResponseCollection(mut collection),
        ) => {
            let token = collection.security_tokens.remove(0);
            token.into()
        }
        ExchangeUserTokenOutcome::Issued(soap::BodyContent::RequestSecurityTokenResponse(
            token,
        )) => (*token).into(),
        _ => return Err("Only responses are handled".into()),
    };
    let Token::Compact(user_token) = user_token else {
        return Err("Unsupported token".into());
    };
    let resp = authenticate_xbox_user(client, user_token).await?;

    let xsts = request_xsts_token(client, resp.token, relying_party).await?;
    Ok(xsts)
}

pub async fn run_with_title(
    client: &reqwest::Client,
    dev_token: LegacyToken,
    legacy: LegacyToken,
    title_id: u32,
    relying_party: &str,
    signer_opt: Option<xal::RequestSigner>,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut auth = if let Some(signer) = signer_opt {
        xal::XalAuthenticator::with_signer(
            xal::XalAppParameters {
                client_id: "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
                title_id: Some(title_id.to_string()),
                auth_scopes: vec![],
                redirect_uri: None,
                client_secret: None,
            },
            xal::client_params::CLIENT_WINDOWS(),
            "RETAIL".to_string(),
            signer,
        )
    } else {
        xal::XalAuthenticator::new(
            xal::XalAppParameters {
                client_id: "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
                title_id: Some(title_id.to_string()),
                auth_scopes: vec![],
                redirect_uri: None,
                client_secret: None,
            },
            xal::client_params::CLIENT_WINDOWS(),
            "RETAIL".to_string(),
        )
    };

    log::info!("[MS-AUTH] Starting run_with_title: TitleID={}, RP='{}'", title_id, relying_party);

    let device_token_resp = crate::api::live::exchange_device_token(
        client,
        dev_token.clone(),
        "{28C08266-F973-4AE6-FFE4-409B249F138F}".to_string(),
        "scope=service::user.auth.xboxlive.com::MBI_SSL&api-version=2.0".to_owned(),
        Some(crate::models::soap::PolicyReference::token_broker()),
    )
    .await?;
    let Token::Compact(compact_dev) = device_token_resp.into() else {
        return Err("Device token compact err".into());
    };
    log::info!("[MS-AUTH] Successfully exchanged device token via login.live.com (length: {})", compact_dev.len());

    let dt = auth.get_device_token_rps(compact_dev).await?;
    log::info!("[MS-AUTH] Successfully obtained XBL Device Token from device.auth.xboxlive.com (expires: {})", dt.not_after);

    // Read game version from env (set by xodus-cli from AppxManifest.xml).
    // This causes Microsoft's title.auth.xboxlive.com to embed the TVR (Title Version Record)
    // into the title token's xti.tvr claim, which is required by Athena Ares login.
    let game_version = std::env::var("XODUS_GAME_VERSION").ok();
    let game_version_ref = game_version.as_deref();
    let package_family_name = std::env::var("XODUS_PACKAGE_FAMILY_NAME").ok();
    let pfn_ref = package_family_name.as_deref();
    log::info!("[MS-AUTH] Using TitleVersion for TVR: {:?}, PackageFamilyName: {:?}", game_version_ref, pfn_ref);

    let title_tok_opt: Option<xal::response::TitleToken> = match auth.get_title_token_win_versioned(&dt.token, title_id.into(), game_version_ref, pfn_ref).await {
        Ok(tok) => {
            log::info!("[MS-AUTH] Successfully obtained versioned Title Token (expires: {})", tok.not_after);
            Some(tok)
        }
        Err(err) => {
            log::warn!("[MS-AUTH] Versioned title token failed ({err}), retrying basic title token...");
            match auth.get_title_token_win(&dt.token, title_id.into()).await {
                Ok(tok) => {
                    log::info!("[MS-AUTH] Successfully obtained basic Title Token (expires: {})", tok.not_after);
                    Some(tok)
                }
                Err(err2) => {
                    log::warn!("[MS-AUTH] Basic title token failed ({err2}), proceeding with Device+User token auth...");
                    None
                }
            }
        }
    };

    let user_token = crate::api::live::exchange_user_token(
        client,
        legacy,
        "USERNAME".to_string(),
        dev_token,
        None,
        Some("Silent".to_string()),
        "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
        &[(
            "user.auth.xboxlive.com".to_owned(),
            Some(soap::PolicyReference::mbi_ssl()),
        )],
    )
    .await?;

    let user_token: Token = match user_token {
        ExchangeUserTokenOutcome::Fault(f) => {
            log::error!("[MS-AUTH] User token exchange returned SOAP Fault: {f:?}");
            return Err(format!("Failed to get exchange MS token: {f:?}").into());
        }
        ExchangeUserTokenOutcome::Issued(
            soap::BodyContent::RequestSecurityTokenResponseCollection(mut collection),
        ) => {
            let token = collection.security_tokens.remove(0);
            token.into()
        }
        ExchangeUserTokenOutcome::Issued(soap::BodyContent::RequestSecurityTokenResponse(
            token,
        )) => (*token).into(),
        _ => return Err("Only responses are handled".into()),
    };
    let Token::Compact(user_token) = user_token else {
        return Err("Unsupported token".into());
    };
    log::info!("[MS-AUTH] Successfully exchanged user token via login.live.com");

    let resp = authenticate_xbox_user(client, user_token).await?;
    log::info!("[MS-AUTH] Successfully authenticated user at user.auth.xboxlive.com (UHS: {:?}, expires: {})", resp.user_hash(), resp.not_after);

    let xal_user = xal::response::UserToken {
        issue_instant: chrono::Utc::now(),
        not_after: resp.not_after,
        token: resp.token.clone(),
        display_claims: None,
    };

    log::info!("[MS-AUTH] Requesting multi-claim XSTS token from xsts.auth.xboxlive.com for RP '{}' (Sandbox: RETAIL, Device+Title+User)...", relying_party);
    let xsts_token = match auth.get_xsts_token(
        Some(&dt),
        title_tok_opt.as_ref(),
        Some(&xal_user),
        relying_party,
    ).await {
        Ok(tok) => {
            log::info!("[MS-AUTH] XSTS token issued successfully by xsts.auth.xboxlive.com (expires: {}, token len: {})", tok.not_after, tok.token.len());
            tok
        }
        Err(err) => {
            log::error!("[MS-AUTH] XSTS authorization failed for RP '{}': {}", relying_party, err);
            return Err(err.into());
        }
    };

    let display_claims = xsts_token.display_claims.map(|dc| crate::models::xbox::DisplayClaims {
        xui: dc.xui.into_iter().map(|map| crate::models::xbox::XuiClaim {
            uhs: map.get("uhs").cloned().unwrap_or_default(),
            gtg: map.get("gtg").cloned(),
            xid: map.get("xid").cloned(),
            mgt: map.get("mgt").cloned(),
            agg: map.get("agg").cloned(),
        }).collect(),
        xti: if dc.xti.is_empty() {
            vec![crate::models::xbox::XtiClaim {
                tid: Some(title_id.to_string()),
                tvr: None,
            }]
        } else {
            dc.xti.into_iter().map(|map| crate::models::xbox::XtiClaim {
                tid: map.get("tid").cloned().or_else(|| Some(title_id.to_string())),
                tvr: map.get("tvr").cloned(),
            }).collect()
        },
        xdi: None,
    }).unwrap_or_else(|| crate::models::xbox::DisplayClaims {
        xui: vec![crate::models::xbox::XuiClaim {
            uhs: resp.user_hash().unwrap_or("0").to_string(),
            gtg: None,
            xid: None,
            mgt: None,
            agg: None,
        }],
        xti: vec![crate::models::xbox::XtiClaim {
            tid: Some(title_id.to_string()),
            tvr: None,
        }],
        xdi: None,
    });

    log::info!("[MS-AUTH] Formatted DisplayClaims: XUI count={}, XTI count={:?}", display_claims.xui.len(), display_claims.xti);

    let signer = auth.request_signer();
    if let Ok(mut lock) = SIGNER_CACHE.lock() {
        lock.insert(relying_party.to_string(), signer.clone());
        lock.insert(format!("{relying_party}#tid={title_id}"), signer);
    }

    let xsts_resp = XstsResponse {
        not_after: xsts_token.not_after,
        token: xsts_token.token,
        display_claims,
    };

    Ok(xsts_resp)
}

static SIGNER_CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, xal::RequestSigner>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

pub fn sign_request_for_rp(
    relying_party: &str,
    method: &str,
    raw_url: &str,
    auth_header: &str,
    body: &[u8],
) -> Option<String> {
    let signer = {
        let lock = SIGNER_CACHE.lock().ok()?;
        let trimmed_rp = relying_party.trim_end_matches('/');
        lock.get(relying_party)
            .or_else(|| lock.get(trimmed_rp))
            .cloned()
            .or_else(|| {
                lock.iter()
                    .find(|(k, _)| {
                        let k_trim = k.trim_end_matches('/');
                        trimmed_rp.contains(k_trim) || k_trim.contains(trimmed_rp)
                    })
                    .map(|(_, v)| v.clone())
            })
            .or_else(|| {
                lock.values().next().cloned()
            })
            .or_else(|| {
                let tokens = crate::tokens::TokenManager::with_keychain_and_memory();
                Some(tokens.get_or_create_device_signer())
            })?
    };
    let path_and_query = if let Ok(url) = reqwest::Url::parse(raw_url) {
        match url.query() {
            Some(q) => format!("{}?{}", url.path(), q),
            None => url.path().to_string(),
        }
    } else {
        raw_url.to_string()
    };
    crate::api::xbox::auth::sign_request(&signer, method, &path_and_query, auth_header, body)
}

pub async fn get_or_request_xsts(
    client: &reqwest::Client,
    tokens: &crate::tokens::TokenManager,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(cached) = tokens.get_cached_xsts(relying_party) {
        if cached.not_after > chrono::Utc::now() + chrono::Duration::minutes(5) {
            return Ok(cached);
        }
    }
    let Token::Legacy(dev_token) = tokens.get_device_sts_token()? else {
        return Err("Device token is not legacy".into());
    };
    let Token::Legacy(user_token) = tokens.get_user_sts_token()? else {
        return Err("User token is not legacy".into());
    };
    let xsts = run(client, dev_token, user_token, relying_party).await?;
    tokens.cache_xsts(relying_party, &xsts);
    Ok(xsts)
}

pub async fn get_or_request_xsts_for_title(
    client: &reqwest::Client,
    tokens: &crate::tokens::TokenManager,
    title_id: u32,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    let cache_key = format!("{relying_party}#tid={title_id}");
    if let Some(cached) = tokens.get_cached_xsts(&cache_key) {
        if cached.not_after > chrono::Utc::now() + chrono::Duration::minutes(5) {
            return Ok(cached);
        }
    }
    let Token::Legacy(dev_token) = tokens.get_device_sts_token()? else {
        return Err("Device token is not legacy".into());
    };
    let Token::Legacy(user_token) = tokens.get_user_sts_token()? else {
        return Err("User token is not legacy".into());
    };
    let signer = tokens.get_or_create_device_signer();
    let xsts = run_with_title(client, dev_token, user_token, title_id, relying_party, Some(signer)).await?;
    tokens.cache_xsts(&cache_key, &xsts);
    Ok(xsts)
}

