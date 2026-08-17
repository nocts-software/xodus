/// Standalone Erminebeard diagnostic.
/// Tests the TVR fix: passes TitleVersion + PackageFamilyName when building title token.
/// Usage: cargo run --bin diag_ares
use xodus::tokens::TokenManager;
use xodus::api::xbox::{get_or_request_xsts_for_title, get_or_request_xsts, sign_request_for_rp};
use base64::Engine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    xodus::secrets::init_secrets().expect("Unable to initialize credentials");

    // Set env vars to simulate what xodus-cli sets from AppxManifest.xml
    unsafe {
        std::env::set_var("XODUS_GAME_VERSION", "2.150.9409.0");
        std::env::set_var("XODUS_PACKAGE_FAMILY_NAME", "Microsoft.SeaofThieves_8wekyb3d8bbwe");
    }
    println!("[ENV] XODUS_GAME_VERSION=2.150.9409.0");
    println!("[ENV] XODUS_PACKAGE_FAMILY_NAME=Microsoft.SeaofThieves_8wekyb3d8bbwe");
    println!("[INFO] Title token display_claims will be logged via [MS-AUTH] Title Token lines\n");

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let tokens = TokenManager::with_keychain_and_memory();

    println!("=== DIAG: Athena Ares Erminebeard Diagnostic ===\n");

    // Step 1: Get XSTS multi-claim token (triggers title token request with TVR)
    println!("[Step 1] Requesting XSTS multi-claim token WITH TitleVersion...");
    println!("         (watch for '[MS-AUTH] Title Token display_claims.xti' in output)");
    let xsts = get_or_request_xsts_for_title(
        &client,
        &tokens,
        1717113201,
        "rp://athena.prod.msrareservices.com/",
    ).await;
    let xsts = match xsts {
        Err(e) => {
            eprintln!("[FAIL] Could not get Athena XSTS token: {e}");
            return Ok(());
        }
        Ok(tok) => {
            println!("[OK] Got XSTS token (len={})", tok.token.len());
            println!("     UHS: {}", tok.user_hash().unwrap_or("UNKNOWN"));
            let tvr_present = tok.display_claims.xti.iter().any(|x| x.tvr.is_some());
            if tvr_present {
                println!("[OK] TVR claim IS present in XSTS display_claims! Erminebeard should be resolved.");
                for xti in &tok.display_claims.xti {
                    println!("     xti: tid={:?} tvr={:?}", xti.tid, xti.tvr);
                }
            } else {
                println!("[WARN] TVR claim is MISSING from XSTS display_claims.");
                println!("       xti: {:?}", tok.display_claims.xti);
                println!("       The XSTS token is JWE-encrypted — TVR may be in encrypted body.");
            }
            tok
        }
    };

    let uhs = xsts.user_hash().unwrap_or("UNKNOWN").to_string();
    let auth_header = format!("XBL3.0 x={};{}", uhs, xsts.token);
    println!("     Auth header length: {} bytes\n", auth_header.len());

    // Step 2: Athena Discovery
    println!("[Step 2] Athena Discovery...");
    let discovery_url = "https://discovery.prod.athena.msrareservices.com/discovery/app/endpoint?tid=1717113201";
    let stamp_host = {
        let resp = client.get(discovery_url)
            .header("Authorization", &auth_header)
            .header("User-Agent", "Athena/2.150.9409.0 (WinGDK; Windows 10.0.19045.0)")
            .send().await;
        match resp {
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                println!("[Discovery] HTTP {}", status);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(url) = parsed.get("login").and_then(|l| l.get("url")).and_then(|u| u.as_str()) {
                        println!("[Discovery] login.url = {}", url);
                        if let Some(host_start) = url.find("://") {
                            let after = &url[host_start + 3..];
                            let host = after.split('/').next().unwrap_or(after);
                            println!("[Discovery] stamp host = {}", host);
                            host.to_string()
                        } else { "stamp3-fd.prod.athena.msrareservices.com".to_string() }
                    } else {
                        println!("[Discovery] No login.url: {}", &body[..body.len().min(200)]);
                        "stamp3-fd.prod.athena.msrareservices.com".to_string()
                    }
                } else {
                    println!("[Discovery] Non-JSON: {}", &body[..body.len().min(200)]);
                    "stamp3-fd.prod.athena.msrareservices.com".to_string()
                }
            }
            Err(e) => {
                eprintln!("[Discovery FAIL] {e}");
                "stamp3-fd.prod.athena.msrareservices.com".to_string()
            }
        }
    };

    // Step 3: Call Ares login
    println!("\n[Step 3] Testing Ares login...");
    let ares_url = format!("https://{}/ares/login/api/token/client", stamp_host);
    println!("[Ares URL] {}", ares_url);

    for (label, body) in &[
        ("Empty {}", serde_json::json!({})),
        ("version", serde_json::json!({"version": "2.150.9409.0"})),
    ] {
        let resp = client.post(&ares_url)
            .header("Authorization", &auth_header)
            .header("User-Agent", "Athena/2.150.9409.0 (WinGDK; Windows 10.0.19045.0)")
            .header("Content-Type", "application/json")
            .json(body)
            .send().await;
        match resp {
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                println!("[Ares {}] HTTP {} -> {}", label, status, text);
            }
            Err(e) => eprintln!("[Ares {} NET ERROR] {e}", label),
        }
    }

    // Step 4: With ECDSA signature (PoP)
    println!("\n[Step 4] Testing Ares login with ECDSA PoP signature...");
    let body_bytes = b"{}";
    let sig = sign_request_for_rp(
        "rp://athena.prod.msrareservices.com/",
        "POST",
        &ares_url,
        &auth_header,
        body_bytes,
    );
    match sig {
        Some(s) => {
            println!("[PoP] Signature (len={}): {}...", s.len(), &s[..s.len().min(30)]);
            let resp = client.post(&ares_url)
                .header("Authorization", &auth_header)
                .header("Signature", &s)
                .header("User-Agent", "Athena/2.150.9409.0 (WinGDK; Windows 10.0.19045.0)")
                .header("Content-Type", "application/json")
                .body("{}")
                .send().await;
            match resp {
                Ok(r) => {
                    let status = r.status();
                    let text = r.text().await.unwrap_or_default();
                    println!("[Ares with Signature] HTTP {} -> {}", status, text);
                }
                Err(e) => eprintln!("[Ares with Signature NET ERROR] {e}"),
            }
        }
        None => println!("[WARN] Could not generate ECDSA signature"),
    }

    // Step 5: Try to get XStoreQueryLicenseToken JWT from licensing.mp.microsoft.com/v8.0/licenseToken
    // This JWT (with tvr claim) is what Athena actually checks for Erminebeard
    println!("\n[Step 5] Requesting XStoreQueryLicenseToken JWT from Microsoft...");
    println!("         Trying multiple RPs to find the right auth for licensing.mp.microsoft.com");

    for rp in &["http://mp.microsoft.com/", "http://licensing.xboxlive.com/", "http://xboxlive.com", "rp://mp.microsoft.com/"] {
        let mp_xsts = xodus::api::xbox::get_or_request_xsts(
            &client,
            &tokens,
            rp,
        ).await;
        let mp_tok = match mp_xsts {
            Err(e) => {
                println!("[LicenseToken] XSTS for {} failed: {}", rp, e);
                continue;
            }
            Ok(t) => t,
        };
        let mp_uhs = mp_tok.user_hash().unwrap_or("UNKNOWN").to_string();
        let mp_auth = format!("XBL3.0 x={};{}", mp_uhs, mp_tok.token);
        println!("[LicenseToken] Testing RP='{}' XSTS (uhs={})...", rp, mp_uhs);

        let lic_req = serde_json::json!({
            "productIds": ["9P2N57MC619K"],
            "parentProductId": "9P2N57MC619K",
            "customDeveloperString": "xodus-diag-test-123",
            "enforceSellableBy": false
        });
        let resp = client.post("https://licensing.mp.microsoft.com/v8.0/licenseToken")
            .header("Authorization", &mp_auth)
            .header("Content-Type", "application/json")
            .header("User-Agent", "XboxLm-PC/Microsoft.GamingServices_32.107.4002.0_x64__8wekyb3d8bbwe")
            .json(&lic_req)
            .send().await;
        match resp {
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                println!("[LicenseToken RP={}] HTTP {} -> {}", rp, status, &body[..body.len().min(300)]);
                if status.is_success() {
                    let jwt = body.trim_matches('"').to_string();
                    let parts: Vec<&str> = jwt.split('.').collect();
                    if parts.len() >= 2 {
                        use base64::Engine;
                        if let Ok(decoded) = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(parts[1]) {
                            let payload = String::from_utf8_lossy(&decoded);
                            println!("[LicenseToken] JWT payload: {}", &payload.to_string()[..payload.len().min(500)]);
                            if payload.contains("tvr") {
                                println!("[LicenseToken] *** TVR IS PRESENT in license JWT! ***");
                            } else {
                                println!("[LicenseToken] TVR NOT found in license JWT payload");
                            }
                        }
                    }
                }
            }
            Err(e) => println!("[LicenseToken RP={}] Network error: {}", rp, e),
        }
    }
    println!("\n=== Diagnostic complete ===");
    Ok(())
}
