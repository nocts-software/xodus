use crate::models::xbox::{
    UserAuthProperties, UserAuthRequest, XstsPropertyBag, XstsRequest, XstsResponse,
};

pub async fn authenticate_xbox_user(
    client: &reqwest::Client,
    rps_ticket: String,
) -> reqwest::Result<XstsResponse> {
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
        .await?
        .error_for_status()?;

    resp.json().await
}

pub async fn request_xsts_token(
    client: &reqwest::Client,
    token: String,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
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

    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        log::warn!("[XSTS AUTH] Authorization failed for RP '{relying_party}': HTTP {status} - {text}");
        return Err(format!("XSTS HTTP {status}: {text}").into());
    }

    let xsts: XstsResponse = serde_json::from_str(&text).map_err(|e| {
        log::error!("[XSTS AUTH] Failed to parse XSTS JSON: {e}");
        format!("XSTS JSON error: {e}")
    })?;
    Ok(xsts)
}

pub async fn request_xsts_token_with_claims(
    client: &reqwest::Client,
    user_token: Option<String>,
    device_token: Option<String>,
    title_token: Option<String>,
    relying_party: &str,
) -> Result<XstsResponse, Box<dyn std::error::Error + Send + Sync>> {
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

    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        log::warn!("[XSTS AUTH] Authorization failed for RP '{relying_party}': HTTP {status} - {text}");
        return Err(format!("XSTS HTTP {status}: {text}").into());
    }

    let xsts: XstsResponse = serde_json::from_str(&text).map_err(|e| {
        log::error!("[XSTS AUTH] Failed to parse XSTS JSON: {e}");
        format!("XSTS JSON error: {e}")
    })?;
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
    use base64::Engine;

    let signing_policy_version: i32 = 1;
    let version_bytes = signing_policy_version.to_be_bytes();
    let now = chrono::Utc::now();
    let filetime_val = (now.timestamp() + 11644473600) * 10000000 + (now.timestamp_subsec_nanos() as i64 / 100);
    let filetime_bytes = filetime_val.to_be_bytes();

    let prehash = xal::RequestSigner::prehash_message_data(
        &version_bytes,
        &filetime_bytes,
        method,
        path_and_query,
        auth_header,
        body,
        0,
    );

    let signing_key: SigningKey = signer.keypair.clone().into();
    let signature: p256::ecdsa::Signature = signing_key.sign_prehash(&prehash).ok()?;

    let mut sig_bytes = Vec::new();
    sig_bytes.extend_from_slice(&version_bytes);
    sig_bytes.extend_from_slice(&filetime_bytes);
    sig_bytes.extend_from_slice(&signature.to_bytes());
    Some(base64::engine::general_purpose::STANDARD.encode(&sig_bytes))
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
