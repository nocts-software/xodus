use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio_util::sync::CancellationToken;
use xodus::tokens::TokenManager;

use xodus_service::connection;
use xodus_service::utils;


#[tokio::main]
async fn main() {
    xodus::secrets::init_secrets().expect("Failed to init keychain");
    let tokens = Arc::new(TokenManager::with_keychain_and_memory());
    let init_client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default();
    xodus::tokens::device::ensure_device_credentials(&init_client, &tokens).await;
    let xodus::models::secrets::Token::Legacy(device_token) =
        tokens.get_device_sts_token().unwrap()
    else {
        panic!("Device token isnt legacy")
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,xodus=debug,xodus_service=debug")).init();
    let runtime_dir = utils::get_runtime_dir();
    let cancellation = CancellationToken::new();
    let socket_path = format!("{runtime_dir}/xodus.sock");
    let trigger = cancellation.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failure to handle ctrl_c");
        trigger.cancel();
    });
    {
        let _ = tokio::fs::remove_file(&socket_path).await;
        let _ = tokio::fs::remove_file("/tmp/xodus.sock").await;
        let listener = UnixListener::bind(&socket_path).expect("Unable to bind to socket");
        log::info!("[XODUS-SERVICE] Bound successfully to Unix socket at {socket_path}");
        let mode = 0o600;
        let perms = Permissions::from_mode(mode);
        _ = tokio::fs::set_permissions(&socket_path, perms).await;
        if socket_path != "/tmp/xodus.sock" {
            let _ = tokio::fs::symlink(&socket_path, "/tmp/xodus.sock").await;
        }
        loop {
            let accept = tokio::select! {
                r = listener.accept() => r,
                _ = cancellation.cancelled() => break,
            }
            .expect("Failed to accept");

            let token = cancellation.clone();
            let device_token = device_token.clone();
            let tokens = tokens.clone();
            tokio::spawn(async move {
                connection::router::route(accept.0, token, device_token, tokens).await
            });
        }
    }

    _ = tokio::fs::remove_file(socket_path).await;
    _ = tokio::fs::remove_file("/tmp/xodus.sock").await;
}
