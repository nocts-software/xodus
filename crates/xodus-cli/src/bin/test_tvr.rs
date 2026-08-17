use xodus::xal::{XalAuthenticator, app_params, client_params};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut auth = XalAuthenticator::new(
        app_params::APP_GAMEPASS_BETA(),
        client_params::CLIENT_WINDOWS(),
        "RETAIL".to_string()
    );

    let token_path = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".local/share/xodus/xodus_state.json");
    let token_store = xodus::xal::TokenStore::load_from_file(token_path.to_str().unwrap()).expect("Failed to load tokens");

    // Acquire Title token for SoT
    let dt = auth.get_device_token().await?;
    
    // Since xal-rs get_title_token_win takes device_token.token as string
    let title_token = auth.get_title_token_win(&dt.token, 1717113201).await?;
    println!("Title Token Claims: {:?}", title_token.display_claims);

    // XSTS token for Athena
    let xsts_token = auth.get_xsts_token(
        Some(&dt),
        Some(&title_token),
        token_store.user_token.as_ref(),
        "rp://athena.prod.msrareservices.com/"
    ).await?;

    println!("XSTS Token Claims: {:?}", xsts_token.display_claims);

    Ok(())
}
