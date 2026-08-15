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
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut auth = xal::XalAuthenticator::new(
        xal::XalAppParameters {
            client_id: "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
            title_id: Some(title_id.to_string()),
            auth_scopes: vec![],
            redirect_uri: None,
            client_secret: None,
        },
        xal::client_params::CLIENT_WINDOWS(),
        "RETAIL".to_string(),
    );

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
    let dt = auth.get_device_token_rps(compact_dev).await?;
    let title_tok = auth.get_title_token_win(&dt.token, title_id.into()).await?;

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

    let xal_user = xal::response::UserToken {
        issue_instant: chrono::Utc::now(),
        not_after: resp.not_after,
        token: resp.token.clone(),
        display_claims: None,
    };

    let xsts_token = auth.get_xsts_token(
        Some(&dt),
        Some(&title_tok),
        Some(&xal_user),
        relying_party,
    ).await?;

    let display_claims = xsts_token.display_claims.map(|dc| crate::models::xbox::DisplayClaims {
        xui: dc.xui.into_iter().map(|map| crate::models::xbox::XuiClaim {
            uhs: map.get("uhs").cloned().unwrap_or_default(),
            gtg: map.get("gtg").cloned(),
            xid: map.get("xid").cloned(),
            mgt: map.get("mgt").cloned(),
            agg: map.get("agg").cloned(),
        }).collect(),
        xti: vec![crate::models::xbox::XtiClaim {
            tid: Some(title_id.to_string()),
        }],
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
        }],
        xdi: None,
    });

    let xsts_resp = XstsResponse {
        not_after: xsts_token.not_after,
        token: xsts_token.token,
        display_claims,
    };

    Ok(xsts_resp)
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
    let xsts = run_with_title(client, dev_token, user_token, title_id, relying_party).await?;
    tokens.cache_xsts(&cache_key, &xsts);
    Ok(xsts)
}

