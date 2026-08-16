use std::path::{Path, PathBuf};
use std::process::ExitCode;
use xodus::tokens::TokenManager;

pub async fn run(
    client: &reqwest::Client,
    tokens: &TokenManager,
    target: String,
    skip_save_sync: bool,
    remove_compatdata: bool,
) -> ExitCode {
    println!("Uninstalling game: {target}");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

    // 1. Resolve target path
    let target_path = resolve_game_path(&target).await;
    let Some(game_dir) = target_path else {
        eprintln!("Error: Could not locate installed game directory for '{target}'");
        return ExitCode::FAILURE;
    };

    println!("Found installed game directory: {}", game_dir.display());

    // 2. Identify Title ID for cache cleanup
    let title_id = detect_title_id(&game_dir).await;

    // 3. Backup / Sync cloud saves before removal if requested
    if !skip_save_sync {
        println!("Syncing pending cloud saves to Xbox Live before uninstalling...");
        let _ = crate::commands::save::push(client, tokens, game_dir.to_string_lossy().to_string()).await;
    }

    // 4. Clean up Xodus runtime caches
    if let Some(ref tid) = title_id {
        let bin_cache = PathBuf::from(format!("{}/.cache/xodus/bin/{}", home, tid));
        if bin_cache.exists() {
            println!("Removing binary cache: {}", bin_cache.display());
            let _ = tokio::fs::remove_dir_all(&bin_cache).await;
        }

        let run_cache = PathBuf::from(format!("{}/.cache/xodus/run/{}", home, tid));
        if run_cache.exists() {
            println!("Removing runtime cache: {}", run_cache.display());
            let _ = tokio::fs::remove_dir_all(&run_cache).await;
        }

        if remove_compatdata {
            let compat_dir = PathBuf::from(format!("{}/.local/share/xodus/compatdata/{}", home, tid));
            if compat_dir.exists() {
                println!("Removing Proton compatdata prefix: {}", compat_dir.display());
                let _ = tokio::fs::remove_dir_all(&compat_dir).await;
            }
        }
    }

    // 5. Delete the main game directory
    println!("Removing game files from: {}", game_dir.display());
    match tokio::fs::remove_dir_all(&game_dir).await {
        Ok(_) => {
            println!("Successfully uninstalled game from {}", game_dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to remove directory {}: {}", game_dir.display(), e);
            ExitCode::FAILURE
        }
    }
}

async fn resolve_game_path(target: &str) -> Option<PathBuf> {
    let direct_path = PathBuf::from(target);
    if direct_path.is_dir() {
        return Some(direct_path);
    }

    // Check /mnt/w11/XboxGames/<target>
    let default_root = PathBuf::from("/mnt/w11/XboxGames");
    let candidate = default_root.join(target);
    if candidate.is_dir() {
        return Some(candidate);
    }

    // Search /mnt/w11/XboxGames/ for folder names or matching manifests
    if let Ok(mut entries) = tokio::fs::read_dir(&default_root).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                if name.eq_ignore_ascii_case(target) {
                    return Some(p);
                }
                let content = p.join("Content");
                let check_dir = if content.is_dir() { content } else { p.clone() };
                for cfg_name in ["MicrosoftGame.config", "AppxManifest.xml", "appxmanifest.xml"] {
                    if let Ok(xml) = tokio::fs::read_to_string(check_dir.join(cfg_name)).await {
                        if xml.to_lowercase().contains(&target.to_lowercase()) {
                            return Some(p);
                        }
                    }
                }
            }
        }
    }

    None
}

async fn detect_title_id(game_dir: &Path) -> Option<String> {
    let content = game_dir.join("Content");
    let check_dir = if content.is_dir() { content } else { game_dir.to_path_buf() };

    for cfg_name in ["MicrosoftGame.config", "AppxManifest.xml", "appxmanifest.xml"] {
        if let Ok(xml) = tokio::fs::read_to_string(check_dir.join(cfg_name)).await {
            if let Some(tid) = msixvc::manifest::AppxManifest::extract_title_id(&xml) {
                return Some(tid);
            }
        }
    }

    game_dir.file_name().map(|n| n.to_string_lossy().to_string())
}
