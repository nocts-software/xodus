use crate::models::xbox::{
    UserAuthProperties, UserAuthRequest, XstsPropertyBag, XstsRequest, XstsResponse,
};

pub async fn authenticate_xbox_user(
    client: &reqwest::Client,
    rps_ticket: String,
) -> reqwest::Result<XstsResponse> {
    let start = std::time::Instant::now();
    log::info!("[MS-AUTH] POST https://user.auth.xboxlive.com/user/authenticate (RPS Ticket length: {})", rps_ticket.len());
    let body = UserAuthRequest {
        relying_party: "http://auth.xboxlive.com".to_string(),
        token_type: "JWT".to_string(),
        properties: UserAuthProperties {
            auth_method: "RPS".to_string(),
            site_name: "user.auth.xboxlive.com".to_string(),
            rps_ticket,
        },
    };

    let resp = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&body)
        .send()
        .await?;

    let elapsed = start.elapsed();
    let status = resp.status();
    log::info!("[MS-AUTH] user.auth.xboxlive.com responded: HTTP {} in {:.2?}", status, elapsed);
    let resp = resp.error_for_status()?;
    let parsed: XstsResponse = resp.json().await?;
    log::info!("[MS-AUTH] user.auth.xboxlive.com success: UHS={:?}, NotAfter={}", parsed.user_hash(), parsed.not_after);
    Ok(parsed)
}

pub async fn request_xsts_token(
    client: &reqwest::Client,
    token: String,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    log::info!("[MS-AUTH] POST https://xsts.auth.xboxlive.com/xsts/authorize (Single-Token, RP: '{}')", relying_party);
    let body = XstsRequest {
        relying_party: Some(relying_party.to_string()),
        token_type: Some("JWT".to_string()),
        properties: XstsPropertyBag {
            user_tokens: Some(vec![token]),
            sandbox_id: Some("RETAIL".to_string()),
            delegation_token: None,
            service_token: None,
            device_token: None,
            title_token: None,
        },
    };

    let resp = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&body)
        .send()
        .await?;

    let elapsed = start.elapsed();
    let status = resp.status();
    let text = resp.text().await?;
    log::info!("[MS-AUTH] xsts.auth.xboxlive.com responded: HTTP {} in {:.2?} (Body length: {} bytes)", status, elapsed, text.len());
    if !status.is_success() {
        log::warn!("[XSTS AUTH] Authorization failed for RP '{relying_party}': HTTP {status} - {text}");
        return Err(format!("XSTS HTTP {status}: {text}").into());
    }

    let xsts: XstsResponse = serde_json::from_str(&text).map_err(|e| {
        log::error!("[XSTS AUTH] Failed to parse XSTS JSON: {e} (Raw: {text})");
        format!("XSTS JSON error: {e}")
    })?;
    log::info!("[MS-AUTH] XSTS Authorized successfully: UHS={:?}, Expiration={}", xsts.user_hash(), xsts.not_after);
    Ok(xsts)
}

pub async fn request_xsts_token_with_claims(
    client: &reqwest::Client,
    user_token: Option<String>,
    device_token: Option<String>,
    title_token: Option<String>,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    log::info!("[MS-AUTH] POST https://xsts.auth.xboxlive.com/xsts/authorize (Multi-Claim, User: {}, Device: {}, Title: {}, RP: '{}')",
        user_token.is_some(), device_token.is_some(), title_token.is_some(), relying_party);
    let body = XstsRequest {
        relying_party: Some(relying_party.to_string()),
        token_type: Some("JWT".to_string()),
        properties: XstsPropertyBag {
            user_tokens: user_token.map(|u| vec![u]),
            sandbox_id: Some("RETAIL".to_string()),
            delegation_token: None,
            service_token: None,
            device_token,
            title_token,
        },
    };

    let resp = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&body)
        .send()
        .await?;

    let elapsed = start.elapsed();
    let status = resp.status();
    let text = resp.text().await?;
    log::info!("[MS-AUTH] xsts.auth.xboxlive.com multi-claim responded: HTTP {} in {:.2?} (Body length: {} bytes)", status, elapsed, text.len());
    if !status.is_success() {
        log::warn!("[XSTS AUTH] Multi-claim Authorization failed for RP '{relying_party}': HTTP {status} - {text}");
        return Err(format!("XSTS HTTP {status}: {text}").into());
    }

    let xsts: XstsResponse = serde_json::from_str(&text).map_err(|e| {
        log::error!("[XSTS AUTH] Failed to parse multi-claim XSTS JSON: {e} (Raw: {text})");
        format!("XSTS JSON error: {e}")
    })?;
    log::info!("[MS-AUTH] Multi-claim XSTS Authorized successfully: UHS={:?}, Expiration={}", xsts.user_hash(), xsts.not_after);
    Ok(xsts)
}

pub fn sign_request(
    signer: &xal::RequestSigner,
    method: &str,
    path_and_query: &str,
    auth_header: &str,
    body: &[u8],
) -> Option<String> {
    use p256::ecdsa::SigningKey;
    use p256::ecdsa::signature::hazmat::PrehashSigner;
    signer.sign_raw_to_string(
        1,
        chrono::Utc::now(),
        method,
        path_and_query,
        auth_header,
        body,
        8192,
    ).ok()
}

pub fn sign_http_request(method: &str, raw_url: &str, auth_header: &str, body: &[u8]) -> Option<String> {
    let path_and_query = if let Ok(url) = reqwest::Url::parse(raw_url) {
        match url.query() {
            Some(q) => format!("{}?{}", url.path(), q),
            None => url.path().to_string(),
        }
    } else {
        raw_url.to_string()
    };

    let auth = xal::XalAuthenticator::new(
        xal::XalAppParameters {
            client_id: "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
            title_id: None,
            auth_scopes: vec![],
            redirect_uri: None,
            client_secret: None,
        },
        xal::client_params::CLIENT_WINDOWS(),
        "RETAIL".to_string(),
    );
    let signer = auth.request_signer();
    sign_request(&signer, method, &path_and_query, auth_header, body)
}

pub fn get_xsts_auth_header(xsts: XstsResponse) -> String {
    let uhs = xsts.user_hash().expect("XSTS response missing xui claim");
    format!("XBL3.0 x={uhs};{}", xsts.token)
}
