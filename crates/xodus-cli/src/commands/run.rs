use std::collections::HashMap;
use std::os::fd::{AsFd, IntoRawFd};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use msixvc::models::xvd::PAGE_SIZE;
use msixvc::xvd::{SegmentFile, XvdFile};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
#[cfg(target_os = "linux")]
use rustix::fs::{MemfdFlags, memfd_create};
use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};
#[cfg(not(target_os = "linux"))]
use tempfile::{tempdir, tempfile, tempfile_in};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use xodus::tokens::TokenManager;

use crate::license::get_license;

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

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<Executable") && trimmed.contains("Name=") {
            if let Some(start) = trimmed.find("Name=\"") {
                let rest = &trimmed[start + 6..];
                if let Some(end) = rest.find('"') {
                    let exe_name = &rest[..end];
                    if !exe_name.eq_ignore_ascii_case("gamelaunchhelper.exe") {
                        info.executable = Some(exe_name.to_string());
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
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let socket_path = format!("{}/xodus.sock", runtime_dir);

    if tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
        return;
    }

    println!("Ensuring xodus-service background daemon is active...");
    let service_binary = {
        let home = std::env::var("HOME").unwrap_or_default();
        let local_bin = PathBuf::from(format!("{}/.local/bin/xodus-service", home));
        if local_bin.exists() {
            Some(local_bin)
        } else if let Ok(path_var) = std::env::var("PATH") {
            path_var.split(':').map(PathBuf::from).map(|p| p.join("xodus-service")).find(|p| p.exists())
        } else {
            None
        }
    };


    if let Some(bin) = service_binary {
        let _ = tokio::process::Command::new(bin)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        for _ in 0..15 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
                println!("xodus-service daemon connected.");
                return;
            }
        }
    }
}

pub async fn run(
    client: &reqwest::Client,
    tokens: &TokenManager,
    source: String,
    wine: String,
    exe: Option<String>,
    market: Option<String>,
) -> ExitCode {
    ensure_service_running().await;

    let mut lfiles: HashMap<String, SegmentFile> = HashMap::new();

    let out: &Path = Path::new(&source);
    let out_absolute = match std::fs::canonicalize(out) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to resolve path {:?}: {}", out, e);
            return ExitCode::FAILURE;
        }
    };

    // Determine execution directory (check if Content/ subfolder exists)
    let content_dir = if out_absolute.join("Content").is_dir() {
        out_absolute.join("Content")
    } else {
        out_absolute.clone()
    };

    let mut container_path = None;
    if out_absolute.join(".xodus-streaming.msixvc").exists() {
        container_path = Some(out_absolute.join(".xodus-streaming.msixvc"));
    } else if let Ok(entries) = std::fs::read_dir(&out_absolute) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let name = p.file_name().unwrap().to_string_lossy();
                if name.len() == 36 && !name.contains('.') {
                    container_path = Some(p);
                    break;
                }
            }
        }
    }

    let mut exe = exe;
    let mut package_family_name = None;
    let mut parsed_title_id = None;

    // Parse MicrosoftGame.config if present
    let config_path = if content_dir.join("MicrosoftGame.config").exists() {
        Some(content_dir.join("MicrosoftGame.config"))
    } else if out_absolute.join("MicrosoftGame.config").exists() {
        Some(out_absolute.join("MicrosoftGame.config"))
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
            if let Ok(manifest) = msixvc::manifest::AppxManifest::parse(&content) {
                let pfn = manifest.package_family_name();
                println!(
                    "Detected Package Identity: {} (v{}) [Family: {}]",
                    manifest.identity.name, manifest.identity.version, pfn
                );
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

    let target_exe_name = exe.unwrap_or_else(|| "Brotato.exe".to_string());
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
        "Windows.ApplicationModel.AppService.AppServiceConnection",
        "Windows.ApplicationModel.Package",
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

    let wine_dll_path = match std::env::var("WINEDLLPATH") {
        Ok(paths) => format!("{}:{}:{}", local_xodus_lib, repo_xodus_lib, paths),
        Err(_) => format!("{}:{}", local_xodus_lib, repo_xodus_lib),
    };

    if is_plaintext || container_path.is_none() {
        println!("Launching in-place executable directly with Wine: {:?}", target_exe_path);
        let mut cmd = Command::new(&wine);
        cmd.current_dir(&content_dir)
           .arg(&target_exe_path)
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

    let final_path = container_path.unwrap();
    println!("Found XVD/MSIXVC container: {:?}", final_path);

    let mut file = OpenOptions::new()
        .read(true)
        .open(final_path.to_owned())
        .await
        .unwrap();

    let xvd = XvdFile::parse(&mut file).await.expect("no err");

    let files = xvd.parse_user_package_files(&mut file).await.expect("ok");

    for (k, v) in &files {
        if k == "SegmentMetadata.bin" {
            let sfiles = xvd.parse_segment_metadata(&mut file, v).await.expect("ok");
            lfiles = sfiles;
        }
    }

    if lfiles.is_empty() {
        let sfiles = xvd
            .parse_ntfs_segment_metadata(&mut file, !lfiles.is_empty())
            .await
            .expect("ok");
        lfiles.extend(sfiles);
    }

    let license = get_license(
        client,
        tokens,
        xvd.content_id().to_string(),
        market.unwrap_or("neutral".to_string()),
    )
    .await;
    if let Err(err) = license {
        eprintln!("{}", err);
        return ExitCode::FAILURE;
    }
    let (key, game_splicense) = license.unwrap();
    if game_splicense.content_keys.len() != 1 {
        eprintln!(
            "unexpected number of content keys {}",
            game_splicense.content_keys.len()
        );
        return ExitCode::FAILURE;
    }
    let Some((_, content_key)) = game_splicense.content_keys.into_iter().next() else {
        return ExitCode::FAILURE;
    };

    let full_key = content_key.unpack(&key).expect("failed to unpack");

    let title_id_str = xvd.content_id().to_string();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let cache_dir = PathBuf::from(format!("{}/.cache/xodus/bin/{}", home, title_id_str));
    tokio::fs::create_dir_all(&cache_dir).await.ok();

    let cached_exe = cache_dir.join(&target_exe_name);

    let sfile_opt = lfiles.iter().find(|(k, _)| {
        let norm_k = k.replace('\\', "/").to_ascii_lowercase();
        let norm_target = target_exe_name.to_ascii_lowercase();
        norm_k == norm_target || norm_k.ends_with(&format!("/{}", norm_target)) || norm_k.ends_with(&norm_target)
    }).map(|(_, v)| v);

    if let Some(sfile) = sfile_opt {
        if !cached_exe.exists() {
            println!("Found segment for {}: offset={}, length={}", target_exe_name, sfile.offset, sfile.length);
            let mut out_f = File::create(&cached_exe).await.unwrap();
            if target_exe_path.exists() {
                let mut src_f = File::open(&target_exe_path).await.unwrap();
                if let Err(err) = xvd.mount_mem_fd(&mut src_f, &mut out_f, sfile, *full_key, |_, _| {}).await {
                    eprintln!("Failed to decrypt binary: {}", err);
                }
            } else {
                let mut src_f = File::open(&final_path).await.unwrap();
                if let Err(err) = xvd.extract_file(&mut src_f, &mut out_f, sfile, *full_key, |_, _| {}).await {
                    eprintln!("Failed to decrypt binary: {}", err);
                }
            }
            use tokio::io::AsyncWriteExt;
            out_f.flush().await.ok();
        }

        // Disable console-specific CPlatformUI overlay draw routines and clean shutdown
        if let Ok(mut data) = std::fs::read(&cached_exe) {
            let mut modified = false;
            for rva in [0x21911e0, 0x2196a50, 0x2197150, 0x21973e0, 0x2197580, 0x21978d0, 0x2197a00] {
                if rva > 0x1000 && rva - 0x1000 + 0x600 < data.len() {
                    let off = 0x600 + (rva - 0x1000);
                    if data[off] != 0xc3 {
                        data[off] = 0xc3;
                        modified = true;
                    }
                }
            }

            // Bypass GDK suspend timeout shutdown (RVA 0x2194ad3: 0f 84 b3 00 00 00 -> e9 b4 00 00 00 90)
            let suspend_off = 0x600 + (0x2194ad3 - 0x1000);
            if suspend_off + 6 <= data.len() && &data[suspend_off..suspend_off+6] == &[0x0f, 0x84, 0xb3, 0x00, 0x00, 0x00] {
                data[suspend_off..suspend_off+6].copy_from_slice(&[0xe9, 0xb4, 0x00, 0x00, 0x00, 0x90]);
                modified = true;
            }
            if modified {
                let _ = std::fs::write(&cached_exe, &data);
            }
        }

    } else {
        println!("Executable {} not found in segment metadata (total {} files)", target_exe_name, lfiles.len());
        for (k, _) in lfiles.iter().take(10) {
            println!("  Segment file: {}", k);
        }
    }




    let run_dir = PathBuf::from(format!("{}/.cache/xodus/run/{}", home, title_id_str));
    tokio::fs::create_dir_all(&run_dir).await.ok();

    // Zero-copy symlink all assets and configs from Content into run_dir
    if let Ok(entries) = std::fs::read_dir(&content_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let file_name = p.file_name().unwrap();
            let dest = run_dir.join(file_name);
            if file_name.to_string_lossy() == target_exe_name {
                continue;
            }
            if !dest.exists() {
                #[cfg(unix)]
                let _ = std::os::unix::fs::symlink(&p, &dest);
            }
        }
    }

    let game_binary_in_run = run_dir.join(&target_exe_name);
    if cached_exe.exists() {
        let _ = tokio::fs::remove_file(&game_binary_in_run).await;
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&cached_exe, &game_binary_in_run);
    }

    let exec_target = if game_binary_in_run.exists() {
        game_binary_in_run
    } else if cached_exe.exists() {
        cached_exe
    } else {
        target_exe_path
    };

    let proton_binary = {
        let candidates = [
            PathBuf::from("/usr/share/steam/compatibilitytools.d/proton-cachyos-native/proton"),
            PathBuf::from(format!("{}/.local/share/Steam/compatibilitytools.d/proton-cachyos-native/proton", home)),
            PathBuf::from("/usr/share/steam/compatibilitytools.d/GE-Proton11-3/proton"),
        ];
        candidates.into_iter().find(|p| p.exists())
    };

    // Copy GDK and AppCore helper libraries into run_dir
    let gdk_so = PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/xgameruntime.dll.so");
    let twinapi_so = PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/twinapi.appcore.dll.so");
    if gdk_so.exists() {
        let _ = tokio::fs::copy(&gdk_so, run_dir.join("xgameruntime.dll")).await;
    }
    if twinapi_so.exists() {
        let _ = tokio::fs::copy(&twinapi_so, run_dir.join("twinapi.appcore.dll")).await;
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

    if use_proton {
        let compat_title = parsed_title_id.as_deref().unwrap_or(&title_id_str);
        let compat_data = PathBuf::from(format!("{}/.local/share/xodus/compatdata/{}", home, compat_title));
        tokio::fs::create_dir_all(&compat_data).await.ok();

        cmd.arg("run")
           .arg(&exec_target)
           .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", format!("{}/.local/share/Steam", home))
           .env("STEAM_COMPAT_DATA_PATH", &compat_data)
           .env("SteamGameId", compat_title)
           .env("SteamAppId", compat_title)
           .env("PROTON_LOG", "1")
           .env("WINEDLLOVERRIDES", "xgameruntime=n,b;twinapi.appcore=n,b")
           .env("WINEDLLPATH", &wine_dll_path);

    } else {
        cmd.arg(&exec_target)
           .env("WINEDLLOVERRIDES", &dll_overrides)
           .env("WINEDLLPATH", &wine_dll_path);
    }

    if let Some(pfn) = package_family_name {
        cmd.env("LOCAL_APP_MODEL_PACKAGE_FAMILY_NAME", pfn);
    }
    if let Some(tid) = parsed_title_id {
        cmd.env("XODUS_TITLE_ID", tid);
    }

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

    ExitCode::from(status.code().map(|c| c as u8).unwrap_or(0))
}

