use xodus::models::secrets::Token;
use xodus::models::live::ExchangeUserTokenOutcome;
use xodus::models::soap;
use xodus::tokens::TokenManager;

pub async fn get_store_license_token(
    client: &reqwest::Client,
    tokens: &TokenManager,
    content_id: String,
) -> Result<String, String> {
    let dev_token = tokens.get_device_sts_token().unwrap();
    let Token::Legacy(dev_token) = dev_token else {
        return Err("Invalid STS token".to_string());
    };
    let user = tokens.get_user().unwrap();
    let user_token = tokens.get_user_sts_token().unwrap();
    let Token::Legacy(legacy) = user_token else {
        return Err("Unsupported user token".to_string());
    };

    let ms_device_token = xodus::api::live::exchange_device_token(
        client,
        dev_token.clone(),
        "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
        "www.microsoft.com".to_owned(),
        Some(soap::PolicyReference::mbi_ssl()),
    )
    .await
    .map_err(|e| format!("Exchange device token error: {e}"))?;

    let user_token = xodus::api::live::exchange_user_token(
        client,
        legacy,
        user.username,
        dev_token,
        None,
        Some("Silent".to_string()),
        "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
        &[(
            "www.microsoft.com".to_owned(),
            Some(soap::PolicyReference::mbi_ssl()),
        )],
    )
    .await
    .map_err(|e| format!("Exchange user token error: {e}"))?;

    let ms_device_token: Token = ms_device_token.into();
    let Token::Compact(ms_device_token) = ms_device_token else {
        return Err("Unsupported device token format".to_string());
    };

    let user_token: Token = match user_token {
        ExchangeUserTokenOutcome::Fault(f) => return Err(format!("Fault: {:?}", f)),
        ExchangeUserTokenOutcome::Issued(
            soap::BodyContent::RequestSecurityTokenResponseCollection(mut collection),
        ) => {
            let token = collection.security_tokens.remove(0);
            token.into()
        }
        ExchangeUserTokenOutcome::Issued(soap::BodyContent::RequestSecurityTokenResponse(
            token,
        )) => (*token).into(),
        _ => return Err("Only responses are handled".to_string()),
    };
    let Token::Compact(user_token) = user_token else {
        return Err("Unsupported user token format".to_string());
    };

    let (response, _game_license) = xodus::licensing::content::get_license_content_ex(
        client,
        ms_device_token,
        user_token,
        user.puid,
        content_id.clone(),
        "US".to_string(),
        false,
    )
    .await
    .map_err(|e| format!("get_license_content error: {e}"))?;

    if let Some(lease) = response.license.leases.first() {
        log::info!("[MS-LICENSING] Returning authentic store license token (length: {})", lease.value.len());
        return Ok(lease.value.clone());
    }

    if let Some(key) = response.license.keys.first() {
        log::info!("[MS-LICENSING] Returning key token as fallback (length: {})", key.value.len());
        return Ok(key.value.clone());
    }

    Err("No valid license or lease returned from Microsoft licensing service".to_string())
}
