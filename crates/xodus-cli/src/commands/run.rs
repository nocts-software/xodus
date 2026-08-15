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

    let target_exe_name = exe.unwrap_or_else(|| "Game.exe".to_string());
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
    let rdir = std::env::var("XODUS_RUNTIME_PATH").unwrap_or_default();

    let wine_dll_path = match std::env::var("WINEDLLPATH") {
        Ok(paths) => format!("{}:{}:{}:{}", rdir, local_xodus_lib, repo_xodus_lib, paths),
        Err(_) => format!("{}:{}:{}", rdir, local_xodus_lib, repo_xodus_lib),
    };

    if is_plaintext || container_path.is_none() {
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

    println!("Encrypted section infos count: {}", xvd.encrypted_section_infos.len());
    for (i, s) in xvd.encrypted_section_infos.iter().enumerate() {
        println!("  Section #{}: offset={}, length={}, data_units={:?}", i, s.section_offset, s.section_length, s.data_units.as_ref().map(|u| u.len()));
    }

    // Decrypt all .exe segments present in the package into cache_dir (preserving relative path structure)
    let mut decrypted_exes: Vec<(String, PathBuf)> = Vec::new();
    for (k, sfile) in &lfiles {
        let norm_k = k.replace('\\', "/");
        if norm_k.to_ascii_lowercase().ends_with(".exe") {
            let rel_path = norm_k.trim_start_matches('/').to_string();
            let dest_exe = cache_dir.join(&rel_path);
            if let Some(parent) = dest_exe.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            let should_extract = !dest_exe.exists() || std::fs::metadata(&dest_exe).map(|m| m.len() == 0).unwrap_or(true);
            if should_extract {
                println!("Decrypting executable segment {}: length={}", rel_path, sfile.length);
                if let Ok(mut out_f) = File::create(&dest_exe).await {
                    let full_src = content_dir.join(&rel_path);
                    if full_src.exists() {
                        if let Ok(mut src_f) = File::open(&full_src).await {
                            let _ = xvd.mount_mem_fd(&mut src_f, &mut out_f, sfile, *full_key, |_, _| {}).await;
                        }
                    } else {
                        if let Ok(mut src_f) = File::open(&final_path).await {
                            let _ = xvd.extract_file(&mut src_f, &mut out_f, sfile, *full_key, |_, _| {}).await;
                        }
                    }
                    use tokio::io::AsyncWriteExt;
                    out_f.flush().await.ok();
                }
            }
            decrypted_exes.push((rel_path, dest_exe));
        }
    }

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
    let _game_drive_letter = 'X';

    // EAC Settings.json: always keep original relative path (Athena\Binaries\WinGDK\SotGame.exe).
    // EAC bootstrapper resolves this path relative to its working directory (run_dir).
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
                if file_name.to_string_lossy().to_lowercase() == "settings.json" {
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        let modified_content = content.replace("\"WaitForGameProcessExit\": false", "\"WaitForGameProcessExit\": true");
                        let _ = std::fs::write(&dest, modified_content);
                    } else {
                        let _ = std::fs::copy(&p, &dest);
                    }
                } else if !dest.exists() {
                    #[cfg(unix)]
                    let _ = std::os::unix::fs::symlink(&p, &dest);
                }
            }
        }
        println!("EAC Settings.json: using original relative executable path");
    }

    let game_binary_in_run = run_dir.join(&target_exe_name);
    if cached_exe.exists() {
        let _ = tokio::fs::remove_file(&game_binary_in_run).await;
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&cached_exe, &game_binary_in_run);
    }

    // Deploy GDK runtime DLLs directly into run_dir, executable directories, and prefix system32
    let runtime_dir = format!("{}/.local/share/xodus/runtime", home);
    let dll_names = ["xgameruntime.dll", "twinapi.appcore.dll", "api-ms-win-core-psm-appnotify-l1-1-0.dll", "xgameruntime.dll.so", "twinapi.appcore.dll.so"];
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
    };

    // Locate bundled GDK and AppCore helper libraries from AppImage or system
    let runtime_dir = std::env::var("XODUS_RUNTIME_PATH")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../lib")))
        })
        .or_else(|| Some(PathBuf::from("/usr/lib/xodus")));

    let gdk_dll = runtime_dir
        .as_ref()
        .map(|d| d.join("xgameruntime.dll"))
        .filter(|p| p.exists())
        .or_else(|| Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/xgameruntime.dll")).filter(|p| p.exists()));

    let twinapi_dll = runtime_dir
        .as_ref()
        .map(|d| d.join("twinapi.appcore.dll"))
        .filter(|p| p.exists())
        .or_else(|| Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/twinapi.appcore.dll")).filter(|p| p.exists()));

    let appnotify_dll = runtime_dir
        .as_ref()
        .map(|d| d.join("api-ms-win-core-psm-appnotify-l1-1-0.dll"))
        .filter(|p| p.exists())
        .or_else(|| Some(PathBuf::from("/run/media/noct/ssd1/Repo/other/xodus/xgameruntime/api-ms-win-core-psm-appnotify-l1-1-0.dll")).filter(|p| p.exists()));

    let target_sub_dirs = [
        run_dir.clone(),
        cached_exe.parent().unwrap_or(&run_dir).to_path_buf(),
        run_dir.join("Athena").join("Binaries").join("WinGDK"),
    ];

    for dir in &target_sub_dirs {
        if dir.exists() {
            if let Some(ref gdk) = gdk_dll {
                let _ = tokio::fs::copy(gdk, dir.join("xgameruntime.dll")).await;
                let gdk_so = gdk.with_extension("dll.so");
                if gdk_so.exists() {
                    let _ = tokio::fs::copy(&gdk_so, dir.join("xgameruntime.dll.so")).await;
                    let _ = tokio::fs::copy(&gdk_so, dir.join("xgameruntime.so")).await;
                }
            }
            if let Some(ref twinapi) = twinapi_dll {
                let _ = tokio::fs::copy(twinapi, dir.join("twinapi.appcore.dll")).await;
                let twinapi_so = twinapi.with_extension("dll.so");
                if twinapi_so.exists() {
                    let _ = tokio::fs::copy(&twinapi_so, dir.join("twinapi.appcore.dll.so")).await;
                    let _ = tokio::fs::copy(&twinapi_so, dir.join("twinapi.appcore.so")).await;
                }
            }
            if let Some(ref appnotify) = appnotify_dll {
                let _ = tokio::fs::copy(appnotify, dir.join("api-ms-win-core-psm-appnotify-l1-1-0.dll")).await;
            }
            let _ = tokio::fs::copy(content_dir.join("MicrosoftGame.config"), dir.join("MicrosoftGame.config")).await;
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
        let compat_title = parsed_title_id.as_deref().unwrap_or(&title_id_str);
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
            if let Some(ref appnotify) = appnotify_dll {
                let dst = system32.join("api-ms-win-core-psm-appnotify-l1-1-0.dll");
                let _ = tokio::fs::remove_file(&dst).await;
                let _ = tokio::fs::copy(appnotify, dst).await;
            }
        }

        let eac_runtime_path = format!("{}/.local/share/Steam/steamapps/common/Proton EasyAntiCheat Runtime/v2", home);
        let has_eac_runtime = std::path::Path::new(&eac_runtime_path).exists();
        if has_eac_runtime {
            println!("EAC Runtime found at: {}", eac_runtime_path);
        }

        cmd.arg("run")
           .arg(&win_exec_target)
           .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", format!("{}/.local/share/Steam", home))
           .env("STEAM_COMPAT_DATA_PATH", &compat_data)
           .env("PROTON_LOG", "1")
           .env("WINEDEBUG", "+gdkc,+xgameruntime")
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
            "xgameruntime=n,b;twinapi.appcore=n,b;api-ms-win-core-psm-appnotify-l1-1-0=n,b;\
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

