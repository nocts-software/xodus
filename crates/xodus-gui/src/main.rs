use std::sync::Arc;
use tao::{
    dpi::{LogicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};
use wry::WebViewBuilder;
use xodus::tokens::TokenManager;

const HTML: &str = include_str!("../ui/index.html");
const CSS: &str = include_str!("../ui/styles.css");
const ASSETS: &str = include_str!("../ui/assets.js");
const JS: &str = include_str!("../ui/app.js");

#[derive(Debug)]
enum CustomEvent {
    EvaluateScript(String),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env("XODUS_LOG");
    xodus::secrets::init_secrets().ok();

    let tokens = Arc::new(TokenManager::with_keychain_and_memory());
    let mut event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    
    let window = WindowBuilder::new()
        .with_title("noct's xodus gui")
        .with_decorations(false)
        .with_resizable(true)
        .with_inner_size(Size::Logical(LogicalSize::new(1280.0, 800.0)))
        .with_min_inner_size(Size::Logical(LogicalSize::new(960.0, 600.0)))
        .build(&event_loop)?;

    let window = Arc::new(window);
    let win_ipc = window.clone();

    let tokens_clone = tokens.clone();
    let combined_html = HTML
        .replace("<link rel=\"stylesheet\" href=\"styles.css\">", &format!("<style>{}</style>", CSS))
        .replace("<script src=\"app.js\"></script>", &format!("<script>{}</script><script>{}</script>", ASSETS, JS));


    let rt = tokio::runtime::Runtime::new()?;

    let proxy_ipc = proxy.clone();
    let tokens_ipc = tokens.clone();

    let builder = WebViewBuilder::new()
        .with_custom_protocol("xodus-file".into(), move |_id, request| {
            let uri = request.uri().to_string();
            let mut raw_path = uri
                .trim_start_matches("xodus-file://")
                .trim_start_matches("xodus-file:")
                .replace("%20", " ");
            while raw_path.starts_with('/') {
                raw_path.remove(0);
            }
            let full_path = format!("/{raw_path}");
            eprintln!("[xodus-file] Requested: {uri} -> {full_path}");
            if let Ok(bytes) = std::fs::read(&full_path) {
                let mime = if full_path.ends_with(".png") {
                    "image/png"
                } else if full_path.ends_with(".jpg") || full_path.ends_with(".jpeg") {
                    "image/jpeg"
                } else if full_path.ends_with(".svg") {
                    "image/svg+xml"
                } else {
                    "application/octet-stream"
                };
                wry::http::Response::builder()
                    .header("Content-Type", mime)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(bytes.into())
                    .unwrap()
            } else {
                eprintln!("[xodus-file] File not found: {full_path}");
                wry::http::Response::builder()
                    .status(404)
                    .body(Vec::new().into())
                    .unwrap()
            }
        })
        .with_html(&combined_html)

        .with_ipc_handler(move |req| {

            let body = req.body();
            log::info!("IPC Message: {body}");
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some(cmd) = v.get("cmd").and_then(|c| c.as_str()) {
                    match cmd {
                        "drag_window" => {
                            let _ = win_ipc.drag_window();
                        }
                        "minimize" => {
                            win_ipc.set_minimized(true);
                        }
                        "maximize" => {
                            let is_max = win_ipc.is_maximized();
                            win_ipc.set_maximized(!is_max);
                        }
                        "close" => {
                            std::process::exit(0);
                        }
                        "launch_game" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                rt.spawn(async move {
                                    let child = tokio::process::Command::new("xodus")
                                        .arg("run")
                                        .arg(&path_owned)
                                        .spawn();
                                    if let Ok(mut c) = child {
                                        let _ = c.wait().await;
                                    }
                                });
                            }
                        }
                        "sync_saves" | "pull_save" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                rt.spawn(async move {
                                    let _ = tokio::process::Command::new("xodus")
                                        .arg("save")
                                        .arg("pull")
                                        .arg(&path_owned)
                                        .status()
                                        .await;
                                });
                            }
                        }
                        "push_save" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                rt.spawn(async move {
                                    let _ = tokio::process::Command::new("xodus")
                                        .arg("save")
                                        .arg("push")
                                        .arg(&path_owned)
                                        .status()
                                        .await;
                                });
                            }
                        }
                        "sync_all_saves" => {
                            log::info!("Auto-syncing cloud saves for all installed titles...");
                            let proxy_tokio = proxy_ipc.clone();
                            rt.spawn(async move {
                                let default_path = std::path::PathBuf::from("/mnt/w11/XboxGames");
                                if let Ok(mut entries) = tokio::fs::read_dir(&default_path).await {
                                    while let Ok(Some(entry)) = entries.next_entry().await {
                                        let p = entry.path();
                                        if p.is_dir() {
                                            let _ = tokio::process::Command::new("xodus")
                                                .arg("save")
                                                .arg("pull")
                                                .arg(&p)
                                                .status()
                                                .await;
                                        }
                                    }
                                }
                                let script = "if (window.showToast) window.showToast('Successfully synchronized cloud saves for all installed games');";
                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script.to_string()));
                            });
                        }
                        "init" | "sync_licenses" | "get_friends" | "get_profile" => {
                            let is_force_sync = cmd == "sync_licenses" || cmd == "get_friends" || cmd == "get_profile";
                            log::info!("Checking SQLite database cache (force_sync: {is_force_sync})...");
                            let tokens_clone = tokens_ipc.clone();
                            let proxy_tokio = proxy_ipc.clone();
                            rt.spawn(async move {
                                let db = xodus::db::Database::open_default().ok();
                                let mut has_fresh_catalog = false;
                                let mut has_fresh_profile = false;

                                // 1. Instantly hydrate UI from SQLite DB cache (zero network latency)
                                if let Some(ref database) = db {
                                    if let Ok(Some(cached_prof)) = database.get_user_profile("me") {
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();
                                        if now <= cached_prof.updated_at + 86400 {
                                            has_fresh_profile = true;
                                        }

                                        let json_prof = serde_json::json!({
                                            "gamertag": cached_prof.gamertag,
                                            "gamerScore": cached_prof.gamer_score.unwrap_or_else(|| "0".into()),
                                            "displayPicRaw": cached_prof.display_pic_url.unwrap_or_default(),
                                            "presence": cached_prof.presence_state.unwrap_or_else(|| "Online".into()),
                                        });
                                        let script = format!("if (window.setUserData) window.setUserData({json_prof});");
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));

                                        let gp_script = format!("if (window.setGamePassStatus) window.setGamePassStatus({});", cached_prof.has_gamepass);
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(gp_script));
                                    }

                                    if let Ok(cached_friends) = database.get_friends("me") {
                                        if !cached_friends.is_empty() {
                                            let json_friends: Vec<_> = cached_friends.into_iter().map(|f| {
                                                serde_json::json!({
                                                    "gamertag": f.gamertag,
                                                    "displayPicRaw": f.display_pic_url.unwrap_or_default(),
                                                    "presenceState": f.presence_state,
                                                    "presenceText": f.presence_title.unwrap_or_default(),
                                                })
                                            }).collect();
                                            if let Ok(json_str) = serde_json::to_string(&json_friends) {
                                                let script = format!("if (window.setFriendsData) window.setFriendsData({json_str});");
                                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                            }
                                        }
                                    }

                                    if let Ok(cached_products) = database.get_all_catalog_products() {
                                        if !cached_products.is_empty() {
                                            if cached_products.len() > 100 {
                                                has_fresh_catalog = true;
                                            }
                                            
                                            let default_path = std::path::PathBuf::from("/mnt/w11/XboxGames");
                                            let mut installed_folders = std::collections::HashSet::new();
                                            if let Ok(mut entries) = tokio::fs::read_dir(&default_path).await {
                                                while let Ok(Some(entry)) = entries.next_entry().await {
                                                    if entry.path().is_dir() {
                                                        if let Some(name) = entry.file_name().to_str() {
                                                            installed_folders.insert(name.to_lowercase());
                                                        }
                                                    }
                                                }
                                            }

                                            let cached_items: Vec<_> = cached_products.into_iter().map(|mut p| {
                                                let mut is_installed = false;
                                                let mut install_path = format!("/mnt/w11/XboxGames/{}", p.product_id);
                                                for folder in &installed_folders {
                                                    if folder == &p.product_id.to_lowercase() || folder == &p.title.to_lowercase() || p.title.to_lowercase().contains(folder) {
                                                        is_installed = true;
                                                        install_path = format!("/mnt/w11/XboxGames/{}", folder); // simplistic mapping
                                                        break;
                                                    }
                                                }
                                                
                                                serde_json::json!({
                                                    "id": p.product_id,
                                                    "productId": p.product_id,
                                                    "title": p.title,
                                                    "developer": p.developer,
                                                    "licenseType": "gamepass",
                                                    "installed": is_installed,
                                                    "size": "Standard",
                                                    "path": install_path,
                                                    "cover": p.poster_url.unwrap_or_else(|| "https://shared.steamstatic.com/store_item_assets/steam/apps/1817230/library_600x900.jpg".into()),
                                                    "cloudSynced": true,
                                                    "lastPlayed": "Licensed"
                                                })
                                            }).collect();
                                            if let Ok(json_str) = serde_json::to_string(&cached_items) {
                                                let script = format!("if (window.setLibraryData) window.setLibraryData({json_str});");
                                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                            }
                                        }
                                    }
                                }

                                // Skip background network requests if cache is already fresh and user didn't force sync
                                if !is_force_sync && has_fresh_catalog && has_fresh_profile {
                                    log::info!("SQLite cache is fresh (catalog & profile). Skipping redundant API queries on launch.");
                                    return;
                                }

                                // 2. Perform background network sync to update DB
                                let client = reqwest::Client::new();
                                if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens_clone, "http://xboxlive.com").await {
                                    let xuid_str = xsts.xuid().map(|s| s.to_string());
                                    let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                                    let mut current_gamertag = "Player".to_string();
                                    let mut current_pic = None;
                                    let mut current_score = None;

                                    if let Ok(Some(profile)) = xodus::api::xbox::get_user_profile(&client, &auth_header).await {
                                        current_gamertag = profile.gamertag.clone();
                                        current_pic = Some(profile.display_pic.clone());
                                        current_score = Some(profile.gamerscore.clone());

                                        if let Ok(json_str) = serde_json::to_string(&profile) {
                                            let script = format!("if (window.setUserData) window.setUserData({json_str});");
                                            let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                        }
                                    }
                                    if let Ok(friends) = xodus::api::xbox::SocialClient::new(&client).get_friends(&auth_header).await {
                                        if !friends.is_empty() {
                                            if let Some(ref database) = db {
                                                let cached_f: Vec<_> = friends.iter().map(|f| {
                                                    xodus::db::CachedFriend {
                                                        xuid: "me".to_string(),
                                                        friend_xuid: f.gamertag.clone(),
                                                        gamertag: f.gamertag.clone(),
                                                        display_pic_url: f.display_pic_raw.clone(),
                                                        presence_state: f.presence_state.clone().unwrap_or_else(|| "Offline".into()),
                                                        presence_title: f.presence_text.clone(),
                                                        updated_at: 0,
                                                    }
                                                }).collect();
                                                let _ = database.save_friends("me", &cached_f);
                                            }

                                            if let Ok(json_str) = serde_json::to_string(&friends) {
                                                let script = format!("if (window.setFriendsData) window.setFriendsData({json_str});");
                                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                            }
                                        }
                                    }
                                    let has_gamepass = xodus::api::xbox::check_user_gamepass_subscription(&client, &auth_header, xuid_str.as_deref()).await;
                                    let gp_script = format!("if (window.setGamePassStatus) window.setGamePassStatus({has_gamepass});");
                                    let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(gp_script));

                                    // Save refreshed profile & gamepass status in SQLite
                                    if let Some(ref database) = db {
                                        let _ = database.save_user_profile(&xodus::db::CachedUserProfile {
                                            xuid: "me".to_string(),
                                            gamertag: current_gamertag,
                                            display_pic_url: current_pic,
                                            gamer_score: current_score,
                                            presence_state: Some("Online".into()),
                                            presence_title: None,
                                            has_gamepass,
                                            subscription_tier: if has_gamepass { Some("GamePass".into()) } else { None },
                                            updated_at: 0,
                                        });
                                    }

                                    let mut final_games = Vec::new();
                                    let mut owned_ids = std::collections::HashSet::new();

                                    // Helper to resolve items from DB first, and only query missing IDs from network
                                    let resolve_catalog = |ids: &[String], default_license: &str, db_opt: Option<&xodus::db::Database>, client: &reqwest::Client| {
                                        let mut items = Vec::new();
                                        let mut missing = Vec::new();

                                        if let Some(database) = db_opt {
                                            for id in ids {
                                                if let Ok(Some(cached)) = database.get_catalog_product(id) {
                                                    items.push(xodus::api::xbox::GameCatalogItem {
                                                        id: cached.product_id.clone(),
                                                        product_id: cached.product_id.clone(),
                                                        title: cached.title,
                                                        developer: cached.developer,
                                                        license_type: default_license.to_string(),
                                                        installed: false,
                                                        size: "Standard".to_string(),
                                                        path: format!("/mnt/w11/XboxGames/{}", cached.product_id),
                                                        cover: cached.poster_url.unwrap_or_else(|| "https://shared.steamstatic.com/store_item_assets/steam/apps/1817230/library_600x900.jpg".into()),
                                                        cloud_synced: true,
                                                        last_played: "Licensed".to_string(),
                                                    });
                                                } else {
                                                    missing.push(id.clone());
                                                }
                                            }
                                        } else {
                                            missing.extend(ids.iter().cloned());
                                        }

                                        (items, missing)
                                    };

                                    // 1. Fetch User Collections (Owned / Entitled licenses)
                                    if let Ok(collections) = xodus::api::xbox::get_user_collections(&client, &auth_header, xuid_str.as_deref()).await {
                                        let product_ids: Vec<String> = collections.into_iter().map(|c| c.product_id).collect();
                                        if !product_ids.is_empty() {
                                            for pid in &product_ids {
                                                owned_ids.insert(pid.clone());
                                            }
                                            let (cached_items, missing_ids) = resolve_catalog(&product_ids, "owned", db.as_ref(), &client);
                                            final_games.extend(cached_items);

                                            if !missing_ids.is_empty() {
                                                let mut enriched_owned = xodus::api::xbox::enrich_products_catalog(&client, &missing_ids).await;
                                                if let Some(ref database) = db {
                                                    let to_cache: Vec<_> = enriched_owned.iter().map(|item| {
                                                        xodus::db::CachedCatalogProduct {
                                                            product_id: item.product_id.clone(),
                                                            title: item.title.clone(),
                                                            developer: item.developer.clone(),
                                                            publisher: "".to_string(),
                                                            description: "".to_string(),
                                                            poster_url: Some(item.cover.clone()),
                                                            hero_url: None,
                                                            package_family_name: None,
                                                            content_id: None,
                                                            size_in_bytes: None,
                                                            raw_json: None,
                                                            updated_at: 0,
                                                            ttl: 604800,
                                                        }
                                                    }).collect();
                                                    let _ = database.save_catalog_products_batch(&to_cache);
                                                }
                                                for item in &mut enriched_owned {
                                                    item.license_type = "owned".to_string();
                                                }
                                                final_games.extend(enriched_owned);
                                            }
                                        }
                                    }

                                    // 2. Fetch full PC Game Pass Catalog (500+ Game Pass Titles)
                                    if let Ok(gp_ids) = xodus::api::xbox::get_gamepass_catalog_ids(&client).await {
                                        let unowned_gp_ids: Vec<String> = gp_ids.into_iter().filter(|id| !owned_ids.contains(id)).collect();
                                        if !unowned_gp_ids.is_empty() {
                                            let (cached_items, missing_ids) = resolve_catalog(&unowned_gp_ids, "gamepass", db.as_ref(), &client);
                                            final_games.extend(cached_items);

                                            if !missing_ids.is_empty() {
                                                let mut enriched_gp = xodus::api::xbox::enrich_products_catalog(&client, &missing_ids).await;
                                                if let Some(ref database) = db {
                                                    let to_cache: Vec<_> = enriched_gp.iter().map(|item| {
                                                        xodus::db::CachedCatalogProduct {
                                                            product_id: item.product_id.clone(),
                                                            title: item.title.clone(),
                                                            developer: item.developer.clone(),
                                                            publisher: "".to_string(),
                                                            description: "".to_string(),
                                                            poster_url: Some(item.cover.clone()),
                                                            hero_url: None,
                                                            package_family_name: None,
                                                            content_id: None,
                                                            size_in_bytes: None,
                                                            raw_json: None,
                                                            updated_at: 0,
                                                            ttl: 604800,
                                                        }
                                                    }).collect();
                                                    let _ = database.save_catalog_products_batch(&to_cache);
                                                }
                                                for item in &mut enriched_gp {
                                                    item.license_type = "gamepass".to_string();
                                                }
                                                final_games.extend(enriched_gp);
                                            }
                                        }
                                    }

                                    // 3. Mark and integrate installed titles from /mnt/w11/XboxGames
                                    let default_path = std::path::PathBuf::from("/mnt/w11/XboxGames");
                                    if let Ok(mut entries) = tokio::fs::read_dir(&default_path).await {
                                        while let Ok(Some(entry)) = entries.next_entry().await {
                                            let p = entry.path();
                                            if p.is_dir() {
                                                if let Some(folder_name) = p.file_name().and_then(|n| n.to_str()) {
                                                    let mut matched = false;
                                                    for game in &mut final_games {
                                                        if game.title.eq_ignore_ascii_case(folder_name) 
                                                           || game.product_id.eq_ignore_ascii_case(folder_name)
                                                           || game.title.to_lowercase().contains(&folder_name.to_lowercase()) {
                                                            game.installed = true;
                                                            game.path = p.to_string_lossy().to_string();
                                                            game.cloud_synced = true;
                                                            matched = true;
                                                        }
                                                    }
                                                    if !matched {
                                                        final_games.insert(0, xodus::api::xbox::GameCatalogItem {
                                                            id: folder_name.to_string(),
                                                            product_id: folder_name.to_string(),
                                                            title: folder_name.to_string(),
                                                            developer: "Local Game Container".to_string(),
                                                            license_type: "owned".to_string(),
                                                            installed: true,
                                                            size: "Installed".to_string(),
                                                            path: p.to_string_lossy().to_string(),
                                                            cover: "https://shared.steamstatic.com/store_item_assets/steam/apps/1817230/library_600x900.jpg".to_string(),
                                                            cloud_synced: true,
                                                            last_played: "Today".to_string(),
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !final_games.is_empty() {
                                        if let Ok(json_str) = serde_json::to_string(&final_games) {
                                            let script = format!("if (window.setLibraryData) window.setLibraryData({json_str});");
                                            let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                        }
                                    }

                                    // Auto-sync cloud saves for all installed titles in background
                                    let default_path = std::path::PathBuf::from("/mnt/w11/XboxGames");
                                    if let Ok(mut entries) = tokio::fs::read_dir(&default_path).await {
                                        while let Ok(Some(entry)) = entries.next_entry().await {
                                            let p = entry.path();
                                            if p.is_dir() {
                                                let _ = tokio::process::Command::new("xodus")
                                                    .arg("save")
                                                    .arg("pull")
                                                    .arg(&p)
                                                    .status()
                                                    .await;
                                            }
                                        }
                                    }
                                    let script = "if (window.markAllSavesSynced) window.markAllSavesSynced();";
                                    let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script.to_string()));
                                    }
                                });
                            }
                        "login" => {
                            log::info!("Triggering Microsoft login flow...");
                            let tokens_clone = tokens_ipc.clone();
                            let proxy_tokio = proxy_ipc.clone();
                            rt.spawn(async move {
                                let child = tokio::process::Command::new("xodus")
                                    .arg("login")
                                    .spawn();
                                if let Ok(mut c) = child {
                                    let _ = c.wait().await;
                                    let client = reqwest::Client::new();
                                    if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens_clone, "http://xboxlive.com").await {
                                        let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                                        if let Ok(Some(profile)) = xodus::api::xbox::get_user_profile(&client, &auth_header).await {
                                            if let Ok(json_str) = serde_json::to_string(&profile) {
                                                let script = format!("if (window.setUserData) window.setUserData({json_str});");
                                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                            }
                                        }
                                        if let Ok(friends) = xodus::api::xbox::SocialClient::new(&client).get_friends(&auth_header).await {
                                            if let Ok(json_str) = serde_json::to_string(&friends) {
                                                let script = format!("if (window.setFriendsData) window.setFriendsData({json_str});");
                                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        "logout" => {
                            let _ = tokens_ipc.remove_user();
                            let script = "if (window.showToast) window.showToast('Logged out of Microsoft Account');";
                            let _ = proxy_ipc.send_event(CustomEvent::EvaluateScript(script.to_string()));
                        }
                        "set_presence" => {
                            if let Some(state_str) = v.get("state").and_then(|s| s.as_str()) {
                                let st_owned = state_str.to_string();
                                let tokens_clone = tokens_ipc.clone();
                                rt.spawn(async move {
                                    if let Ok(db) = xodus::db::Database::open_default() {
                                        let _ = db.update_presence_state("me", &st_owned);
                                    }
                                    let client = reqwest::Client::new();
                                    if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens_clone, "http://xboxlive.com").await {
                                        let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                                        let _ = xodus::api::xbox::SocialClient::new(&client).set_presence(&auth_header, &st_owned).await;
                                    }
                                });
                            }
                        }
                        "install_game" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                log::info!("Preparing package install & decryption for: {path}");
                            }
                        }
                        _ => {}
                    }
                }
            }
        });


    #[cfg(target_os = "linux")]
    let webview = {
        use gtk::prelude::WidgetExt;
        use gtk::prelude::GtkWindowExt;
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get window default vbox");
        let wv = builder.build_gtk(vbox)?;
        window.gtk_window().show_all();
        window.gtk_window().present();
        wv
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    window.set_focus();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(CustomEvent::EvaluateScript(script)) => {
                let _ = webview.evaluate_script(&script);
            }
            _ => (),
        }
    });

}
