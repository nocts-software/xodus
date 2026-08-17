use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::vec;

use fs2::available_space;
use futures_util::{StreamExt, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use msixvc::streaming;
use msixvc::xvd::{SegmentFile, XvdFile};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};


use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use xodus::tokens::TokenManager;

use crate::license::get_license;
use crate::package::{get_content_id, get_packages};

struct Job {
    name: String,
    content: SegmentFile,
}

enum ProgressEvent {
    Started { id: usize, name: String, total: u64 },
    Advanced { id: usize, delta: u64 },
    Finished { id: usize },
    UpdateRemaining { name: String, total: u64 },
    UpdateStatus { name: String },
}

pub struct CompositeVolumeReader {
    files: Vec<(u64, u64, std::fs::File)>,
    length: u64,
    pos: u64,
}

impl CompositeVolumeReader {
    pub async fn new(primary_path: &Path) -> std::io::Result<Self> {
        let mut files = Vec::new();

        let p_file = std::fs::File::open(primary_path)?;
        let p_len = p_file.metadata()?.len();
        files.push((0u64, p_len, p_file));

        let parent = primary_path.parent().unwrap_or(primary_path);
        let content_dir = parent.join("Content");
        let mut current_offset = 7_073_792u64;

        if content_dir.exists() {
            let candidates = ["data.hvp", "boot.hvp"];
            for candidate in candidates {
                let candidate_path = content_dir.join(candidate);
                if candidate_path.exists() {
                    if let Ok(f) = std::fs::File::open(&candidate_path) {
                        if let Ok(meta) = f.metadata() {
                            let len = meta.len();
                            files.push((current_offset, current_offset + len, f));
                            current_offset += len;
                        }
                    }
                }
            }
        }

        Ok(Self {
            files,
            length: current_offset,
            pos: 0,
        })
    }

    pub fn len(&self) -> u64 {
        self.length
    }
}

impl tokio::io::AsyncSeek for CompositeVolumeReader {
    fn start_seek(mut self: std::pin::Pin<&mut Self>, pos: std::io::SeekFrom) -> std::io::Result<()> {
        let new_pos = match pos {
            std::io::SeekFrom::Start(off) => off as i64,
            std::io::SeekFrom::Current(off) => self.pos as i64 + off,
            std::io::SeekFrom::End(off) => self.length as i64 + off,
        };
        if new_pos < 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "negative seek"));
        }
        self.pos = new_pos as u64;
        Ok(())
    }

    fn poll_complete(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<u64>> {
        std::task::Poll::Ready(Ok(self.pos))
    }
}

impl AsyncRead for CompositeVolumeReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::io::{Read, Seek};
        let pos = self.pos;
        let len = self.length;
        if pos >= len {
            return std::task::Poll::Ready(Ok(()));
        }

        let mut matched = None;
        for (start, end, f) in self.files.iter_mut() {
            if pos >= *start && pos < *end {
                matched = Some((*start, f));
                break;
            }
        }

        if let Some((start_offset, file)) = matched {
            let inner_pos = pos - start_offset;
            if let Err(e) = file.seek(std::io::SeekFrom::Start(inner_pos)) {
                return std::task::Poll::Ready(Err(e));
            }
            let unfilled = buf.initialize_unfilled();
            match file.read(unfilled) {
                Ok(n) => {
                    buf.advance(n);
                    self.pos += n as u64;
                    std::task::Poll::Ready(Ok(()))
                }
                Err(e) => std::task::Poll::Ready(Err(e)),
            }
        } else {
            let next_start = self.files.iter().map(|(s, _, _)| *s).find(|s| *s > pos).unwrap_or(len);
            let to_fill = std::cmp::min((next_start - pos) as usize, buf.remaining());
            let unfilled = buf.initialize_unfilled();
            unfilled[..to_fill].fill(0);
            buf.advance(to_fill);
            self.pos += to_fill as u64;
            std::task::Poll::Ready(Ok(()))
        }
    }
}



pub async fn run(
    client: &reqwest::Client,
    tokens: &TokenManager,
    source: String,
    destination: String,
    try_skip_ntfs: bool,
    parallel: Option<usize>,
    market: Option<String>,
) -> ExitCode {
    let (tx, rx) = tokio::sync::mpsc::channel::<ProgressEvent>(256);
    if source.starts_with("file://") {
        let fsrc = source.strip_prefix("file://").unwrap_or_default();
        let composite = match CompositeVolumeReader::new(Path::new(fsrc)).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to open local package source: {e}");
                return ExitCode::FAILURE;
            }
        };
        let l = composite.len();
        match run_cli_reader(
            client,
            tokens,
            destination,
            try_skip_ntfs,
            parallel,
            market,
            composite,
            l,
            &source,
            &tx,
            rx,
        )
        .await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Operation Failed: {err}");
                ExitCode::FAILURE
            }
        }
    } else {

        let vurl = if source.starts_with("http://") || source.starts_with("https://") {
            source
        } else {
            let content_id = if Uuid::try_parse(&source).is_err() {
                let content_id_task = get_content_id(client, source, market.clone()).await;
                let Ok(content_id) = content_id_task else {
                    let Err(err) = content_id_task else {
                        eprintln!("Unknown Error");
                        return ExitCode::FAILURE;
                    };
                    eprintln!("{}", err);
                    return ExitCode::FAILURE;
                };
                content_id
            } else {
                source
            };
            let package_result = get_packages(client, tokens, content_id.clone()).await;
            let Ok(package) = package_result else {
                let Err(err) = package_result else {
                    eprintln!("Unknown Error");
                    return ExitCode::FAILURE;
                };
                eprintln!("{}", err);
                return ExitCode::FAILURE;
            };
            let Some(file) = package
                .package_files
                .iter()
                .find(|p| {
                    let lower = p.file_name.to_lowercase();
                    lower.ends_with(".msixvc") || lower.ends_with(".xvc") || lower.ends_with(".xvd")
                })
                .or_else(|| package.package_files.first())
            else {
                eprintln!("No package file found in GetBasePackage response");
                return ExitCode::FAILURE;
            };
            let cdn_root = file.cdn_root_paths.first().map(|s| s.as_str()).unwrap_or("");
            format!(
                "{}{}",
                cdn_root,
                file.relative_url
            )
        };
        let url = &vurl;
        let mut pos = 0;
        let http_file = streaming::HttpRead::open(
            client.clone(),
            url,
            Some(|c, _| {
                if tx
                    .try_send(ProgressEvent::Advanced {
                        id: usize::MAX,
                        delta: c - pos,
                    })
                    .is_ok()
                {
                    pos = c;
                }
            }),
        )
        .await
        .expect("ok");
        let l = http_file.len();

        match run_cli_reader(
            client,
            tokens,
            destination,
            try_skip_ntfs,
            parallel,
            market,
            http_file,
            l,
            url,
            &tx,
            rx,
        )
        .await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Operation Failed: {err}");
                ExitCode::FAILURE
            }
        }
    }
}

async fn run_cli_reader<Reader>(
    client: &reqwest::Client,
    tokens: &TokenManager,
    destination: String,
    try_skip_ntfs: bool,
    parallel: Option<usize>,
    market: Option<String>,
    reader: Reader,
    l: u64,
    url: &str,
    tx: &Sender<ProgressEvent>,
    mut rx: Receiver<ProgressEvent>,
) -> Result<(), String>
where
    Reader: AsyncRead + Unpin,
{
    tokio::spawn(async move {
        let multi_progress = MultiProgress::new();
        let total_progess = multi_progress.add(ProgressBar::new(l).with_style(
            ProgressStyle::with_template("{msg:30!} {bytes:>12}/{total_bytes:>12} {bytes_per_sec:>12} [{bar:40.cyan/blue}] {percent:>3}%").unwrap()
            .progress_chars("#>-")
        ));

        total_progess.set_message("Initializing");
        let mut bars: HashMap<usize, ProgressBar> = HashMap::new();

        let mut last_emit = std::time::Instant::now();
        let mut last_bytes = 0u64;
        let mut smoothed_speed = 0.0f64;
        let mut current_stage = "Initializing".to_string();

        while let Some(event) = rx.recv().await {
            match event {
                ProgressEvent::Started { id, name, total } => {
                    let cur_progess = multi_progress.add(ProgressBar::new(total).with_style(
                        ProgressStyle::with_template("{msg:30!} {bytes:>12}/{total_bytes:>12} {bytes_per_sec:>12} [{bar:40.cyan/blue}] {percent:>3}%").unwrap()
                        .progress_chars("#>-")
                    ));
                    cur_progess.set_message(name);
                    bars.insert(id, cur_progess);
                }
                ProgressEvent::Advanced { id, delta } => {
                    if let Some(bar) = bars.get(&id) {
                        bar.inc(delta);
                    }
                    total_progess.inc(delta);
                }
                ProgressEvent::Finished { id } => {
                    if let Some(bar) = bars.remove(&id) {
                        bar.finish_and_clear();
                    }
                }
                ProgressEvent::UpdateRemaining { name, total } => {
                    current_stage = name.clone();
                    total_progess.set_message(name);
                    total_progess.set_length(total_progess.position() + total);
                }
                ProgressEvent::UpdateStatus { name } => {
                    current_stage = name.clone();
                    total_progess.set_message(name);
                }
            }

            let cur_pos = total_progess.position();
            let total_len = total_progess.length().unwrap_or(l);
            let elapsed = last_emit.elapsed();

            if elapsed.as_millis() >= 200 || (total_len > 0 && cur_pos >= total_len) {
                let delta_b = if cur_pos >= last_bytes { cur_pos - last_bytes } else { 0 };
                let dt_secs = elapsed.as_secs_f64();
                if dt_secs > 0.0 {
                    let inst_speed = delta_b as f64 / dt_secs;
                    if smoothed_speed == 0.0 {
                        smoothed_speed = inst_speed;
                    } else {
                        smoothed_speed = 0.75 * smoothed_speed + 0.25 * inst_speed;
                    }
                }
                last_bytes = cur_pos;
                last_emit = std::time::Instant::now();

                let eta_secs = if smoothed_speed > 1024.0 && total_len > cur_pos {
                    ((total_len - cur_pos) as f64 / smoothed_speed) as u64
                } else {
                    0
                };
                let pct = if total_len > 0 {
                    (cur_pos as f64 / total_len as f64 * 100.0).min(100.0)
                } else {
                    0.0
                };

                println!(
                    "PROGRESS:{{\"bytes\":{},\"total\":{},\"speed\":{},\"eta\":{},\"percent\":{:.2},\"stage\":\"{}\"}}",
                    cur_pos,
                    total_len,
                    smoothed_speed as u64,
                    eta_secs,
                    pct,
                    current_stage
                );
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
        }

        total_progess.abandon();
    });
    run_reader(
        client,
        tokens,
        destination,
        try_skip_ntfs,
        parallel,
        market,
        reader,
        l,
        url,
        tx,
    )
    .await
}

async fn run_reader<Reader>(
    client: &reqwest::Client,
    tokens: &TokenManager,
    destination: String,
    try_skip_ntfs: bool,
    parallel: Option<usize>,
    market: Option<String>,
    reader: Reader,
    l: u64,
    url: &str,
    tx: &Sender<ProgressEvent>,
) -> Result<(), String>
where
    Reader: AsyncRead + Unpin,
{
    let out: &Path = Path::new(&destination);

    std::fs::create_dir_all(out).expect("ok");

    let cache_path = out.join(".xodus-streaming-tmp.msixvc");
    let final_path = out.join(".xodus-streaming.msixvc");

    let mut remote_file = streaming::PrefixCacheFile::new(reader, l, cache_path.clone())
        .await
        .expect("no err");
    let remote_xvd = XvdFile::parse(&mut remote_file).await.expect("no err");
    println!("Encrypted section infos: {:?}", remote_xvd.encrypted_section_infos);
    let mut rfiles: HashMap<String, SegmentFile> = HashMap::new();

    let mut lfiles: HashMap<String, SegmentFile> = HashMap::new();

    let files = remote_xvd
        .parse_user_package_files(&mut remote_file)
        .await
        .expect("ok");
    for (k, v) in &files {
        if k == "SegmentMetadata.bin" {
            let sfiles = remote_xvd
                .parse_segment_metadata(&mut remote_file, v)
                .await
                .expect("ok");
            rfiles = sfiles;
        }
    }

    if !try_skip_ntfs || rfiles.is_empty() {
        tx.send(ProgressEvent::UpdateStatus {
            name: "Downloading ntfs...".to_owned(),
        })
        .await
        .ok();
        let sfiles = remote_xvd
            .parse_ntfs_segment_metadata(&mut remote_file, !rfiles.is_empty())
            .await
            .expect("ok");
        rfiles.extend(sfiles);
    }

    let file = OpenOptions::new()
        .read(true)
        .open(final_path.to_owned())
        .await
        .ok();

    if let Some(mut file) = file {
        let xvd = XvdFile::parse(&mut file).await.expect("no err");

        let files = xvd.parse_user_package_files(&mut file).await.expect("ok");
        for (k, v) in &files {
            if k == "SegmentMetadata.bin" {
                let sfiles = xvd.parse_segment_metadata(&mut file, v).await.expect("ok");
                lfiles = sfiles;
            }
        }

        if let Ok(sfiles) = xvd
            .parse_ntfs_segment_metadata(&mut file, !lfiles.is_empty())
            .await
        {
            lfiles.extend(sfiles);
        }
    }

    let license = get_license(
        client,
        tokens,
        remote_xvd.content_id().to_string(),
        market.unwrap_or("neutral".to_string()),
    )
    .await;
    if let Err(err) = license {
        return Err(format!("Access Denied: {}", err));
    }
    let (key, game_splicense) = license.unwrap();
    if game_splicense.content_keys.len() != 1 {
        return Err(format!("Unexpected number of content keys: {}", game_splicense.content_keys.len()));
    }
    let Some((_, content_key)) = game_splicense.content_keys.into_iter().next() else {
        return Err("No content keys found in SPLicense".to_string());
    };

    let full_key = content_key.unpack(&key).expect("failed to unpack");

    // Collect list of files to download vs already downloaded files
    let mut files_to_download = Vec::new();
    let mut already_downloaded_bytes = 0u64;

    for (name, v) in &rfiles {
        let target_file = out.join(name.replace("\\", "/"));
        if target_file.exists() {
            if let Ok(meta) = std::fs::metadata(&target_file) {
                if meta.len() == v.length && v.length > 0 {
                    already_downloaded_bytes += v.length;
                    continue; // File is already complete and verified on disk!
                }
            }
        }
        files_to_download.push((name.clone(), v.clone()));
    }

    let remaining_download_size: u64 = files_to_download.iter().map(|(_, v)| v.length).sum();
    let total_package_size: u64 = rfiles.values().map(|v| v.length).sum();

    let required_free_space = remaining_download_size;
    let available_free_space = match available_space(out) {
        Ok(space) => space,
        Err(err) => {
            return Err(format!(
                "Failed to determine available space for {}: {}",
                out.display(),
                err
            ));
        }
    };

    if available_free_space < required_free_space {
        return Err(format!(
            "Not enough free disk space on {}: need {} bytes, have {} bytes (remaining: {})",
            out.display(),
            required_free_space,
            available_free_space,
            remaining_download_size
        ));
    }

    tx.send(ProgressEvent::UpdateRemaining {
        name: if already_downloaded_bytes > 0 { "Resuming download".to_owned() } else { "Downloading".to_owned() },
        total: total_package_size,
    })
    .await
    .ok();

    if already_downloaded_bytes > 0 {
        tx.send(ProgressEvent::Advanced {
            id: usize::MAX,
            delta: already_downloaded_bytes,
        })
        .await
        .ok();
    }

    if files_to_download.is_empty() {
        println!("All files already completely downloaded and verified.");
        std::fs::remove_file(&final_path).ok();
        std::fs::rename(&cache_path, &final_path).ok();
        return Ok(());
    }

    let remote_xvd_ref = &remote_xvd;
    stream::iter(
        files_to_download
            .iter()
            .map(|(n, v)| Job {
                name: n.clone(),
                content: SegmentFile {
                    offset: v.offset,
                    length: v.length,
                    data_hashs: vec![],
                    keep_encrypted: v.keep_encrypted,
                },
            })
            .enumerate(),
    )
    .for_each_concurrent(parallel.unwrap_or(4), |(id, job)| {
        let tx = tx.clone();
        let client = client.clone();
        async move {
            let target_file = out.join(job.name.replace("\\", "/"));
            if let Some(folder) = target_file.parent() {
                std::fs::create_dir_all(folder).expect("ok");
            }
            let mut fout = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&target_file)
                .await
                .expect("ok");
            let mut lp = 0;

            let progress = |pos, _| {
                if tx
                    .try_send(ProgressEvent::Advanced {
                        id,
                        delta: pos - lp,
                    })
                    .is_ok()
                {
                    lp = pos;
                }
            };
            let path = job.name.to_owned();
            let shown = if path.len() > 30 {
                format!("...{}", &path[path.len() - 27..])
            } else {
                path.clone()
            };
            tx.send(ProgressEvent::Started {
                id,
                name: shown,
                total: job.content.length,
            })
            .await
            .ok();

            if let Some(fpath) = url.strip_prefix("file://") {
                let mut composite = CompositeVolumeReader::new(Path::new(&fpath)).await.expect("ok");
                if let Err(err) = remote_xvd_ref
                    .extract_file(&mut composite, &mut fout, &job.content, *full_key, progress)
                    .await
                {
                    eprintln!("Failed to extract file {}: {}", job.name, err);
                }
                let _ = fout.flush().await;
                tx.send(ProgressEvent::Finished { id }).await.ok();
            } else {
                if let Err(err) = remote_xvd_ref
                    .download_file_http(&client, url, &mut fout, &job.content, *full_key, progress)
                    .await
                {
                    eprintln!("Failed to download file {}: {}", job.name, err);
                }
                let _ = fout.flush().await;
                tx.send(ProgressEvent::Finished { id }).await.ok();
            }
        }
    })
    .await;

    std::fs::remove_file(&final_path).ok();
    std::fs::rename(&cache_path, &final_path).map_err(|e| format!("Failed to move cached container: {e}"))?;
    Ok(())
}
