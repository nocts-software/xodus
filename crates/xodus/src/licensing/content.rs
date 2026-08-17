use std::collections::HashMap;

use base64::prelude::*;
use xal::cvlib::CorrelationVector;
//use xal::extensions::CorrelationVectorReqwestBuilder;

use crate::licensing::utils;
use crate::models::devicecredential::License;
use crate::models::licensing::{
    DeviceContext, LicenseContentRequest, LicenseContentResponse, LicenseUserIdentity,
};

// we might need a bump in xal-rs concerning reqwest,
// that might block us from using the correlationvector extension
pub async fn get_license_content(
    client: &reqwest::Client,
    device_ms_token: String,
    user_ms_token: String,
    ticket_reference: String,
    content_id: String,
    market: String,
) -> Result<(LicenseContentResponse, License), Box<dyn std::error::Error + Send + Sync>> {
    let (res, lic) = get_license_content_ex(client, device_ms_token, user_ms_token, ticket_reference, content_id, market, true).await?;
    let lic = lic.ok_or_else(|| "No license XML found in response".to_string())?;
    Ok((res, lic))
}

pub async fn get_license_content_ex(
    client: &reqwest::Client,
    device_ms_token: String,
    user_ms_token: String,
    ticket_reference: String,
    content_id: String,
    market: String,
    key_only: bool,
) -> Result<(LicenseContentResponse, Option<License>), Box<dyn std::error::Error + Send + Sync>> {
    let cv = CorrelationVector::new();
    let start = std::time::Instant::now();
    log::info!("[MS-LICENSING] POST https://licensing.mp.microsoft.com/v7.0/licenses/content for content_id='{}', market='{}', key_only={}", content_id, market, key_only);
    let response = client
        .post("https://licensing.mp.microsoft.com/v7.0/licenses/content")
        .header("from", "XboxLicenseManager")
        .header("Authorization", device_ms_token)
        .header("user-agent", "XboxLm-PC/Microsoft.GamingServices_32.107.4002.0_x64__8wekyb3d8bbwe")
        .header("MS-CV", cv.to_string())
        .json(&LicenseContentRequest {
            content_id: content_id.clone(),
            market,
            client_challenge: "PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0idXRmLTgiID8+PENsaWVudENoYWxsZW5nZSB4bWxuczp4c2k9Imh0dHA6Ly93d3cudzMub3JnLzIwMDEvWE1MU2NoZW1hLWluc3RhbmNlIiB4bWxuczp4c2Q9Imh0dHA6Ly93d3cudzMub3JnLzIwMDEvWE1MU2NoZW1hIiB4bWxucz0iaHR0cDovL3NjaGVtYXMubWljcm9zb2Z0LmNvbS9vbmVzdG9yZS9zZWN1cml0eS9ta21zL0xpY1JlcS92MSIgVmVyc2lvbj0iMiI+PExpY2Vuc2VQcm90b2NvbFZlcnNpb24+NTwvTGljZW5zZVByb3RvY29sVmVyc2lvbj48U2lnbmluZ0tleVZlcnNpb24+MTwvU2lnbmluZ0tleVZlcnNpb24+PENsaWVudFZlcnNpb24+MjwvQ2xpZW50VmVyc2lvbj48L0NsaWVudENoYWxsZW5nZT4=".into(),
            concurrency_mode: "Rude".into(),
            license_version: 4,
            need_key: true,
            key_only,
            device_context: DeviceContext::default(),
            users: HashMap::from_iter(
                [(utils::generate_suid(),
                vec![LicenseUserIdentity {
                    identity_type: "Msa".to_string(),
                    identity_value: user_ms_token,
                    local_ticket_reference: ticket_reference,
                }])],
            ),
        })
        .send()
        .await?;

    let elapsed = start.elapsed();
    let status = response.status();
    log::info!("[MS-LICENSING] licensing.mp.microsoft.com responded: HTTP {} in {:.2?}", status, elapsed);
    if !status.is_success() {
        let err_body = response.text().await.unwrap_or_default();
        log::error!("[MS-LICENSING] Licensing error response: HTTP {} - {}", status, err_body);
        return Err(format!("Microsoft Licensing Service denied access ({status}): {err_body}").into());
    }

    let content_res = response.json::<LicenseContentResponse>().await?;
    log::info!("[MS-LICENSING] License content response received: {} keys, {} leases in bundle",
        content_res.license.keys.len(), content_res.license.leases.len());

    let license = if let Some(key) = content_res.license.keys.first() {
        let license_b64 = &key.value;
        let decoded_bytes = BASE64_STANDARD.decode(license_b64)
            .map_err(|e| format!("Failed to decode license base64: {e}"))?;
        let xml_str = String::from_utf8(decoded_bytes)
            .map_err(|e| format!("Failed to parse license UTF-8: {e}"))?;
        Some(quick_xml::de::from_str::<License>(&xml_str)
            .map_err(|e| format!("Failed to deserialize license XML: {e}"))?)
    } else {
        None
    };

    Ok((content_res, license))
}
