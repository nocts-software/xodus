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

