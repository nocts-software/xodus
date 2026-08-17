use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tao::{
    dpi::{LogicalSize, PhysicalPosition, Position, Size},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
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

fn normalize_title(title: &str) -> String {
    let mut s = title.to_lowercase();
    let patterns = [
        " - windows",
        " (windows)",
        " - pc",
        " (pc)",
        " - xbox series x|s",
        " - xbox one",
        " windows 10 edition",
        " windows edition",
        ": 2026 edition",
        ": 2025 edition",
        ": 2024 edition",
        " standard edition",
        " digital edition",
    ];
    for pat in patterns {
        s = s.replace(pat, "");
    }
    s.trim().to_string()
}

fn edition_tier(title: &str) -> u32 {
    let lower = title.to_lowercase();
    if lower.contains("ultimate") || lower.contains("complete") || lower.contains("anniversary") || lower.contains("collector") {
        4
    } else if lower.contains("premium") || lower.contains("gold") {
        3
    } else if lower.contains("deluxe") {
        2
    } else if lower.contains("enhanced") || lower.contains("special") || lower.contains("day one") {
        1
    } else {
        0 // Standard / Base
    }
}

fn is_valid_pc_big_id(id: &str) -> bool {
    id.len() == 12 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_valid_store_id(id: &str) -> bool {
    is_valid_pc_big_id(id) || id.contains('.') || id.contains('_')
}

#[derive(Debug, Clone)]
struct InstalledGameInfo {
    folder_name: String,
    title: String,
    store_id: Option<String>,
    pfn: Option<String>,
    path: String,
}

fn scan_installed_game_info(path: &std::path::Path) -> Option<InstalledGameInfo> {
    if !path.is_dir() { return None; }
    let folder_name = path.file_name()?.to_str()?.to_string();
    let folder_lower = folder_name.to_lowercase();
    if folder_lower == "gamesave" || folder_lower == "wgs" || folder_lower == "msixvc" || folder_lower.starts_with('.') || folder_lower.starts_with('$') || folder_lower == "system volume information" || folder_lower == "temp" {
        return None;
    }

    let config_paths = [
        path.join("MicrosoftGame.config"),
        path.join("MicrosoftGame.Config"),
        path.join("Content").join("MicrosoftGame.config"),
        path.join("Content").join("MicrosoftGame.Config"),
    ];
    let manifest_paths = [
        path.join("appxmanifest.xml"),
        path.join("AppxManifest.xml"),
        path.join("Content").join("appxmanifest.xml"),
        path.join("Content").join("AppxManifest.xml"),
    ];

    let mut detected_title = folder_name.clone();
    let mut detected_store_id = None;
    let mut detected_pfn = None;

    for cp in &config_paths {
        if cp.exists() {
            if let Ok(content) = std::fs::read_to_string(cp) {
                if let Some(s_pos) = content.find("<StoreId>") {
                    if let Some(e_pos) = content[s_pos..].find("</StoreId>") {
                        let sid = content[s_pos + 9..s_pos + e_pos].trim();
                        if is_valid_pc_big_id(sid) {
                            detected_store_id = Some(sid.to_string());
                        }
                    }
                }
                if let Some(d_pos) = content.find("DefaultDisplayName=\"") {
                    if let Some(e_pos) = content[d_pos + 20..].find('"') {
                        let dname = &content[d_pos + 20..d_pos + 20 + e_pos];
                        if !dname.is_empty() {
                            detected_title = dname.to_string();
                        }
                    }
                }
                if let Some(i_pos) = content.find("<Identity Name=\"") {
                    if let Some(e_pos) = content[i_pos + 16..].find('"') {
                        let iname = &content[i_pos + 16..i_pos + 16 + e_pos];
                        if !iname.is_empty() {
                            detected_pfn = Some(iname.to_string());
                        }
                    }
                }
                break;
            }
        }
    }

    for mp in &manifest_paths {
        if mp.exists() {
            if let Ok(content) = std::fs::read_to_string(mp) {
                if let Some(d_pos) = content.find("<DisplayName>") {
                    if let Some(e_pos) = content[d_pos..].find("</DisplayName>") {
                        let dname = content[d_pos + 13..d_pos + e_pos].trim();
                        if !dname.is_empty() && !dname.starts_with("ms-resource:") {
                            detected_title = dname.to_string();
                        }
                    }
                }
                if let Some(i_pos) = content.find("<Identity Name=\"") {
                    if let Some(e_pos) = content[i_pos + 16..].find('"') {
                        let iname = &content[i_pos + 16..i_pos + 16 + e_pos];
                        if !iname.is_empty() {
                            detected_pfn = Some(iname.to_string());
                        }
                    }
                }
                break;
            }
        }
    }

    if !has_game_files(path) {
        return None;
    }

    Some(InstalledGameInfo {
        folder_name,
        title: detected_title,
        store_id: detected_store_id,
        pfn: detected_pfn,
        path: path.to_string_lossy().to_string(),
    })
}

fn deduplicate_games(games: Vec<xodus::api::xbox::GameCatalogItem>, has_gamepass_sub: bool) -> Vec<xodus::api::xbox::GameCatalogItem> {
    let mut map: std::collections::BTreeMap<String, xodus::api::xbox::GameCatalogItem> = std::collections::BTreeMap::new();

    for g in games {
        let key = normalize_title(&g.title);
        if key.is_empty() || key == "gamesave" || key == "wgs" {
            continue;
        }

        if let Some(existing) = map.get_mut(&key) {
            let existing_tier = edition_tier(&existing.title);
            let g_tier = edition_tier(&g.title);

            let is_g_installed = g.installed;
            let is_existing_installed = existing.installed;

            if existing.license_type == "owned" && g.license_type == "gamepass" {
                if has_gamepass_sub && g_tier > existing_tier {
                    // Game Pass has a higher edition tier (e.g. Deluxe vs Standard) and user has active Game Pass -> prefer Game Pass edition!
                    let was_installed = is_existing_installed || is_g_installed;
                    let path = if is_g_installed { g.path.clone() } else { existing.path.clone() };
                    *existing = g.clone();
                    existing.installed = was_installed;
                    existing.path = path;
                } else {
                    // Otherwise, always prefer our owned license!
                    if is_g_installed { existing.installed = true; existing.path = g.path.clone(); }
                }
            } else if existing.license_type == "gamepass" && g.license_type == "owned" {
                if has_gamepass_sub && existing_tier > g_tier {
                    // Game Pass in map has a higher edition tier than owned -> keep Game Pass edition
                    if is_g_installed { existing.installed = true; existing.path = g.path.clone(); }
                } else {
                    // Otherwise, prefer owned license!
                    let was_installed = is_existing_installed || is_g_installed;
                    let path = if g.installed { g.path.clone() } else { existing.path.clone() };
                    *existing = g.clone();
                    existing.license_type = "owned".to_string();
                    existing.installed = was_installed;
                    existing.path = path;
                }
            } else {
                // Both owned or both gamepass: prefer higher tier or installed version
                if g_tier > existing_tier || (!existing.installed && g.installed) {
                    let was_owned = existing.license_type == "owned" || g.license_type == "owned";
                    *existing = g.clone();
                    if was_owned { existing.license_type = "owned".to_string(); }
                }
            }

            // Always prefer 12-char BigID if current item has numeric ID
            if !is_valid_pc_big_id(&existing.product_id) && is_valid_pc_big_id(&g.product_id) {
                existing.product_id = g.product_id.clone();
                existing.id = g.product_id.clone();
            }
            if (existing.developer.is_empty() || existing.developer == "Xbox Game Studios" || existing.developer == "Local Game Container" || existing.developer == "Local Game")
                && (!g.developer.is_empty() && g.developer != "Xbox Game Studios" && g.developer != "Local Game Container" && g.developer != "Local Game") {
                existing.developer = g.developer.clone();
            }
            if existing.cover.contains("library_600x900.jpg") && !g.cover.contains("library_600x900.jpg") {
                existing.cover = g.cover.clone();
            }
        } else {
            map.insert(key, g);
        }
    }

    let db = xodus::db::Database::open_default().ok();
    let all_catalog_products = db.as_ref().and_then(|d| d.get_all_catalog_products().ok()).unwrap_or_default();

    let mut result: Vec<xodus::api::xbox::GameCatalogItem> = Vec::new();
    for mut g in map.into_values() {
        // If not locally installed on disk, verify it has a valid Windows PC Store BigID or PFN
        if !g.installed {
            if !is_valid_store_id(&g.product_id) {
                // Attempt lookup in catalog products by title
                let mut resolved = false;
                for p in &all_catalog_products {
                    if (p.title.eq_ignore_ascii_case(&g.title) || normalize_title(&p.title) == normalize_title(&g.title)) && is_valid_pc_big_id(&p.product_id) {
                        g.product_id = p.product_id.clone();
                        g.id = p.product_id.clone();
                        resolved = true;
                        break;
                    }
                }
                if !resolved {
                    // Filter out console-only / legacy Xbox titles completely
                    continue;
                }
            }

            // If it's a Game Pass title but user does not have an active Game Pass subscription, do not include
            if g.license_type == "gamepass" && !has_gamepass_sub {
                continue;
            }
        }
        result.push(g);
    }

    result
}

fn has_game_files(path: &std::path::Path) -> bool {
    let mut has_executable = false;

    fn check_dir_recursive(dir: &std::path::Path, depth: u32, has_exe: &mut bool) {
        if depth > 5 || *has_exe { return; }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let file_name = p.file_name().unwrap_or_default().to_string_lossy();
                // Skip hidden files/directories and temporary download archives
                if file_name.starts_with('.') || file_name.starts_with('$') || file_name.ends_with(".tmp") || file_name.ends_with(".msixvc") {
                    continue;
                }
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "exe" {
                            if let Ok(meta) = p.metadata() {
                                if meta.len() > 10240 { // Real executable > 10KB
                                    *has_exe = true;
                                    return;
                                }
                            }
                        }
                    }
                } else if p.is_dir() {
                    let dir_lower = file_name.to_lowercase();
                    if dir_lower != "gamesave" && dir_lower != "wgs" && !dir_lower.starts_with('.') {
                        check_dir_recursive(&p, depth + 1, has_exe);
                    }
                }
            }
        }
    }

    check_dir_recursive(path, 0, &mut has_executable);
    has_executable
}

fn find_xodus_cli() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cli_cand = dir.join("xodus-cli");
            if cli_cand.exists() {
                return cli_cand;
            }
            let cand = dir.join("xodus");
            if cand.exists() {
                return cand;
            }
        }
    }
    if let Ok(appdir) = std::env::var("APPDIR") {
        let p1 = std::path::PathBuf::from(&appdir).join("usr/bin/xodus");
        if p1.exists() { return p1; }
        let p2 = std::path::PathBuf::from(&appdir).join("usr/bin/xodus-cli");
        if p2.exists() { return p2; }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let p_cargo = std::path::PathBuf::from(&home).join(".cargo/bin/xodus");
    if p_cargo.exists() { return p_cargo; }
    let p_cargo_cli = std::path::PathBuf::from(&home).join(".cargo/bin/xodus-cli");
    if p_cargo_cli.exists() { return p_cargo_cli; }
    let p_local = std::path::PathBuf::from("/usr/local/bin/xodus");
    if p_local.exists() { return p_local; }
    let p_usr = std::path::PathBuf::from("/usr/bin/xodus");
    if p_usr.exists() { return p_usr; }

    std::path::PathBuf::from("xodus")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowState {
    x: Option<i32>,
    y: Option<i32>,
    width: u32,
    height: u32,
    is_maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1280,
            height: 800,
            is_maximized: false,
        }
    }
}

fn get_window_state_path() -> std::path::PathBuf {
    let base_dir = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|s| !s.is_empty())
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(std::env::temp_dir);

    base_dir.join("xodus").join("window_state.json")
}

fn load_window_state() -> WindowState {
    let path = get_window_state_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(state) = serde_json::from_str::<WindowState>(&data) {
            return state;
        }
    }
    WindowState::default()
}

fn save_window_state(state: &WindowState) {
    let path = get_window_state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, json);
    }
}

fn record_window_state(window: &tao::window::Window, state_lock: &Arc<Mutex<WindowState>>) {
    let is_max = window.is_maximized();
    let mut st = state_lock.lock().unwrap();
    st.is_maximized = is_max;
    if !is_max {
        if let Ok(pos) = window.outer_position() {
            st.x = Some(pos.x);
            st.y = Some(pos.y);
        }
        let scale = window.scale_factor().max(1.0);
        let size = window.inner_size().to_logical::<f64>(scale);
        st.width = (size.width as u32).clamp(960, 3840);
        st.height = (size.height as u32).clamp(600, 2160);
    }
    save_window_state(&st);
}

async fn run_hydrate_and_sync(
    tokens: Arc<TokenManager>,
    proxy_tokio: EventLoopProxy<CustomEvent>,
    _is_force_sync: bool,
) {
    let db = xodus::db::Database::open_default().ok();

    // 1. Instantly hydrate UI from SQLite DB cache (zero network latency)
    if let Some(ref database) = db {
        if let Ok(Some(cached_prof)) = database.get_user_profile("me") {
            let json_prof = serde_json::json!({
                "gamertag": cached_prof.gamertag,
                "gamerScore": cached_prof.gamer_score.unwrap_or_else(|| "0".into()),
                "displayPicRaw": cached_prof.display_pic_url.unwrap_or_default(),
                "presence": cached_prof.presence_state.unwrap_or_else(|| "Online".into()),
                "hasGamePass": cached_prof.has_gamepass,
                "subscriptionTier": cached_prof.subscription_tier,
            });
            let script = format!("if (window.setUserData) window.setUserData({json_prof});");
            let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));

            let tier_json = serde_json::to_string(&cached_prof.subscription_tier).unwrap_or_else(|_| "null".into());
            let gp_script = format!("if (window.setGamePassStatus) window.setGamePassStatus({}, {});", cached_prof.has_gamepass, tier_json);
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

        let default_path = std::path::PathBuf::from("/mnt/w11/XboxGames");
        let mut installed_list = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&default_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(info) = scan_installed_game_info(&entry.path()) {
                    installed_list.push(info);
                }
            }
        }

        let cached_prof = database.get_user_profile("me").ok().flatten();
        let has_gp = cached_prof.map(|p| p.has_gamepass).unwrap_or(false);

        let cached_products = database.get_all_catalog_products().unwrap_or_default();
        let catalog_map: std::collections::HashMap<String, xodus::db::CachedCatalogProduct> = cached_products
            .into_iter()
            .map(|p| (p.product_id.clone(), p))
            .collect();

        let cached_entitlements = database.get_user_entitlements("me").unwrap_or_default();
        let mut cached_items: Vec<xodus::api::xbox::GameCatalogItem> = Vec::new();
        let mut added_ids = std::collections::HashSet::new();

        // 1a. Add all user entitlements (Owned games)
        for ent in &cached_entitlements {
            if added_ids.contains(&ent.product_id) { continue; }
            let mut title = ent.title.clone().unwrap_or_default();
            let mut developer = "Xbox Game Studios".to_string();
            let mut cover = xodus::api::xbox::DEFAULT_COVER_URL.to_string();

            if let Some(cat) = catalog_map.get(&ent.product_id) {
                if !cat.title.is_empty() { title = cat.title.clone(); }
                if !cat.developer.is_empty() { developer = cat.developer.clone(); }
                if let Some(ref p_url) = cat.poster_url {
                    if !p_url.is_empty() { cover = p_url.clone(); }
                }
            }

            if title.is_empty() {
                title = ent.product_id.clone();
            }

            let mut is_installed = false;
            let mut install_path = format!("/mnt/w11/XboxGames/{}", ent.product_id);

            for inst in &installed_list {
                let matches_sid = inst.store_id.as_deref().map(|s| s.eq_ignore_ascii_case(&ent.product_id)).unwrap_or(false);
                let matches_pfn = inst.pfn.as_deref().map(|s| s.eq_ignore_ascii_case(&ent.product_id)).unwrap_or(false);
                let matches_title = inst.title.eq_ignore_ascii_case(&title) || normalize_title(&inst.title) == normalize_title(&title);
                let matches_folder = inst.folder_name.eq_ignore_ascii_case(&ent.product_id) || inst.folder_name.eq_ignore_ascii_case(&title);

                if matches_sid || matches_pfn || matches_title || matches_folder {
                    is_installed = true;
                    install_path = inst.path.clone();
                    break;
                }
            }

            added_ids.insert(ent.product_id.clone());
            cached_items.push(xodus::api::xbox::GameCatalogItem {
                id: ent.product_id.clone(),
                product_id: ent.product_id.clone(),
                title,
                developer,
                license_type: "owned".to_string(),
                installed: is_installed,
                size: if is_installed { "Installed".to_string() } else { "Standard".to_string() },
                path: install_path,
                cover,
                cloud_synced: true,
                last_played: if is_installed { "Installed".to_string() } else { "Licensed".to_string() },
            });
        }

        // 1b. Add any on-disk installed titles not in entitlements
        for inst in &installed_list {
            let pid = inst.store_id.clone().or_else(|| inst.pfn.clone()).unwrap_or_else(|| inst.folder_name.clone());
            let already_present = cached_items.iter().any(|g| {
                g.title.eq_ignore_ascii_case(&inst.title)
                    || normalize_title(&g.title) == normalize_title(&inst.title)
                    || inst.store_id.as_deref().map(|s| s.eq_ignore_ascii_case(&g.product_id)).unwrap_or(false)
            });
            if !already_present {
                added_ids.insert(pid.clone());
                cached_items.push(xodus::api::xbox::GameCatalogItem {
                    id: pid.clone(),
                    product_id: pid,
                    title: inst.title.clone(),
                    developer: "Local Game".to_string(),
                    license_type: "owned".to_string(),
                    installed: true,
                    size: "Installed".to_string(),
                    path: inst.path.clone(),
                    cover: xodus::api::xbox::DEFAULT_COVER_URL.to_string(),
                    cloud_synced: true,
                    last_played: "Today".to_string(),
                });
            }
        }

        // 1c. If user has active Game Pass, add unowned Game Pass catalog products
        if has_gp {
            for (p_id, cat) in &catalog_map {
                if !added_ids.contains(p_id) {
                    cached_items.push(xodus::api::xbox::GameCatalogItem {
                        id: p_id.clone(),
                        product_id: p_id.clone(),
                        title: cat.title.clone(),
                        developer: cat.developer.clone(),
                        license_type: "gamepass".to_string(),
                        installed: false,
                        size: "Standard".to_string(),
                        path: format!("/mnt/w11/XboxGames/{}", p_id),
                        cover: cat.poster_url.clone().unwrap_or_else(|| xodus::api::xbox::DEFAULT_COVER_URL.into()),
                        cloud_synced: true,
                        last_played: "Game Pass".to_string(),
                    });
                }
            }
        }

        let deduplicated = deduplicate_games(cached_items, has_gp);
        if !deduplicated.is_empty() {
            if let Ok(json_str) = serde_json::to_string(&deduplicated) {
                let script = format!("if (window.setLibraryData) window.setLibraryData({json_str});");
                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
            }
        }
    }

    // 2. Perform background network sync to update DB with latest licenses & presence
    let client = reqwest::Client::new();
    if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await {
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
        let (has_gamepass, subscription_tier) = xodus::api::xbox::check_user_gamepass_subscription(&client, &auth_header, xuid_str.as_deref()).await;
        let tier_json = serde_json::to_string(&subscription_tier).unwrap_or_else(|_| "null".into());
        let gp_script = format!("if (window.setGamePassStatus) window.setGamePassStatus({has_gamepass}, {tier_json});");
        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(gp_script));

        let saved_presence = if let Some(ref database) = db {
            database.get_user_profile("me").ok().flatten().and_then(|p| p.presence_state).unwrap_or_else(|| "Active".into())
        } else {
            "Active".to_string()
        };

        // Sync saved presence state with Xbox Live on startup/refresh
        let _ = xodus::api::xbox::SocialClient::new(&client).set_presence(&auth_header, &saved_presence).await;

        let user_json = serde_json::json!({
            "gamertag": current_gamertag,
            "displayPic": current_pic,
            "gamerscore": current_score,
            "presence": saved_presence,
            "hasGamePass": has_gamepass,
            "subscriptionTier": subscription_tier,
        });
        let user_script = format!("if (window.setUserData) window.setUserData({user_json});");
        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(user_script));

        // Save refreshed profile & gamepass status in SQLite
        if let Some(ref database) = db {
            let _ = database.save_user_profile(&xodus::db::CachedUserProfile {
                xuid: "me".to_string(),
                gamertag: current_gamertag,
                display_pic_url: current_pic,
                gamer_score: current_score,
                presence_state: Some(saved_presence),
                presence_title: None,
                has_gamepass,
                subscription_tier: subscription_tier.clone(),
                updated_at: 0,
            });
        }

        let mut final_games = Vec::new();
        let mut owned_ids = std::collections::HashSet::new();

        // 1. Fetch User Collections (Owned / Entitled licenses with full metadata & artwork)
        let licensing_xsts = xodus::api::xbox::get_or_request_xsts(&client, &tokens, "http://licensing.xboxlive.com").await.ok();
        let licensing_auth = licensing_xsts.map(xodus::api::xbox::get_xsts_auth_header);

        let owned_games = xodus::api::xbox::get_user_owned_catalog_items(
            &client,
            Some(&tokens),
            &auth_header,
            licensing_auth.as_deref(),
            xuid_str.as_deref(),
            db.as_ref(),
        ).await;

        for og in &owned_games {
            owned_ids.insert(og.product_id.clone());
        }
        final_games.extend(owned_games);

        // Also ensure all cached entitlements from DB are preserved in final_games
        if let Some(ref database) = db {
            if let Ok(ents) = database.get_user_entitlements("me") {
                let catalog_products = database.get_all_catalog_products().unwrap_or_default();
                let cat_map: std::collections::HashMap<String, xodus::db::CachedCatalogProduct> = catalog_products
                    .into_iter()
                    .map(|p| (p.product_id.clone(), p))
                    .collect();

                for e in ents {
                    if !owned_ids.contains(&e.product_id) {
                        let mut title = e.title.clone().unwrap_or_default();
                        let mut developer = "Xbox Game Studios".to_string();
                        let mut cover = xodus::api::xbox::DEFAULT_COVER_URL.to_string();
                        if let Some(cat) = cat_map.get(&e.product_id) {
                            if !cat.title.is_empty() { title = cat.title.clone(); }
                            if !cat.developer.is_empty() { developer = cat.developer.clone(); }
                            if let Some(ref p_url) = cat.poster_url {
                                if !p_url.is_empty() { cover = p_url.clone(); }
                            }
                        }
                        if title.is_empty() { title = e.product_id.clone(); }
                        owned_ids.insert(e.product_id.clone());
                        final_games.push(xodus::api::xbox::GameCatalogItem {
                            id: e.product_id.clone(),
                            product_id: e.product_id.clone(),
                            title,
                            developer,
                            license_type: "owned".to_string(),
                            installed: false,
                            size: "Standard".to_string(),
                            path: format!("/mnt/w11/XboxGames/{}", e.product_id),
                            cover,
                            cloud_synced: true,
                            last_played: "Licensed".to_string(),
                        });
                    }
                }
            }
        }

        // Helper to resolve items from DB first, and only query missing IDs from network
        let resolve_catalog = |ids: &[String], default_license: &str, db_opt: Option<&xodus::db::Database>, _client: &reqwest::Client| {
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
                            cover: cached.poster_url.unwrap_or_else(|| xodus::api::xbox::DEFAULT_COVER_URL.into()),
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

        // 2. Fetch full PC Game Pass Catalog ONLY if user has active Game Pass subscription
        if has_gamepass {
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
        }

        // 3. Mark and integrate installed titles from /mnt/w11/XboxGames
        let default_path = std::path::PathBuf::from("/mnt/w11/XboxGames");
        if let Ok(mut entries) = tokio::fs::read_dir(&default_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(inst) = scan_installed_game_info(&entry.path()) {
                    let mut matched = false;
                    for game in &mut final_games {
                        let matches_sid = inst.store_id.as_deref().map(|s| s.eq_ignore_ascii_case(&game.product_id)).unwrap_or(false);
                        let matches_pfn = inst.pfn.as_deref().map(|s| s.eq_ignore_ascii_case(&game.product_id)).unwrap_or(false);
                        let matches_title = inst.title.eq_ignore_ascii_case(&game.title) || normalize_title(&inst.title) == normalize_title(&game.title);
                        let matches_folder = inst.folder_name.eq_ignore_ascii_case(&game.product_id) || inst.folder_name.eq_ignore_ascii_case(&game.title);

                        if matches_sid || matches_pfn || matches_title || matches_folder {
                            game.installed = true;
                            game.path = inst.path.clone();
                            game.license_type = "owned".to_string();
                            game.cloud_synced = true;
                            if let Some(ref sid) = inst.store_id {
                                if !is_valid_pc_big_id(&game.product_id) {
                                    game.product_id = sid.clone();
                                    game.id = sid.clone();
                                }
                            }
                            matched = true;
                            break;
                        }
                    }

                    if !matched {
                        let pid = inst.store_id.clone().or_else(|| inst.pfn.clone()).unwrap_or_else(|| inst.folder_name.clone());
                        final_games.insert(0, xodus::api::xbox::GameCatalogItem {
                            id: pid.clone(),
                            product_id: pid,
                            title: inst.title,
                            developer: "Local Game".to_string(),
                            license_type: "owned".to_string(),
                            installed: true,
                            size: "Installed".to_string(),
                            path: inst.path,
                            cover: xodus::api::xbox::DEFAULT_COVER_URL.to_string(),
                            cloud_synced: true,
                            last_played: "Today".to_string(),
                        });
                    }
                }
            }
        }

        let final_games = deduplicate_games(final_games, has_gamepass);

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
                    let folder_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let folder_lower = folder_name.to_lowercase();
                    if folder_lower == "gamesave" || folder_lower == "wgs" || folder_lower == "msixvc" || folder_lower.starts_with('.') || folder_lower.starts_with('$') {
                        continue;
                    }
                    let _ = tokio::process::Command::new(find_xodus_cli())
                        .arg("save")
                        .arg("pull")
                        .arg(&p)
                        .status().await;
                }
            }
        }
        let script = "if (window.markAllSavesSynced) window.markAllSavesSynced();";
        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script.to_string()));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env("XODUS_LOG");
    xodus::secrets::init_secrets().ok();

    let tokens = Arc::new(TokenManager::with_keychain_and_memory());
    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let saved_state = load_window_state();
    let win_state = Arc::new(Mutex::new(saved_state.clone()));
    let win_state_ipc = win_state.clone();
    let win_state_loop = win_state.clone();

    let initial_w = saved_state.width.clamp(960, 3840);
    let initial_h = saved_state.height.clamp(600, 2160);

    let mut window_builder = WindowBuilder::new()
        .with_title("noct's xodus gui")
        .with_decorations(false)
        .with_resizable(true)
        .with_min_inner_size(Size::Logical(LogicalSize::new(960.0, 600.0)))
        .with_inner_size(Size::Logical(LogicalSize::new(initial_w as f64, initial_h as f64)));

    if let (Some(x), Some(y)) = (saved_state.x, saved_state.y) {
        if x >= -50 && y >= -50 && x < 5000 && y < 5000 {
            window_builder = window_builder.with_position(Position::Physical(PhysicalPosition::new(x, y)));
        }
    }

    if saved_state.is_maximized {
        window_builder = window_builder.with_maximized(true);
    }

    let window = window_builder.build(&event_loop)?;

    let window = Arc::new(window);
    let win_ipc = window.clone();

    let combined_html = HTML
        .replace("<link rel=\"stylesheet\" href=\"styles.css\">", &format!("<style>{}</style>", CSS))
        .replace("<script src=\"app.js\"></script>", &format!("<script>{}</script><script>{}</script>", ASSETS, JS));


    let rt = std::sync::Arc::new(tokio::runtime::Runtime::new()?);
    let rt_ipc = rt.clone();
    let rt_startup = rt.clone();

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
                    .header("content-type", mime)
                    .header("access-control-allow-origin", "*")
                    .body(bytes.into())
                    .unwrap()
            } else {
                wry::http::Response::builder()
                    .status(404)
                    .body(Vec::new().into())
                    .unwrap()
            }
        })
        .with_devtools(true)
        .with_html(&combined_html)

        .with_ipc_handler(move |req| {
            let rt = rt_ipc.clone();

            let body = req.body();
            eprintln!("[XODUS IPC] Received command: {body}");
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
                            record_window_state(&win_ipc, &win_state_ipc);
                        }
                        "close" => {
                            record_window_state(&win_ipc, &win_state_ipc);
                            std::process::exit(0);
                        }
                        "launch_game" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                let proxy_tokio = proxy_ipc.clone();
                                rt.spawn(async move {
                                    // 1. Check cloud save status
                                    let status_output = tokio::process::Command::new(find_xodus_cli())
                                        .arg("save")
                                        .arg("status")
                                        .arg("--json")
                                        .arg(&path_owned)
                                        .output()
                                        .await;
                                    
                                    let mut discrepancy = false;
                                    let mut local_info = String::from("No local saves found.");
                                    let mut cloud_info = String::from("No remote saves found.");
                                    
                                    if let Ok(out) = status_output {
                                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                                            if let Some(d) = json.get("discrepancy").and_then(|v| v.as_bool()) {
                                                discrepancy = d;
                                            }
                                            if let Some(local_arr) = json.get("local_blobs").and_then(|v| v.as_array()) {
                                                if !local_arr.is_empty() {
                                                    local_info = format!("{} files ({} bytes)", local_arr.len(), local_arr.iter().map(|f| f.get("size").and_then(|s| s.as_u64()).unwrap_or(0)).sum::<u64>());
                                                }
                                            }
                                            if let Some(remote_arr) = json.get("remote_blobs").and_then(|v| v.as_array()) {
                                                if !remote_arr.is_empty() {
                                                    cloud_info = format!("{} files ({} bytes)", remote_arr.len(), remote_arr.iter().map(|f| f.get("size").and_then(|s| s.as_u64()).unwrap_or(0)).sum::<u64>());
                                                }
                                            }
                                        }
                                    }

                                    if discrepancy {
                                        let script = format!("if (window.showCloudSyncDialog) window.showCloudSyncDialog('{}', '{}', '{}');", path_owned.replace('\'', "\\'"), local_info, cloud_info);
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    } else {
                                        let script = "if (window.showToast) window.showToast('Launching game...');";
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script.to_string()));

                                        // 2. Launch game immediately if no discrepancy
                                        let log_file = std::fs::File::create("/tmp/xodus-run.log").unwrap();
                                        let child = tokio::process::Command::new(find_xodus_cli())
                                            .arg("run")
                                            .arg(&path_owned)
                                            .stdout(std::process::Stdio::from(log_file.try_clone().unwrap()))
                                            .stderr(std::process::Stdio::from(log_file))
                                            .spawn();
                                        if let Ok(mut c) = child {
                                            let _ = c.wait().await;
                                        }
                                    }
                                });
                            }
                        }
                        "resolve_save_conflict" => {
                            if let (Some(path), Some(choice)) = (v.get("path").and_then(|p| p.as_str()), v.get("choice").and_then(|c| c.as_str())) {
                                let path_owned = path.to_string();
                                let choice_owned = choice.to_string();
                                let proxy_tokio = proxy_ipc.clone();
                                rt.spawn(async move {
                                    if choice_owned == "cloud" {
                                        let _ = tokio::process::Command::new(find_xodus_cli()).arg("save").arg("pull").arg(&path_owned).status().await;
                                    } else if choice_owned == "local" {
                                        let _ = tokio::process::Command::new(find_xodus_cli()).arg("save").arg("push").arg(&path_owned).status().await;
                                    }
                                    
                                    let script = "if (window.showToast) window.showToast('Launching game...');";
                                    let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script.to_string()));

                                    let log_file = std::fs::File::create("/tmp/xodus-run.log").unwrap();
                                    let child = tokio::process::Command::new(find_xodus_cli())
                                        .arg("run")
                                        .arg(&path_owned)
                                        .stdout(std::process::Stdio::from(log_file.try_clone().unwrap()))
                                        .stderr(std::process::Stdio::from(log_file))
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
                                    let _ = tokio::process::Command::new(find_xodus_cli())
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
                                    let _ = tokio::process::Command::new(find_xodus_cli())
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
                                            let _ = tokio::process::Command::new(find_xodus_cli())
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
                                run_hydrate_and_sync(tokens_clone, proxy_tokio, is_force_sync).await;
                            });
                        }
                        "login" => {
                            log::info!("Triggering Microsoft login flow...");
                            let tokens_clone = tokens_ipc.clone();
                            let proxy_tokio = proxy_ipc.clone();
                            rt.spawn(async move {
                                let child = tokio::process::Command::new(find_xodus_cli())
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
                            let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("Game").to_string();
                            let product_id = v.get("productId").or_else(|| v.get("id")).and_then(|p| p.as_str()).unwrap_or("").to_string();
                            let path = v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string();
                            log::info!("Handling install request for: {title} (ID: {product_id}, Path: {path})");

                            let proxy_tokio = proxy_ipc.clone();

                            rt.spawn(async move {
                                // Check if product_id is a 12-char BigID or lookup from catalog_cache in DB
                                let mut target_id = product_id.clone();
                                if target_id.len() != 12 || !target_id.chars().all(|c| c.is_alphanumeric()) {
                                    if let Ok(db) = xodus::db::Database::open_default() {
                                        if let Ok(products) = db.get_all_catalog_products() {
                                            for p in products {
                                                if (p.title.eq_ignore_ascii_case(&title) || normalize_title(&p.title) == normalize_title(&title)) && p.product_id.len() == 12 {
                                                    target_id = p.product_id;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                if target_id.len() != 12 || !target_id.chars().all(|c| c.is_alphanumeric()) {
                                    log::warn!("Cannot install {title}: Not a valid Windows PC Store BigID ({target_id})");
                                    let script = format!(
                                        "if (window.onInstallError) window.onInstallError('{}', 'This title is an Xbox console-only license and has no Windows PC MSIXVC package on the Microsoft Store.');",
                                        title.replace('\'', "\\'")
                                    );
                                    let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    return;
                                }

                                let safe_title: String = title.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_').collect();
                                let dest_dir = format!("/mnt/w11/XboxGames/{}", safe_title.trim());
                                let _ = tokio::fs::create_dir_all(&dest_dir).await;

                                let script = format!(
                                    "if (window.updateDownloadProgress) window.updateDownloadProgress('{}', 15, 'Streaming package chunks from Microsoft CDN...');",
                                    title.replace('\'', "\\'")
                                );
                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));

                                let cli_path = find_xodus_cli();
                                let child = tokio::process::Command::new(&cli_path)
                                    .arg("streaming")
                                    .arg(&target_id)
                                    .arg(&dest_dir)
                                    .spawn();

                                if let Ok(mut c) = child {
                                    let status = c.wait().await;
                                    if status.map(|s| s.success()).unwrap_or(false) && has_game_files(std::path::Path::new(&dest_dir)) {
                                        let script = format!(
                                            "if (window.onInstallComplete) window.onInstallComplete('{}', '{}');",
                                            title.replace('\'', "\\'"),
                                            dest_dir.replace('\'', "\\'")
                                        );
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    } else {
                                        if !has_game_files(std::path::Path::new(&dest_dir)) {
                                            let _ = tokio::fs::remove_dir_all(&dest_dir).await;
                                        }
                                        let script = format!(
                                            "if (window.onInstallError) window.onInstallError('{}', 'Package streaming download failed or was canceled.');",
                                            title.replace('\'', "\\'")
                                        );
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    }
                                } else {
                                    if !has_game_files(std::path::Path::new(&dest_dir)) {
                                        let _ = tokio::fs::remove_dir_all(&dest_dir).await;
                                    }
                                    let script = format!(
                                        "if (window.onInstallError) window.onInstallError('{}', 'Failed to launch xodus streaming downloader.');",
                                        title.replace('\'', "\\'")
                                    );
                                    let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                }
                            });
                        }
                        "uninstall_game" => {
                            let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("Game").to_string();
                            let path = v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string();
                            log::info!("Handling uninstall request for: {title} (Path: {path})");
                            let proxy_tokio = proxy_ipc.clone();

                            rt.spawn(async move {
                                let cli_path = find_xodus_cli();
                                let target_arg = if !path.is_empty() { path.clone() } else { title.clone() };

                                let child = tokio::process::Command::new(&cli_path)
                                    .arg("uninstall")
                                    .arg(&target_arg)
                                    .spawn();

                                if let Ok(mut c) = child {
                                    let status = c.wait().await;
                                    if status.map(|s| s.success()).unwrap_or(false) {
                                        let script = format!(
                                            "if (window.onUninstallComplete) window.onUninstallComplete('{}', '{}');",
                                            title.replace('\'', "\\'"),
                                            path.replace('\'', "\\'")
                                        );
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    } else {
                                        if !path.is_empty() && std::path::Path::new(&path).is_dir() {
                                            let _ = tokio::fs::remove_dir_all(&path).await;
                                            let script = format!(
                                                "if (window.onUninstallComplete) window.onUninstallComplete('{}', '{}');",
                                                title.replace('\'', "\\'"),
                                                path.replace('\'', "\\'")
                                            );
                                            let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                        } else {
                                            let script = format!(
                                                "if (window.showToast) window.showToast('Failed to uninstall {}');",
                                                title.replace('\'', "\\'")
                                            );
                                            let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                        }
                                    }
                                } else {
                                    if !path.is_empty() && std::path::Path::new(&path).is_dir() {
                                        let _ = tokio::fs::remove_dir_all(&path).await;
                                        let script = format!(
                                            "if (window.onUninstallComplete) window.onUninstallComplete('{}', '{}');",
                                            title.replace('\'', "\\'"),
                                            path.replace('\'', "\\'")
                                        );
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    } else {
                                        let script = format!(
                                            "if (window.showToast) window.showToast('Failed to start uninstaller for {}');",
                                            title.replace('\'', "\\'")
                                        );
                                        let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                    }
                                }
                            });
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
        let gtk_win = window.gtk_window();
        if let (Some(x), Some(y)) = (saved_state.x, saved_state.y) {
            if x >= -50 && y >= -50 && x < 5000 && y < 5000 {
                gtk_win.move_(x, y);
            }
        }
        if saved_state.is_maximized {
            gtk_win.maximize();
        } else {
            gtk_win.resize(initial_w as i32, initial_h as i32);
        }
        gtk_win.show_all();
        gtk_win.present();
        wv
    };
    window.set_focus();

    let proxy_startup = proxy.clone();
    let tokens_startup = tokens.clone();
    rt_startup.spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        run_hydrate_and_sync(tokens_startup, proxy_startup.clone(), false).await;
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                record_window_state(&window, &win_state_loop);
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Moved(pos),
                ..
            } => {
                if !window.is_maximized() {
                    let mut st = win_state_loop.lock().unwrap();
                    st.x = Some(pos.x);
                    st.y = Some(pos.y);
                    save_window_state(&st);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let is_max = window.is_maximized();
                let mut st = win_state_loop.lock().unwrap();
                st.is_maximized = is_max;
                if !is_max {
                    let scale = window.scale_factor().max(1.0);
                    let log_size = size.to_logical::<f64>(scale);
                    st.width = (log_size.width as u32).clamp(960, 3840);
                    st.height = (log_size.height as u32).clamp(600, 2160);
                }
                save_window_state(&st);
            }
            Event::UserEvent(CustomEvent::EvaluateScript(script)) => {
                if let Err(e) = webview.evaluate_script(&script) {
                    eprintln!("[XODUS] Failed to evaluate script: {e}");
                } else {
                    eprintln!("[XODUS] Successfully evaluated script (length {})", script.len());
                }
            }
            _ => (),
        }
    });

}
