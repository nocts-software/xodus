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

pub fn get_xsts_auth_header(xsts: XstsResponse) -> String {
    let uhs = xsts.user_hash().expect("XSTS response missing xui claim");
    format!("XBL3.0 x={uhs};{}", xsts.token)
}
