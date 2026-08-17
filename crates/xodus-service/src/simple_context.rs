// Temporary context, full service design will be much more extensive

use std::sync::Arc;

use xodus::models::secrets::LegacyToken;
use xodus::tokens::TokenManager;

pub struct SimpleContext {
    pub client: reqwest::Client,
    pub device_token: Option<LegacyToken>,
    tokens: Arc<TokenManager>,
}

impl SimpleContext {
    pub fn new(device_token: LegacyToken, tokens: Arc<TokenManager>) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(format!("xodus-service/{}", env!("CARGO_PKG_VERSION")))
            .danger_accept_invalid_certs(true)
            .connection_verbose(true)
            .build()
            .unwrap();

        Self {
            client,
            device_token: Some(device_token),
            tokens,
        }
    }

    pub fn mock() -> Self {
        let client = reqwest::Client::new();
        let tokens = Arc::new(TokenManager::with_keychain_and_memory());
        Self {
            client,
            device_token: None,
            tokens,
        }
    }

    pub fn tokens(&self) -> &Arc<TokenManager> {
        &self.tokens
    }
}

