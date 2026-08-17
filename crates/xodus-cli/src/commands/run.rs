use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use msixvc::xvd::{SegmentFile, XvdFile};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
#[cfg(target_os = "linux")]
use rustix::fs::{MemfdFlags, memfd_create};
#[cfg(not(target_os = "linux"))]
use tempfile::{tempdir, tempfile, tempfile_in};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use xodus::tokens::TokenManager;

use crate::license::get_license;

#[allow(dead_code)]
#[cfg(target_os = "linux")]
fn make_temp_file(_folder: &str) -> std::io::Result<std::fs::File> {
    let fd = memfd_create("xodus", MemfdFlags::CLOEXEC).map_err(std::io::Error::from)?;
    Ok(std::fs::File::from(fd))
}

#[cfg(not(target_os = "linux"))]
fn make_temp_file(folder: &str) -> std::io::Result<std::fs::File> {
    if folder.is_empty() {
        tempfile()
    } else {
        tempfile_in(folder)
    }
}

#[cfg(target_os = "macos")]
async fn prepare(lfiles: &HashMap<String, SegmentFile>) -> (impl AsyncFnOnce(), String) {
    let disk_size: u64 = lfiles
        .iter()
        .filter(|f| f.1.keep_encrypted)
        .map(|f| f.1.length + 4 * PAGE_SIZE as u64)
        .reduce(|o, s| o + s)
        .unwrap_or(PAGE_SIZE as u64);

    let device_s = String::from_utf8(
        Command::new("/usr/bin/hdiutil")
            .arg("attach")
            .arg("-nomount")
            .arg(format!("ram://{}", disk_size.div_ceil(256)))
            .output()
            .await
            .unwrap()
            .stdout,
    )
    .unwrap();

    let device = device_s.trim();

    let vol = uuid::Uuid::new_v4().to_string();

    let fmt = Command::new("/sbin/newfs_hfs")
        .arg("-v")
        .arg(vol)
        .arg(device)
        .status()
        .await
        .unwrap();
    assert!(fmt.success());

    let mount_dir_obj = tempdir().unwrap();
    let mount_dir = mount_dir_obj.path().to_str().unwrap();

    let mnt = Command::new("/sbin/mount")
        .arg("-t")
        .arg("hfs")
        .arg("-o")
        .arg("nobrowse")
        .arg("-v")
        .arg(device)
        .arg(mount_dir)
        .status()
        .await
        .unwrap();
    assert!(mnt.success());
    let mount_dir_cl = mount_dir.to_string();
    let device_cl = device.to_string();
    (
        async move || {
            let mnt = Command::new("/sbin/umount")
                .arg("-f")
                .arg(mount_dir_cl)
                .status()
                .await
                .unwrap();
            assert!(mnt.success());

            let mnt = Command::new("/usr/bin/hdiutil")
                .arg("detach")
                .arg("-force")
                .arg(&device_cl)
                .status()
                .await
                .unwrap();
            assert!(mnt.success());
        },
        mount_dir.to_owned(),
    )
}

#[allow(dead_code)]
#[cfg(not(target_os = "macos"))]
async fn prepare(_lfiles: &HashMap<String, SegmentFile>) -> (impl AsyncFnOnce(), String) {
    (async || {}, "".to_owned())
}

#[derive(Default, Debug)]
struct GameConfigInfo {
    executable: Option<String>,
    title_id: Option<String>,
    store_id: Option<String>,
    msa_app_id: Option<String>,
    identity_name: Option<String>,
}

fn parse_microsoft_game_config(content: &str) -> GameConfigInfo {
    let mut info = GameConfigInfo::default();

    // Strip comments <!-- ... --> before parsing lines
    let mut clean_content = String::new();
    let mut in_comment = false;
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if !in_comment && c == '<' {
            if chars.peek() == Some(&'!') {
                let mut temp = chars.clone();
                if temp.next() == Some('!') && temp.next() == Some('-') && temp.next() == Some('-') {
                    chars.next(); // '!'
                    chars.next(); // '-'
                    chars.next(); // '-'
                    in_comment = true;
                    continue;
                }
            }
            clean_content.push(c);
        } else if in_comment && c == '-' {
            let mut temp = chars.clone();
            if temp.next() == Some('-') && temp.next() == Some('>') {
                chars.next(); // '-'
                chars.next(); // '>'
                in_comment = false;
                continue;
            }
        } else if !in_comment {
            clean_content.push(c);
        }
    }

    for line in clean_content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<Executable") && trimmed.contains("Name=") {
            if let Some(start) = trimmed.find("Name=\"") {
                let rest = &trimmed[start + 6..];
                if let Some(end) = rest.find('"') {
                    let mut exe_name = rest[..end].replace('\\', "/");
                    if let Some(alias_start) = trimmed.find("Alias=\"") {
                        let alias_rest = &trimmed[alias_start + 7..];
                        if let Some(alias_end) = alias_rest.find('"') {
                            let alias_name = &alias_rest[..alias_end];
                            if !alias_name.is_empty() && exe_name.to_ascii_lowercase().contains("launcher") {
                                if let Some(last_slash) = exe_name.rfind('/') {
                                    exe_name = format!("{}/{}", &exe_name[..last_slash], alias_name);
                                } else {
                                    exe_name = alias_name.to_string();
                                }
                            }
                        }
                    }
                    if !exe_name.eq_ignore_ascii_case("gamelaunchhelper.exe") {
                        info.executable = Some(exe_name);
                    }
                }
            }
        }
        if trimmed.contains("<TitleId>") && trimmed.contains("</TitleId>") {
            if let Some(start) = trimmed.find("<TitleId>") {
                let rest = &trimmed[start + 9..];
                if let Some(end) = rest.find("</TitleId>") {
                    info.title_id = Some(rest[..end].trim().to_string());
                }
            }
        }
        if trimmed.contains("<StoreId>") && trimmed.contains("</StoreId>") {
            if let Some(start) = trimmed.find("<StoreId>") {
                let rest = &trimmed[start + 9..];
                if let Some(end) = rest.find("</StoreId>") {
                    info.store_id = Some(rest[..end].trim().to_string());
                }
            }
        }
        if trimmed.contains("<MSAAppId>") && trimmed.contains("</MSAAppId>") {
            if let Some(start) = trimmed.find("<MSAAppId>") {
                let rest = &trimmed[start + 10..];
                if let Some(end) = rest.find("</MSAAppId>") {
                    info.msa_app_id = Some(rest[..end].trim().to_string());
                }
            }
        }
        if trimmed.contains("<Identity") && trimmed.contains("Name=") {
            if let Some(start) = trimmed.find("Name=\"") {
                let rest = &trimmed[start + 6..];
                if let Some(end) = rest.find('"') {
                    info.identity_name = Some(rest[..end].to_string());
                }
            }
        }
    }

    info
}

async fn ensure_service_running() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_ | "/tmp".to_string());
    let socket_path = format!("{}/xodus.sock", runtime_dir);

    // If we have a game version set (launched via `xodus run`), we need to restart the service
    // so it picks up XODUS_GAME_VERSION and XODUS_PACKAGE_FAMILY_NAME for TVR embedding.
    // Otherwise a long-running service started before version detection won't have these vars.
    let has_game_version = std::env::var("XODUS_GAME_VERSION").is_ok();
    if has_game_version && tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
        // Service is running but might not have the game version env vars.
        // Kill it so we can restart with the correct env vars.
        log::info!("[XODUS-RUN] Restarting xodus-service to inject XODUS_GAME_VERSION={}", std::env::var("XODUS_GAME_VERSION").unwrap_or_default());
        // Kill existing service by removing the socket and sending SIGTERM to any xodus-service processes
        let _ = std::fs::remove_file(&socket_path);
        let _ = tokio::process::Command::new("pkill")
            .args(["-x", "xodus-service"])
            .spawn();
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    } else if !has_game_version && tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
        // Service is running and we don't need game-specific env vars (e.g., background tasks)
        return;
    }

    println!("Ensuring xodus-service background daemon is active...");
    let service_binary = {
        let home = std::env::var("HOME").unwrap_or_default();
        let local_bin = PathBuf::from(format!("{}/.local/bin/xodus-service", home));
        let release_target = PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xodus/target/release/xodus-service");
        let debug_target = PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xodus/target/debug/xodus-service");
        let exe_sibling = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("xodus-service")));

        if let Some(ref sib) = exe_sibling {
            if sib.exists() {
                Some(sib.clone())
            } else if release_target.exists() {
                Some(release_target)
            } else if debug_target.exists() {
                Some(debug_target)
            } else if local_bin.exists() {
                Some(local_bin)
            } else if let Ok(path_var) = std::env::var("PATH") {
                path_var.split(':').map(PathBuf::from).map(|p| p.join("xodus-service")).find(|p| p.exists())
            } else {
                None
            }
        } else if release_target.exists() {
            Some(release_target)
        } else if debug_target.exists() {
            Some(debug_target)
        } else if local_bin.exists() {
            Some(local_bin)
        } else if let Ok(path_var) = std::env::var("PATH") {
            path_var.split(':').map(PathBuf::from).map(|p| p.join("xodus-service")).find(|p| p.exists())
        } else {
            None
        }
    };


    if let Some(bin) = service_binary {
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/xodus-service.log")
            .ok();
        let mut cmd = tokio::process::Command::new(bin);
        cmd.env("RUST_LOG", "info,xodus=debug,xodus_service=debug");
        // Forward game version so the service can embed TitleVersion in the Xbox title token
        if let Ok(game_ver) = std::env::var("XODUS_GAME_VERSION") {
            cmd.env("XODUS_GAME_VERSION", &game_ver);
            log::info!("[XODUS-RUN] Starting xodus-service with XODUS_GAME_VERSION={}", game_ver);
        }
        if let Ok(pfn) = std::env::var("XODUS_PACKAGE_FAMILY_NAME") {
            cmd.env("XODUS_PACKAGE_FAMILY_NAME", &pfn);
            log::info!("[XODUS-RUN] Starting xodus-service with XODUS_PACKAGE_FAMILY_NAME={}", pfn);
        }
        if let Some(ref f) = log_file {
            if let Ok(f_clone) = f.try_clone() {
                cmd.stdout(std::process::Stdio::from(f_clone));
            }
            if let Ok(f_clone) = f.try_clone() {
                cmd.stderr(std::process::Stdio::from(f_clone));
            }
        }
        let _ = cmd.spawn();

        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
                println!("xodus-service daemon connected.");
                return;
            }
        }
    }
}

/// Launches a Microsoft Store / Xbox Game Pass game on Linux via Proton and XGameRuntime.
///
/// Workflow:
/// 1. Inspects game folder for `MicrosoftGame.config`, `AppxManifest.xml`, or MSIXVC/XVD containers.
/// 2. Derives license keys and starts the `xodus-service` background daemon.
/// 3. Automatically syncs cloud saves from Xbox Live before launch.
/// 4. Configures Wine prefix registry for GDK DLL overrides (`xgameruntime.dll`, `twinapi.appcore.dll`).
/// 5. Launches game executable under Proton with DXVK/VKD3D-Proton and MangoHud.
/// 6. Synchronizes updated local save data back to Xbox Live upon session completion.
/// Resolves a game source argument to an absolute directory path.
///
/// Supports:
/// 1. Direct path to a game directory (e.g. `/mnt/w11/XboxGames/Sea of Thieves`, `./Brotato`)
/// 2. Game Title / DisplayName (e.g. `xodus play "Sea of Thieves"`, `xodus play Brotato`)
/// 3. Microsoft Store Product ID / BigID (e.g. `xodus play 9P2N57MC619K`)
/// 4. Scanning standard Xbox library directories (`/mnt/w11/XboxGames`, `~/XboxGames`, `~/.local/share/xodus/installed`, etc.)
pub fn resolve_game_path(source: &str) -> Option<PathBuf> {
    let p = Path::new(source);
    if p.exists() {
        return std::fs::canonicalize(p).ok();
    }

    // List of candidate library root paths to search
    let mut search_roots = Vec::new();

    // 1. Saved custom storage path in database
    if let Ok(db) = xodus::db::Database::open_default() {
        if let Ok(Some(saved)) = db.get_setting("storage_path") {
            let trimmed = saved.trim();
            if !trimmed.is_empty() {
                if trimmed.starts_with("~/") {
                    if let Ok(home) = std::env::var("HOME") {
                        search_roots.push(PathBuf::from(home).join(&trimmed[2..]));
                    }
                } else if trimmed == "~" {
                    if let Ok(home) = std::env::var("HOME") {
                        search_roots.push(PathBuf::from(home));
                    }
                } else {
                    search_roots.push(PathBuf::from(trimmed));
                }
            }
        }
    }

    if let Ok(lib) = std::env::var("XODUS_LIBRARY_PATH").or_else(|_| std::env::var("XODUS_GAMES_PATH")) {
        for part in lib.split(':') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                if trimmed.starts_with("~/") {
                    if let Ok(home) = std::env::var("HOME") {
                        search_roots.push(PathBuf::from(home).join(&trimmed[2..]));
                    }
                } else {
                    search_roots.push(PathBuf::from(trimmed));
                }
            }
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let home_p = PathBuf::from(home);
        search_roots.push(home_p.join("Games"));
        search_roots.push(home_p.join("XboxGames"));
        search_roots.push(home_p.join(".local/share/xodus/installed"));
    }
    search_roots.push(PathBuf::from("/mnt/w11/XboxGames"));
    search_roots.push(PathBuf::from("/var/games/XboxGames"));

    // Also check any mounted media under /run/media
    if let Ok(entries) = std::fs::read_dir("/run/media") {
        for u in entries.flatten() {
            if let Ok(drives) = std::fs::read_dir(u.path()) {
                for d in drives.flatten() {
                    let cand_games = d.path().join("Games");
                    if cand_games.is_dir() {
                        search_roots.push(cand_games);
                    }
                    let cand_xbox = d.path().join("XboxGames");
                    if cand_xbox.is_dir() {
                        search_roots.push(cand_xbox);
                    }
                }
            }
        }
    }

    let source_clean = source.trim();
    let source_lower = source_clean.to_lowercase();
    let mut discovered_games = Vec::new();

    for root in &search_roots {
        if !root.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let folder_name = entry.file_name().to_string_lossy().to_string();
                let folder_lower = folder_name.to_lowercase();

                if folder_lower == "gamesave"
                    || folder_lower == "wgs"
                    || folder_lower.starts_with('.')
                    || folder_lower.starts_with('$')
                {
                    continue;
                }

                // Check exact folder name match
                if folder_lower == source_lower {
                    return std::fs::canonicalize(&path).ok();
                }

                let mut title_match = false;
                let mut id_match = false;
                let mut game_title = folder_name.clone();

                let config_paths = [
                    path.join("MicrosoftGame.config"),
                    path.join("MicrosoftGame.Config"),
                    path.join("Content").join("MicrosoftGame.config"),
                    path.join("Content").join("MicrosoftGame.Config"),
                ];
                for cp in &config_paths {
                    if cp.exists() {
                        if let Ok(content) = std::fs::read_to_string(cp) {
                            if let Some(s_pos) = content.find("<StoreId>") {
                                if let Some(e_pos) = content[s_pos..].find("</StoreId>") {
                                    let sid = content[s_pos + 9..s_pos + e_pos].trim();
                                    if sid.eq_ignore_ascii_case(source_clean) {
                                        id_match = true;
                                    }
                                }
                            }
                            if let Some(d_pos) = content.find("DefaultDisplayName=\"") {
                                if let Some(e_pos) = content[d_pos + 20..].find('"') {
                                    let dname = &content[d_pos + 20..d_pos + 20 + e_pos];
                                    game_title = dname.to_string();
                                    let d_lower = dname.to_lowercase();
                                    if d_lower == source_lower
                                        || d_lower.contains(&source_lower)
                                        || source_lower.contains(&d_lower)
                                    {
                                        title_match = true;
                                    }
                                }
                            }
                        }
                        break;
                    }
                }

                let manifest_paths = [
                    path.join("AppxManifest.xml"),
                    path.join("appxmanifest.xml"),
                    path.join("Content").join("AppxManifest.xml"),
                    path.join("Content").join("appxmanifest.xml"),
                ];
                for mp in &manifest_paths {
                    if mp.exists() {
                        if let Ok(content) = std::fs::read_to_string(mp) {
                            if let Some(d_pos) = content.find("<DisplayName>") {
                                if let Some(e_pos) = content[d_pos..].find("</DisplayName>") {
                                    let dname = content[d_pos + 13..d_pos + e_pos].trim();
                                    if !dname.starts_with("ms-resource:") {
                                        game_title = dname.to_string();
                                        let d_lower = dname.to_lowercase();
                                        if d_lower == source_lower
                                            || d_lower.contains(&source_lower)
                                            || source_lower.contains(&d_lower)
                                        {
                                            title_match = true;
                                        }
                                    }
                                }
                            }
                            if let Some(i_pos) = content.find("<Identity Name=\"") {
                                if let Some(e_pos) = content[i_pos + 16..].find('"') {
                                    let iname = &content[i_pos + 16..i_pos + 16 + e_pos];
                                    if iname.eq_ignore_ascii_case(source_clean) {
                                        id_match = true;
                                    }
                                }
                            }
                        }
                        break;
                    }
                }

                discovered_games.push(format!("  • {} ({})", game_title, path.display()));

                if id_match || title_match || folder_lower.contains(&source_lower) {
                    return std::fs::canonicalize(&path).ok();
                }
            }
        }
    }

    if !discovered_games.is_empty() {
        eprintln!("\n[XODUS] Discovered installed games on system:");
        for g in discovered_games {
            eprintln!("{}", g);
        }
        eprintln!();
    }

    None
}

/// Run / play an installed game with xodus wine / Proton.
pub async fn run(
    client: &reqwest::Client,
    tokens: &TokenManager,
    source: String,
    wine: String,
    exe: Option<String>,
    market: Option<String>,
) -> ExitCode {
    // NOTE: ensure_service_running() is called AFTER AppxManifest parsing so that
    // XODUS_GAME_VERSION and XODUS_PACKAGE_FAMILY_NAME are set before the service starts.

    let mut lfiles: HashMap<String, SegmentFile> = HashMap::new();

    let out_absolute = match resolve_game_path(&source) {
        Some(p) => {
            println!("[XODUS] Launching game from directory: {}", p.display());
            p
        }
        None => {
            eprintln!("[XODUS] Failed to resolve game path for '{}'.", source);
            eprintln!("Please specify a valid game folder path, game title, or product ID.");
            return ExitCode::FAILURE;
        }
    };

    // Determine execution directory (check if Content/ subfolder exists)
    let content_dir = if out_absolute.join("Content").is_dir() {
        out_absolute.join("Content")
    } else {
        out_absolute.clone()
    };

    let mut container_paths: Vec<PathBuf> = Vec::new();
    let mut checked_containers = std::collections::HashSet::new();

    let check_dirs = [&out_absolute, &content_dir];
    for dir in check_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = p.file_name().unwrap().to_string_lossy();
                    let is_container = name.ends_with(".msixvc")
                        || name.ends_with(".xvd")
                        || (name.len() == 36 && !name.contains('.'));
                    if is_container && !checked_containers.contains(&p) {
                        checked_containers.insert(p.clone());
                        container_paths.push(p);
                    }
                }
            }
        }
    }

    let mut exe = exe;
    let mut package_family_name = None;
    let mut parsed_title_id = None;
    let mut parsed_store_id = None;

    // Parse MicrosoftGame.config if present
    let config_path = if content_dir.join("MicrosoftGame.config").exists() {
        Some(content_dir.join("MicrosoftGame.config"))
    } else if content_dir.join("MicrosoftGame.Config").exists() {
        Some(content_dir.join("MicrosoftGame.Config"))
    } else if out_absolute.join("MicrosoftGame.config").exists() {
        Some(out_absolute.join("MicrosoftGame.config"))
    } else if out_absolute.join("MicrosoftGame.Config").exists() {
        Some(out_absolute.join("MicrosoftGame.Config"))
    } else {
        None
    };

    if let Some(cfg) = config_path {
        if let Ok(content) = tokio::fs::read_to_string(&cfg).await {
            let info = parse_microsoft_game_config(&content);
            println!("Parsed MicrosoftGame.config: {:?}", info);
            if info.title_id.is_some() {
                parsed_title_id = info.title_id.clone();
            }
            if info.store_id.is_some() {
                parsed_store_id = info.store_id.clone();
            }
            if exe.is_none() && info.executable.is_some() {
                exe = info.executable;
            }
            if let Some(id_name) = info.identity_name {
                package_family_name = Some(format!("{}_8wekyb3d8bbwe", id_name));
            }
        }
    }


    // Parse AppxManifest.xml if present
    let manifest_path = if content_dir.join("AppxManifest.xml").exists() {
        Some(content_dir.join("AppxManifest.xml"))
    } else if content_dir.join("appxmanifest.xml").exists() {
        Some(content_dir.join("appxmanifest.xml"))
    } else if out_absolute.join("AppxManifest.xml").exists() {
        Some(out_absolute.join("AppxManifest.xml"))
    } else if out_absolute.join("appxmanifest.xml").exists() {
        Some(out_absolute.join("appxmanifest.xml"))
    } else {
        None
    };

    if let Some(mp) = manifest_path {
        if let Ok(content) = tokio::fs::read_to_string(&mp).await {
            if parsed_title_id.is_none() {
                if let Some(tid) = msixvc::manifest::AppxManifest::extract_title_id(&content) {
                    println!("Detected Xbox Title ID from AppxManifest: {}", tid);
                    parsed_title_id = Some(tid);
                }
            }
            if let Ok(manifest) = msixvc::manifest::AppxManifest::parse(&content) {
                let pfn = manifest.package_family_name();
                println!(
                    "Detected Package Identity: {} (v{}) [Family: {}]",
                    manifest.identity.name, manifest.identity.version, pfn
                );
                // Pass the game version and package family name as env vars so xodus-service can embed
                // TitleVersion + PackageFamilyName in the Xbox title token for the Athena TVR claim.
                if !manifest.identity.version.is_empty() {
                    unsafe {
                        std::env::set_var("XODUS_GAME_VERSION", &manifest.identity.version);
                    }
                    log::info!("[XODUS-RUN] Set XODUS_GAME_VERSION={}", manifest.identity.version);
                }
                let pfn = manifest.package_family_name();
                unsafe {
                    std::env::set_var("XODUS_PACKAGE_FAMILY_NAME", &pfn);
                }
                log::info!("[XODUS-RUN] Set XODUS_PACKAGE_FAMILY_NAME={}", pfn);
                package_family_name = Some(pfn);
                if exe.is_none() {
                    if let Some(target_exe) = manifest.primary_executable() {
                        if !target_exe.eq_ignore_ascii_case("gamelaunchhelper.exe") {
                            exe = Some(target_exe.to_string());
                        }
                    }
                }
            }
        }
    }

    // Start (or reconnect to) the xodus-service NOW that XODUS_GAME_VERSION and
    // XODUS_PACKAGE_FAMILY_NAME are set from the AppxManifest, so the service receives them.
    ensure_service_running().await;

    // If exe is still none, search directory for main binary
    if exe.is_none() {
        if let Ok(entries) = std::fs::read_dir(&content_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()).map_or(false, |ext| ext.eq_ignore_ascii_case("exe")) {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    if !name.eq_ignore_ascii_case("gamelaunchhelper.exe") {
                        println!("Auto-selected main executable: {}", name);
                        exe = Some(name);
                        break;
                    }
                }
            }
        }
    }

    let mut target_exe_name = exe.unwrap_or_else(|| "Game.exe".to_string()).replace('\\', "/");
    if target_exe_name.eq_ignore_ascii_case("Minecraft.exe") && (content_dir.join("Minecraft.Windows.exe").exists() || out_absolute.join("Minecraft.Windows.exe").exists()) {
        println!("Selecting direct game engine binary: Minecraft.Windows.exe");
        target_exe_name = "Minecraft.Windows.exe".to_string();
    }
    let target_exe_path = content_dir.join(&target_exe_name);
    println!("Target executable path: {:?}", target_exe_path);

    // Check if the target executable is already a plaintext PE32 binary (starts with MZ)
    let is_plaintext = if target_exe_path.exists() {
        if let Ok(mut f) = File::open(&target_exe_path).await {
            let mut magic = [0u8; 2];
            f.read_exact(&mut magic).await.map(|_| magic == [0x4D, 0x5A]).unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    let dll_overrides = std::env::var("WINEDLLOVERRIDES")
        .unwrap_or_else(|_| "xgameruntime=n,b".to_string());

    let classes = [
        "Windows.Foundation.Metadata.ApiInformation",
        "Windows.ApplicationModel.AppService.AppServiceConnection",
        "Windows.ApplicationModel.Package",
        "Windows.ApplicationModel.DataTransfer.DataTransferManager",
        "Windows.ApplicationModel.DataTransfer.Clipboard",
        "Windows.UI.Text.Core.CoreTextServicesManager",
        "Windows.System.Profile.AnalyticsInfo",
        "Windows.System.Profile.PlatformDiagnosticsAndUsageDataSettings",
        "Windows.Graphics.Display.DisplayInformation",
        "Windows.UI.ViewManagement.UIViewSettings",
        "Windows.UI.ViewManagement.ApplicationView",
        "Windows.UI.ViewManagement.InputPane",
        "Windows.UI.ViewManagement.StatusBar",
        "Windows.UI.ViewManagement.Core.CoreInputView",
        "Windows.Gaming.Preview.GamesEnumeration.GameList",
        "Windows.Gaming.XboxLive.Storage.GameSaveProvider",
        "Windows.Internal.System.Profile.RegionPolicyEvaluator",
        "Windows.ApplicationModel.Core.CoreApplication",
        "Windows.UI.Core.CoreWindow",
    ];

    for class_id in classes {
        let reg_key = format!("HKLM\\Software\\Microsoft\\WindowsRuntime\\ActivatableClassId\\{}", class_id);
        let _ = std::process::Command::new(&wine)
            .args(["reg", "add", &reg_key, "/v", "DllPath", "/t", "REG_SZ", "/d", "C:\\windows\\system32\\xgameruntime.dll", "/f"])
            .status();
        let _ = std::process::Command::new(&wine)
            .args(["reg", "add", &reg_key, "/v", "ActivationType", "/t", "REG_DWORD", "/d", "0", "/f"])
            .status();
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let local_xodus_lib = format!("{}/.local/lib/xodus", home);
    let repo_xodus_lib = "/run/media/noct/ssd1/Repo/other/xodus/xgameruntime".to_string();
    let rdir = std::env::var("XODUS_RUNTIME_PATH").unwrap_or_default();

    let wine_dll_path = match std::env::var("WINEDLLPATH") {
        Ok(paths) => format!("{}:{}:{}:{}", rdir, local_xodus_lib, repo_xodus_lib, paths),
        Err(_) => format!("{}:{}:{}", rdir, local_xodus_lib, repo_xodus_lib),
    };

    if is_plaintext || container_paths.is_empty() {
        println!("Launching in-place executable directly with Wine: {:?}", target_exe_path);
        let mut cmd = Command::new("wine");
        cmd.arg(content_dir.join(&target_exe_name))
           .current_dir(&content_dir)
           .env("WINEDLLOVERRIDES", &dll_overrides)
           .env("WINEDLLPATH", &wine_dll_path);


        if let Some(pfn) = package_family_name {
            cmd.env("LOCAL_APP_MODEL_PACKAGE_FAMILY_NAME", pfn);
        }
        if let Some(tid) = parsed_title_id {
            cmd.env("XODUS_TITLE_ID", tid);
        }


        let mut wn = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to spawn Wine process: {}", e);
                return ExitCode::FAILURE;
            }
        };

        let pid = wn.id().unwrap_or(0);
        if pid > 0 {
            let _ = ctrlc::set_handler(move || {
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGINT);
            });
        }

        let status = match wn.wait().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error waiting for Wine process: {}", e);
                return ExitCode::FAILURE;
            }
        };

        return if status.success() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }

    let mut decrypted_exes: Vec<(String, PathBuf)> = Vec::new();
    let mut primary_title_id_str = parsed_title_id.clone().unwrap_or_default();

    for final_path in &container_paths {
        println!("Found XVD/MSIXVC container: {:?}", final_path);
        let mut file = match OpenOptions::new().read(true).open(final_path.to_owned()).await {
            Ok(f) => f,
            Err(e) => {
                println!("Failed to open container {:?}: {}", final_path, e);
                continue;
            }
        };

        let xvd = match XvdFile::parse(&mut file).await {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to parse XVD header in {:?}: {}", final_path, e);
                continue;
            }
        };

        let mut cont_lfiles: HashMap<String, SegmentFile> = HashMap::new();
        if let Ok(files) = xvd.parse_user_package_files(&mut file).await {
            for (k, v) in &files {
                if k == "SegmentMetadata.bin" {
                    if let Ok(sfiles) = xvd.parse_segment_metadata(&mut file, v).await {
                        cont_lfiles = sfiles;
                    }
                }
            }
        }

        if cont_lfiles.is_empty() {
            if let Ok(sfiles) = xvd.parse_ntfs_segment_metadata(&mut file, !cont_lfiles.is_empty()).await {
                cont_lfiles.extend(sfiles);
            }
        }

        let cid = xvd.content_id().to_string();
        if primary_title_id_str.is_empty() {
            primary_title_id_str = cid.clone();
        }

        let cache_dir = PathBuf::from(format!("{}/.cache/xodus/bin/{}", home, primary_title_id_str));
        tokio::fs::create_dir_all(&cache_dir).await.ok();

        let license = get_license(
            client,
            tokens,
            cid.clone(),
            market.clone().unwrap_or("neutral".to_string()),
        ).await;

        let full_key = match license {
            Ok((key, game_splicense)) => {
                if let Some((_, content_key)) = game_splicense.content_keys.into_iter().next() {
                    match content_key.unpack(&key) {
                        Ok(k) => k,
                        Err(e) => {
                            println!("Failed to unpack content key for {}: {}", cid, e);
                            continue;
                        }
                    }
                } else {
                    println!("No content keys found for {}", cid);
                    continue;
                }
            }
            Err(e) => {
                println!("Failed to get license for container {}: {}", cid, e);
                continue;
            }
        };

        println!("Encrypted section infos count in {}: {}", cid, xvd.encrypted_section_infos.len());
        for (i, s) in xvd.encrypted_section_infos.iter().enumerate() {
            println!("  Section #{}: offset={}, length={}, data_units={:?}", i, s.section_offset, s.section_length, s.data_units.as_ref().map(|u| u.len()));
        }

        // Decrypt all .exe segments present in this package into cache_dir
        for (k, sfile) in &cont_lfiles {
            let norm_k = k.replace('\\', "/");
            if norm_k.to_ascii_lowercase().ends_with(".exe") {
                let rel_path = norm_k.trim_start_matches('/').to_string();
                let dest_exe = cache_dir.join(&rel_path);
                if let Some(parent) = dest_exe.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let should_extract = !dest_exe.exists() || std::fs::metadata(&dest_exe).map(|m| m.len() == 0).unwrap_or(true);
                if should_extract {
                    println!("Decrypting executable segment {} from package {}: length={}", rel_path, cid, sfile.length);
                    match File::create(&dest_exe).await {
                        Ok(mut out_f) => {
                            let mut resolved_src = None;
                            let mut current = content_dir.clone();
                            let parts: Vec<&str> = rel_path.split('/').collect();
                            let mut found_all = true;
                            for part in parts {
                                let mut found_part = false;
                                if let Ok(entries) = std::fs::read_dir(&current) {
                                    for entry in entries.flatten() {
                                        if entry.file_name().to_string_lossy().to_lowercase() == part.to_lowercase() {
                                            current = current.join(entry.file_name());
                                            found_part = true;
                                            break;
                                        }
                                    }
                                }
                                if !found_part {
                                    found_all = false;
                                    break;
                                }
                            }
                            if found_all {
                                resolved_src = Some(current.clone());
                                println!("Resolved path: {:?}", current);
                            } else {
                                println!("Failed to resolve path for: {}", rel_path);
                            }

                            if let Some(full_src) = resolved_src {
                                println!("Opening full_src: {:?}", full_src);
                                match File::open(&full_src).await {
                                    Ok(mut src_f) => {
                                        println!("Successfully opened full_src");
                                        if let Err(e) = xvd.mount_mem_fd(&mut src_f, &mut out_f, sfile, *full_key, |_, _| {}).await {
                                            println!("Extract error: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to open full_src: {}", e);
                                    }
                                }
                            } else {
                                println!("Opening final_path fallback");
                                if let Ok(mut src_f) = File::open(final_path).await {
                                    if let Err(e) = xvd.extract_file(&mut src_f, &mut out_f, sfile, *full_key, |_, _| {}).await {
                                        println!("Extract error: {}", e);
                                    }
                                }
                            }
                            use tokio::io::AsyncWriteExt;
                            out_f.flush().await.ok();
                        }
                        Err(e) => {
                            println!("Failed to create dest_exe {:?}: {}", dest_exe, e);
                        }
                    }
                }
                if !decrypted_exes.iter().any(|(r, _)| r.eq_ignore_ascii_case(&rel_path)) {
                    decrypted_exes.push((rel_path, dest_exe));
                }
            }
        }
    }

    let title_id_str = if !primary_title_id_str.is_empty() {
        primary_title_id_str
    } else {
        parsed_title_id.clone().unwrap_or_else(|| "game".to_string())
    };
    let cache_dir = PathBuf::from(format!("{}/.cache/xodus/bin/{}", home, title_id_str));
    let cached_exe = cache_dir.join(&target_exe_name);

    let run_dir = PathBuf::from(format!("{}/.cache/xodus/run/{}", home, title_id_str));
    tokio::fs::create_dir_all(&run_dir).await.ok();

    // Zero-copy symlink all assets and configs from Content into run_dir, preserving decrypted binaries
    if let Ok(entries) = std::fs::read_dir(&content_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let file_name = p.file_name().unwrap();
            let dest = run_dir.join(file_name);
            if file_name.to_string_lossy() == target_exe_name {
                continue;
            }
            
            if p.is_dir() {
                // Check if any decrypted binary lives under this subfolder
                let sub_folder_name = file_name.to_string_lossy().to_lowercase();
                let has_sub_decrypted = decrypted_exes.iter().any(|(rel, _)| rel.to_lowercase().starts_with(&sub_folder_name));
                if has_sub_decrypted {
                    let _ = tokio::fs::create_dir_all(&dest).await;
                    if let Ok(sub_entries) = std::fs::read_dir(&p) {
                        for sub_entry in sub_entries.flatten() {
                            let sp = sub_entry.path();
                            let s_name = sp.file_name().unwrap();
                            let s_dest = dest.join(s_name);
                            if sp.is_dir() {
                                let _ = tokio::fs::create_dir_all(&s_dest).await;
                                if let Ok(nested_entries) = std::fs::read_dir(&sp) {
                                    for nested in nested_entries.flatten() {
                                        let np = nested.path();
                                        let n_name = np.file_name().unwrap();
                                        let n_dest = s_dest.join(n_name);
                                        let n_name_lower = n_name.to_string_lossy().to_lowercase();
                                        let matched_decrypted = decrypted_exes.iter().find(|(rel, _)| {
                                            rel.to_lowercase().ends_with(&n_name_lower)
                                        });
                                        if let Some((_, dec_path)) = matched_decrypted {
                                            let _ = tokio::fs::remove_file(&n_dest).await;
                                            #[cfg(unix)]
                                            let _ = std::os::unix::fs::symlink(dec_path, &n_dest);
                                        } else if !n_dest.exists() {
                                            #[cfg(unix)]
                                            let _ = std::os::unix::fs::symlink(&np, &n_dest);
                                        }
                                    }
                                }
                            } else {
                                let s_name_lower = s_name.to_string_lossy().to_lowercase();
                                let matched_decrypted = decrypted_exes.iter().find(|(rel, _)| {
                                    rel.to_lowercase().ends_with(&s_name_lower)
                                });
                                if let Some((_, dec_path)) = matched_decrypted {
                                    let _ = tokio::fs::remove_file(&s_dest).await;
                                    #[cfg(unix)]
                                    let _ = std::os::unix::fs::symlink(dec_path, &s_dest);
                                } else if !s_dest.exists() {
                                    #[cfg(unix)]
                                    let _ = std::os::unix::fs::symlink(&sp, &s_dest);
                                }
                            }
                        }
                    }
                } else if !dest.exists() {
                    #[cfg(unix)]
                    let _ = std::os::unix::fs::symlink(&p, &dest);
                }
            } else if !dest.exists() {
                #[cfg(unix)]
                let _ = std::os::unix::fs::symlink(&p, &dest);
            }
        }
    }

    // Patch EasyAntiCheat/Settings.json to use a proper Windows drive letter (fixes EAC rejection of Z: drive)
    // The core EAC problem: EAC refuses to launch executables on Wine's Z: drive (Linux root).
    // Wine always maps Z: to /. Games must appear to be on C: or another lettered drive.
    // Solution: create a dosdevices drive mapping (x:) in the Proton compat prefix that points
    // to content_dir, so EAC sees the game at X:\Athena\Binaries\WinGDK\SotGame.exe — a
    // proper Windows path on a proper Windows drive letter.
    let compat_title_for_eac = parsed_title_id.as_deref().unwrap_or(&title_id_str);
    let compat_data_for_eac = PathBuf::from(format!("{}/.local/share/xodus/compatdata/{}", home, compat_title_for_eac));
    let dosdevices = compat_data_for_eac.join("pfx").join("dosdevices");
    tokio::fs::create_dir_all(&dosdevices).await.ok();

    // Create drive mapping: x: -> content_dir (the game's actual installation directory)
    let xdrive = dosdevices.join("x:");
    if xdrive.is_symlink() {
        let _ = std::fs::remove_file(&xdrive);
    }
    if !xdrive.exists() {
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&content_dir, &xdrive);
    }
    let _game_drive_letter = 'X';    // Patch EasyAntiCheat/Settings.json to use pure Windows paths (fixes EAC Launch Error)
    let eac_dir = run_dir.join("EasyAntiCheat");
    let content_eac_dir = content_dir.join("EasyAntiCheat");
    if content_eac_dir.exists() {
        if eac_dir.is_symlink() {
            let _ = std::fs::remove_file(&eac_dir);
        }
        let _ = std::fs::create_dir_all(&eac_dir);
        if let Ok(entries) = std::fs::read_dir(&content_eac_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let file_name = p.file_name().unwrap();
                let dest = eac_dir.join(file_name);
                if file_name.to_string_lossy().to_lowercase() != "settings.json" {
                    #[cfg(unix)]
                    let _ = std::os::unix::fs::symlink(&p, &dest);
                }
            }
        }
        
        let p = content_eac_dir.join("Settings.json");
        let dest = eac_dir.join("Settings.json");
        if let Ok(content) = std::fs::read_to_string(&p) {
            let mut modified_content = content.clone();
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(exe) = json.get("executable").and_then(|v| v.as_str()) {
                    let fixed = exe.replace('/', "\\\\");
                    json["executable"] = serde_json::Value::String(fixed.clone());
                    json["wait_for_game_process_exit"] = serde_json::Value::String("true".to_string());
                    modified_content = serde_json::to_string_pretty(&json).unwrap();
                    
                    let parts: Vec<&str> = fixed.split('\\').collect();
                    let mut current_run = run_dir.clone();
                    let mut current_content = content_dir.clone();
                    for (i, part) in parts.iter().enumerate() {
                        current_run = current_run.join(part);
                        current_content = current_content.join(part);
                        
                        if i < parts.len() - 1 {
                            // It's a directory component
                            if current_run.is_symlink() {
                                let _ = std::fs::remove_file(&current_run);
                            }
                            if !current_run.exists() {
                                let _ = std::fs::create_dir_all(&current_run);
                                // Symlink other files so we don't lose them
                                if let Ok(entries) = std::fs::read_dir(&current_content) {
                                    for entry in entries.flatten() {
                                        let p2 = entry.path();
                                        let file_name = p2.file_name().unwrap();
                                        let dest2 = current_run.join(file_name);
                                        if file_name.to_string_lossy().to_lowercase() != parts[i+1].to_lowercase() {
                                            #[cfg(unix)]
                                            let _ = std::os::unix::fs::symlink(&p2, &dest2);
                                        }
                                    }
                                }
                            }
                        } else {
                            // It's the executable file
                            if current_run.is_symlink() {
                                let _ = std::fs::remove_file(&current_run);
                            }
                            if !current_run.exists() {
                                // COPY the decrypted executable
                                let mut found_decrypted = false;
                                for (_, dec_path) in &decrypted_exes {
                                    if dec_path.file_name().unwrap().to_string_lossy().to_lowercase() == part.to_lowercase() {
                                        let _ = std::fs::copy(dec_path, &current_run);
                                        found_decrypted = true;
                                        break;
                                    }
                                }
                                if !found_decrypted {
                                    let _ = std::fs::copy(&current_content, &current_run);
                                }
                            }
                        }
                    }
                }
            }
            let _ = std::fs::write(&dest, modified_content);
        }
    } 

    let game_binary_in_run = run_dir.join(&target_exe_name);
    if let Some(parent) = game_binary_in_run.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    if cached_exe.exists() {
        let _ = tokio::fs::remove_file(&game_binary_in_run).await;
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&cached_exe, &game_binary_in_run);
    }

    // Deploy GDK runtime DLLs directly into run_dir, executable directories, and prefix system32
    let repo_runtime_dir = "/run/media/noct/ssd1/Repo/other/xodus/xgameruntime";
    let default_runtime_dir = format!("{}/.local/share/xodus/runtime", home);
    let runtime_dir = if Path::new(repo_runtime_dir).join("xgameruntime.dll").exists() {
        repo_runtime_dir.to_string()
    } else {
        default_runtime_dir
    };
    let dll_names = [
        "xgameruntime.dll", "xgameruntime.dll.so", "xgameruntime.so",
        "twinapi.appcore.dll", "twinapi.appcore.dll.so", "twinapi.appcore.so",
        "api-ms-win-core-psm-appnotify-l1-1-0.dll", "api-ms-win-core-psm-appnotify-l1-1-0.dll.so", "api-ms-win-core-psm-appnotify-l1-1-0.so",
        "windows.ui.core.textinput.dll", "windows.ui.core.textinput.dll.so", "windows.ui.core.textinput.so",
        "wintypes.dll", "wintypes.dll.so", "wintypes.so",
        "Microsoft.WindowsAppRuntime.Bootstrap.dll"
    ];
    for dll in &dll_names {
        let src_dll = Path::new(&runtime_dir).join(dll);
        if src_dll.exists() {
            let _ = std::fs::copy(&src_dll, run_dir.join(dll));
            for (_, dec_path) in &decrypted_exes {
                if let Some(p) = dec_path.parent() {
                    let _ = std::fs::copy(&src_dll, p.join(dll));
                }
            }
            let sys32 = compat_data_for_eac.join("pfx").join("drive_c").join("windows").join("system32");
            if sys32.exists() {
                let _ = std::fs::copy(&src_dll, sys32.join(dll));
            }
        }
    }

    // exec_target MUST be the decrypted binary in run_dir (symlink to cached decrypted PE).
    // content_dir has the encrypted GDK binaries which Wine cannot execute (error 193).
    let exec_target = game_binary_in_run.clone();
    println!("exec_target (decrypted run_dir): {:?}", exec_target);

    let proton_binary = {
        let mut candidates = Vec::new();
        if let Ok(p) = std::env::var("PROTON_PATH") {
            candidates.push(PathBuf::from(p));
        }
        candidates.push(PathBuf::from("/usr/share/steam/compatibilitytools.d/proton-cachyos-native/proton"));
        candidates.push(PathBuf::from(format!("{}/.local/share/Steam/compatibilitytools.d/proton-cachyos-native/proton", home)));
        candidates.push(PathBuf::from("/usr/share/steam/compatibilitytools.d/GE-Proton11-3/proton"));

        // Scan Steam & system compatibilitytools.d directories
        for base in [
            format!("{}/.local/share/Steam/compatibilitytools.d", home),
            format!("{}/.steam/root/compatibilitytools.d", home),
            format!("{}/.steam/steam/compatibilitytools.d", home),
            "/usr/share/steam/compatibilitytools.d".to_string(),
        ] {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let proton_bin = entry.path().join("proton");
                    if proton_bin.exists() {
                        candidates.push(proton_bin);
                    }
                }
            }
        }

        // Scan standard Steam Proton installations (Experimental, 9.0, 8.0, GE-Proton, Flatpak)
        for base in [
            format!("{}/.local/share/Steam/steamapps/common", home),
            format!("{}/.steam/root/steamapps/common", home),
            format!("{}/.steam/steam/steamapps/common", home),
            format!("{}/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common", home),
        ] {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.to_ascii_lowercase().starts_with("proton") {
                        let proton_bin = entry.path().join("proton");
                        if proton_bin.exists() {
                            candidates.push(proton_bin);
                        }
                    }
                }
            }
        }

        candidates.into_iter().find(|p| p.exists())
    };    // Locate bundled GDK and AppCore helper libraries from AppImage or system
    let gdk_dll = Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/xgameruntime.dll"))
        .filter(|p| p.exists())
        .or_else(|| {
            std::env::var("XODUS_RUNTIME_PATH")
                .ok()
                .map(|p| PathBuf::from(p).join("xgameruntime.dll"))
                .filter(|p| p.exists())
        })
        .or_else(|| {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../lib/xgameruntime.dll"))).filter(|p| p.exists())
        })
        .or_else(|| Some(PathBuf::from("/usr/lib/xodus/xgameruntime.dll")).filter(|p| p.exists()));

    let twinapi_dll = Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/twinapi.appcore.dll"))
        .filter(|p| p.exists())
        .or_else(|| {
            std::env::var("XODUS_RUNTIME_PATH")
                .ok()
                .map(|p| PathBuf::from(p).join("twinapi.appcore.dll"))
                .filter(|p| p.exists())
        })
        .or_else(|| {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../lib/twinapi.appcore.dll"))).filter(|p| p.exists())
        })
        .or_else(|| Some(PathBuf::from("/usr/lib/xodus/twinapi.appcore.dll")).filter(|p| p.exists()));

    let appnotify_dll = Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/api-ms-win-core-psm-appnotify-l1-1-0.dll"))
        .filter(|p| p.exists())
        .or_else(|| {
            std::env::var("XODUS_RUNTIME_PATH")
                .ok()
                .map(|p| PathBuf::from(p).join("api-ms-win-core-psm-appnotify-l1-1-0.dll"))
                .filter(|p| p.exists())
        })
        .or_else(|| {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../lib/api-ms-win-core-psm-appnotify-l1-1-0.dll"))).filter(|p| p.exists())
        })
        .or_else(|| Some(PathBuf::from("/usr/lib/xodus/api-ms-win-core-psm-appnotify-l1-1-0.dll")).filter(|p| p.exists()));

    let textinput_dll = Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/windows.ui.core.textinput.dll"))
        .filter(|p| p.exists())
        .or_else(|| {
            std::env::var("XODUS_RUNTIME_PATH")
                .ok()
                .map(|p| PathBuf::from(p).join("windows.ui.core.textinput.dll"))
                .filter(|p| p.exists())
        })
        .or_else(|| {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../lib/windows.ui.core.textinput.dll"))).filter(|p| p.exists())
        })
        .or_else(|| Some(PathBuf::from("/usr/lib/xodus/windows.ui.core.textinput.dll")).filter(|p| p.exists()));

    let wintypes_dll = Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/wintypes.dll"))
        .filter(|p| p.exists())
        .or_else(|| {
            std::env::var("XODUS_RUNTIME_PATH")
                .ok()
                .map(|p| PathBuf::from(p).join("wintypes.dll"))
                .filter(|p| p.exists())
        })
        .or_else(|| {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../lib/wintypes.dll"))).filter(|p| p.exists())
        })
        .or_else(|| Some(PathBuf::from("/usr/lib/xodus/wintypes.dll")).filter(|p| p.exists()));

    let target_sub_dirs = [
        run_dir.clone(),
        cached_exe.parent().unwrap_or(&run_dir).to_path_buf(),
        game_binary_in_run.parent().unwrap_or(&run_dir).to_path_buf(),
        run_dir.join("Retail"),
        run_dir.join("Athena").join("Binaries").join("WinGDK"),
    ];

    for dir in &target_sub_dirs {
        if dir.exists() {
            if let Some(ref gdk) = gdk_dll {
                let target_dll = dir.join("xgameruntime.dll");
                let _ = tokio::fs::remove_file(&target_dll).await;
                let _ = tokio::fs::copy(gdk, &target_dll).await;

                let gdk_so = gdk.with_extension("dll.so");
                if gdk_so.exists() {
                    let target_dll_so = dir.join("xgameruntime.dll.so");
                    let target_so = dir.join("xgameruntime.so");
                    let _ = tokio::fs::remove_file(&target_dll_so).await;
                    let _ = tokio::fs::remove_file(&target_so).await;
                    let _ = tokio::fs::copy(&gdk_so, &target_dll_so).await;
                    let _ = tokio::fs::copy(&gdk_so, &target_so).await;
                }
            }
            if let Some(ref twinapi) = twinapi_dll {
                let target_twinapi = dir.join("twinapi.appcore.dll");
                let _ = tokio::fs::remove_file(&target_twinapi).await;
                let _ = tokio::fs::copy(twinapi, &target_twinapi).await;

                let twinapi_so = twinapi.with_extension("dll.so");
                if twinapi_so.exists() {
                    let target_twinapi_so = dir.join("twinapi.appcore.dll.so");
                    let target_twinapi_short_so = dir.join("twinapi.appcore.so");
                    let _ = tokio::fs::remove_file(&target_twinapi_so).await;
                    let _ = tokio::fs::remove_file(&target_twinapi_short_so).await;
                    let _ = tokio::fs::copy(&twinapi_so, &target_twinapi_so).await;
                    let _ = tokio::fs::copy(&twinapi_so, &target_twinapi_short_so).await;
                }
            }
            if let Some(ref textinput) = textinput_dll {
                let target_textinput = dir.join("windows.ui.core.textinput.dll");
                let _ = tokio::fs::remove_file(&target_textinput).await;
                let _ = tokio::fs::copy(textinput, &target_textinput).await;

                let textinput_so = textinput.with_extension("dll.so");
                if textinput_so.exists() {
                    let target_textinput_so = dir.join("windows.ui.core.textinput.dll.so");
                    let target_textinput_short_so = dir.join("windows.ui.core.textinput.so");
                    let _ = tokio::fs::remove_file(&target_textinput_so).await;
                    let _ = tokio::fs::remove_file(&target_textinput_short_so).await;
                    let _ = tokio::fs::copy(&textinput_so, &target_textinput_so).await;
                    let _ = tokio::fs::copy(&textinput_so, &target_textinput_short_so).await;
                }
            }
            if let Some(ref wintypes) = wintypes_dll {
                let target_wintypes = dir.join("wintypes.dll");
                let _ = tokio::fs::remove_file(&target_wintypes).await;
                let _ = tokio::fs::copy(wintypes, &target_wintypes).await;

                let wintypes_so = wintypes.with_extension("dll.so");
                if wintypes_so.exists() {
                    let target_wintypes_so = dir.join("wintypes.dll.so");
                    let target_wintypes_short_so = dir.join("wintypes.so");
                    let _ = tokio::fs::remove_file(&target_wintypes_so).await;
                    let _ = tokio::fs::remove_file(&target_wintypes_short_so).await;
                    let _ = tokio::fs::copy(&wintypes_so, &target_wintypes_so).await;
                    let _ = tokio::fs::copy(&wintypes_so, &target_wintypes_short_so).await;
                }
            }
            if let Some(ref appnotify) = appnotify_dll {
                let target_appnotify = dir.join("api-ms-win-core-psm-appnotify-l1-1-0.dll");
                let _ = tokio::fs::remove_file(&target_appnotify).await;
                let _ = tokio::fs::copy(appnotify, &target_appnotify).await;

                let appnotify_so = appnotify.with_extension("dll.so");
                if appnotify_so.exists() {
                    let target_appnotify_so = dir.join("api-ms-win-core-psm-appnotify-l1-1-0.dll.so");
                    let target_appnotify_short_so = dir.join("api-ms-win-core-psm-appnotify-l1-1-0.so");
                    let _ = tokio::fs::remove_file(&target_appnotify_so).await;
                    let _ = tokio::fs::remove_file(&target_appnotify_short_so).await;
                    let _ = tokio::fs::copy(&appnotify_so, &target_appnotify_so).await;
                    let _ = tokio::fs::copy(&appnotify_so, &target_appnotify_short_so).await;
                }
            }
            let target_cfg = dir.join("MicrosoftGame.config");
            let _ = tokio::fs::remove_file(&target_cfg).await;
            let _ = tokio::fs::copy(content_dir.join("MicrosoftGame.config"), &target_cfg).await;
        }
    }

    let use_proton = wine == "wine" && proton_binary.is_some();
    let runner = if use_proton {
        proton_binary.unwrap().to_string_lossy().to_string()
    } else {
        wine
    };

    if use_proton {
        println!("Launching in-place executable with Proton CachyOS: {:?}", exec_target);
    } else {
        println!("Launching in-place executable with Wine: {:?}", exec_target);
    }
    println!("Working directory: {:?}", run_dir);

    let mut cmd = Command::new(&runner);
    cmd.current_dir(&run_dir);

    // Pass exec_target as native Linux path to Proton (Proton handles Z: mapping itself).
    // EAC Settings.json uses X:\ so EAC resolves SotGame.exe via dosdevices,
    // but SeaOfThieves.exe (the bootstrapper) can be passed directly.
    let exec_target_str = exec_target.to_string_lossy();
    let win_exec_target = format!("Z:{}", exec_target_str.replace('/', "\\"));
    println!("Launching Windows executable at: {}", win_exec_target);

    if use_proton {
        let compat_title = parsed_title_id.as_deref().unwrap_or(&title_id_str).to_ascii_lowercase();
        let compat_data = PathBuf::from(format!("{}/.local/share/xodus/compatdata/{}", home, compat_title));
        tokio::fs::create_dir_all(&compat_data).await.ok();

        let system32 = compat_data.join("pfx").join("drive_c").join("windows").join("system32");
        if system32.exists() {
            if let Some(ref gdk) = gdk_dll {
                let dst = system32.join("xgameruntime.dll");
                let _ = tokio::fs::remove_file(&dst).await;
                let _ = tokio::fs::copy(gdk, &dst).await;
                let gdk_so = gdk.with_extension("dll.so");
                if gdk_so.exists() {
                    let _ = tokio::fs::copy(&gdk_so, system32.join("xgameruntime.dll.so")).await;
                    let _ = tokio::fs::copy(&gdk_so, system32.join("xgameruntime.so")).await;
                }
            }
            if let Some(ref twinapi) = twinapi_dll {
                let dst = system32.join("twinapi.appcore.dll");
                let _ = tokio::fs::remove_file(&dst).await;
                let _ = tokio::fs::copy(twinapi, &dst).await;
                let twinapi_so = twinapi.with_extension("dll.so");
                if twinapi_so.exists() {
                    let _ = tokio::fs::copy(&twinapi_so, system32.join("twinapi.appcore.dll.so")).await;
                    let _ = tokio::fs::copy(&twinapi_so, system32.join("twinapi.appcore.so")).await;
                }
            }
            if let Some(ref textinput) = textinput_dll {
                let dst = system32.join("windows.ui.core.textinput.dll");
                let _ = tokio::fs::remove_file(&dst).await;
                let _ = tokio::fs::copy(textinput, &dst).await;
                let textinput_so = textinput.with_extension("dll.so");
                if textinput_so.exists() {
                    let _ = tokio::fs::copy(&textinput_so, system32.join("windows.ui.core.textinput.dll.so")).await;
                    let _ = tokio::fs::copy(&textinput_so, system32.join("windows.ui.core.textinput.so")).await;
                }
            }
            if let Some(ref wintypes) = wintypes_dll {
                let dst = system32.join("wintypes.dll");
                let _ = tokio::fs::remove_file(&dst).await;
                let _ = tokio::fs::copy(wintypes, &dst).await;
                let wintypes_so = wintypes.with_extension("dll.so");
                if wintypes_so.exists() {
                    let _ = tokio::fs::copy(&wintypes_so, system32.join("wintypes.dll.so")).await;
                    let _ = tokio::fs::copy(&wintypes_so, system32.join("wintypes.so")).await;
                }
            }
            if let Some(ref appnotify) = appnotify_dll {
                let dst = system32.join("api-ms-win-core-psm-appnotify-l1-1-0.dll");
                let _ = tokio::fs::remove_file(&dst).await;
                let _ = tokio::fs::copy(appnotify, &dst).await;
                let appnotify_so = appnotify.with_extension("dll.so");
                if appnotify_so.exists() {
                    let _ = tokio::fs::copy(&appnotify_so, system32.join("api-ms-win-core-psm-appnotify-l1-1-0.dll.so")).await;
                    let _ = tokio::fs::copy(&appnotify_so, system32.join("api-ms-win-core-psm-appnotify-l1-1-0.so")).await;
                }
            }
        }

        let pfx = compat_data.join("pfx");
        let system_reg = pfx.join("system.reg");
        if system_reg.exists() {
            if let Ok(mut content) = std::fs::read_to_string(&system_reg) {
                let classes = [
                    "Windows.Foundation.Metadata.ApiInformation",
                    "Windows.ApplicationModel.AppService.AppServiceConnection",
                    "Windows.ApplicationModel.Package",
                    "Windows.ApplicationModel.DataTransfer.DataTransferManager",
                    "Windows.ApplicationModel.DataTransfer.Clipboard",
                    "Windows.UI.Text.Core.CoreTextServicesManager",
                    "Windows.System.Profile.AnalyticsInfo",
                    "Windows.System.Profile.PlatformDiagnosticsAndUsageDataSettings",
                    "Windows.Graphics.Display.DisplayInformation",
                    "Windows.UI.ViewManagement.UIViewSettings",
                    "Windows.UI.ViewManagement.ApplicationView",
                    "Windows.UI.ViewManagement.InputPane",
                    "Windows.UI.ViewManagement.StatusBar",
                    "Windows.UI.ViewManagement.Core.CoreInputView",
                    "Windows.Gaming.Preview.GamesEnumeration.GameList",
                    "Windows.Gaming.XboxLive.Storage.GameSaveProvider",
                    "Windows.Internal.System.Profile.RegionPolicyEvaluator",
                    "Windows.ApplicationModel.Core.CoreApplication",
                    "Windows.UI.Core.CoreWindow",
                ];
                let mut modified = false;
                for class_id in classes {
                    let key = format!("[Software\\\\Microsoft\\\\WindowsRuntime\\\\ActivatableClassId\\\\{}]", class_id);
                    let wow_key = format!("[Software\\\\Wow6432Node\\\\Microsoft\\\\WindowsRuntime\\\\ActivatableClassId\\\\{}]", class_id);
                    for k in [&key, &wow_key] {
                        if let Some(pos) = content.find(k) {
                            let section_end = content[pos..].find("\n[").map(|p| pos + p).unwrap_or(content.len());
                            let section = &content[pos..section_end];
                            if let Some(dll_pos) = section.find("\"DllPath\"=") {
                                let abs_dll_pos = pos + dll_pos;
                                let line_end = content[abs_dll_pos..].find('\n').map(|p| abs_dll_pos + p).unwrap_or(content.len());
                                let target_line = "\"DllPath\"=\"C:\\\\windows\\\\system32\\\\xgameruntime.dll\"";
                                if &content[abs_dll_pos..line_end] != target_line {
                                    content.replace_range(abs_dll_pos..line_end, target_line);
                                    modified = true;
                                }
                            }
                        } else {
                            content.push_str(&format!(
                                "\n{} 1786972207\n#time=1dd2e49b9a3f2ea\n\"ActivationType\"=dword:00000000\n\"DllPath\"=\"C:\\\\windows\\\\system32\\\\xgameruntime.dll\"\n\"Threading\"=dword:00000000\n",
                                k
                            ));
                            modified = true;
                        }
                    }
                }
                if modified {
                    let _ = std::fs::write(&system_reg, content);
                }
            }
        }

        let eac_runtime_path = format!("{}/.local/share/Steam/steamapps/common/Proton EasyAntiCheat Runtime/v2", home);
        let has_eac_runtime = std::path::Path::new(&eac_runtime_path).exists();
        if has_eac_runtime {
            println!("EAC Runtime found at: {}", eac_runtime_path);
        }

        // Clean AppImage environment leaks that can cause Proton/Vulkan crashes
        if let Ok(ld) = std::env::var("LD_LIBRARY_PATH") {
            let cleaned_ld: Vec<&str> = ld
                .split(':')
                .filter(|p| !p.contains(".mount_") && !p.is_empty())
                .collect();
            if cleaned_ld.is_empty() {
                cmd.env_remove("LD_LIBRARY_PATH");
            } else {
                cmd.env("LD_LIBRARY_PATH", cleaned_ld.join(":"));
            }
        }
        if let Ok(py) = std::env::var("PYTHONPATH") {
            if py.contains(".mount_") {
                cmd.env_remove("PYTHONPATH");
            }
        }
        if let Ok(pyhome) = std::env::var("PYTHONHOME") {
            if pyhome.contains(".mount_") {
                cmd.env_remove("PYTHONHOME");
            }
        }

        cmd.arg("run")
           .arg(&win_exec_target)
           .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", format!("{}/.local/share/Steam", home))
           .env("STEAM_COMPAT_DATA_PATH", &compat_data)
           .env("PROTON_LOG", "1")
           .env("WINEDEBUG", "+gdkc,+xgameruntime,+winhttp,+wininet,+schannel,+secur32")
           .env("WINEDLLPATH", &wine_dll_path);

        // Wire in Proton's native Linux EAC runtime if available.
        // PROTON_EAC_RUNTIME tells Proton's internal Wine DLL where the Linux EAC shim lives.
        // NOTE: Full EAC Linux runtime requires Steam's EAC service daemon. We set the env var
        // anyway so any Proton-internal hooks can pick it up.
        if has_eac_runtime {
            cmd.env("PROTON_EAC_RUNTIME", &eac_runtime_path);
            // Add the EAC runtime lib64 to Wine's DLL search path
            let eac_lib64 = format!("{}/lib64", eac_runtime_path);
            let combined_dll_path = format!("{}:{}", eac_lib64, wine_dll_path);
            cmd.env("WINEDLLPATH", &combined_dll_path);
            // EAC uses SteamAppId for game identification on its servers
            if let Some(ref tid) = parsed_title_id {
                cmd.env("SteamAppId", tid);
                cmd.env("SteamGameId", tid);
                cmd.env("STEAM_COMPAT_APP_ID", tid);
            }
        } else {
            cmd.env("WINEDLLPATH", &wine_dll_path);
        }
        // Standard DLL overrides — don't touch EAC DLLs, let EAC use its own Windows-side logic
        cmd.env("WINEDLLOVERRIDES",
            "xgameruntime=n,b;twinapi.appcore=n,b;windows.ui.core.textinput=n,b;wintypes=n,b;api-ms-win-core-psm-appnotify-l1-1-0=n,b;\
             steamclient=;steamclient64=;steam_api=;steam_api64=;\
             GameOverlayRenderer=;GameOverlayRenderer64=");


    } else {
        cmd.arg(&win_exec_target)
           .env("WINEDLLOVERRIDES", &dll_overrides)
           .env("WINEDLLPATH", &wine_dll_path);
    }

    if let Some(pfn) = package_family_name {
        cmd.env("LOCAL_APP_MODEL_PACKAGE_FAMILY_NAME", pfn);
    }
    if let Some(tid) = parsed_title_id {
        cmd.env("XODUS_TITLE_ID", tid);
    }
    if let Some(ref sid) = parsed_store_id {
        cmd.env("XODUS_STORE_ID", sid);
    } else {
        cmd.env("XODUS_STORE_ID", "9P2N57MC619K");
    }

    // Auto-sync cloud saves: Pull latest cloud saves before launch
    println!("Auto-syncing cloud saves (pull) from Xbox Live before launch...");
    let _ = crate::commands::save::pull(client, tokens, source.clone()).await;

    let mut wn = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to spawn runner process ({}): {}", runner, e);
            return ExitCode::FAILURE;
        }
    };

    let pid = wn.id().unwrap_or(0);
    if pid > 0 {
        let _ = ctrlc::set_handler(move || {
            let _ = kill(Pid::from_raw(pid as i32), Signal::SIGINT);
        });
    }

    let status = match wn.wait().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error waiting for runner process: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // If the game executable launched a secondary child process (e.g. shipping binary or worker),
    // wait until the child process actually exits before tearing down the session.
    let secondary_exes: Vec<String> = decrypted_exes.iter().filter_map(|(rel, _)| {
        let name = Path::new(rel).file_name()?.to_string_lossy().to_string();
        if !name.eq_ignore_ascii_case(&target_exe_name) {
            Some(name)
        } else {
            None
        }
    }).collect();

    if !secondary_exes.is_empty() {
        println!("Checking for active child game processes ({:?})...", secondary_exes);
        let mut active_child: Option<String> = None;
        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            for child in &secondary_exes {
                let stem = Path::new(child).file_stem().unwrap_or_default().to_string_lossy();
                let check = tokio::process::Command::new("pgrep")
                    .arg("-i")
                    .arg("-f")
                    .arg(&*stem)
                    .output()
                    .await;
                if let Ok(out) = check {
                    if !out.stdout.is_empty() {
                        println!("Detected active child game process: {}", child);
                        active_child = Some(stem.to_string());
                        break;
                    }
                }
            }
            if active_child.is_some() {
                break;
            }
        }

        if let Some(child_stem) = active_child {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                let check = tokio::process::Command::new("pgrep")
                    .arg("-i")
                    .arg("-f")
                    .arg(&child_stem)
                    .output()
                    .await;
                if let Ok(out) = check {
                    if out.stdout.is_empty() {
                        break;
                    }
                } else {
                    break;
                }
            }
            println!("Game session finished.");
        }
    }

    // Auto-sync cloud saves: Push updated local saves to Xbox Live after session
    println!("Auto-syncing cloud saves (push) to Xbox Live after session exit...");
    let _ = crate::commands::save::push(client, tokens, source).await;

    ExitCode::from(status.code().map(|c| c as u8).unwrap_or(0))
}

