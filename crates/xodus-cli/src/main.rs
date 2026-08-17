use std::process::ExitCode;

use clap::{Parser, Subcommand};
use xodus::tokens::TokenManager;

mod commands;
mod license;
mod package;
mod webview;

#[derive(Subcommand, Debug)]
enum SubCommand {
    #[command(
        alias = "get",
        alias = "install",
        about = "Download msixvc or xsp files for a given game"
    )]
    Download {
        #[clap(help = "Product ID / BigId or game title (e.g. '9P2N57MC619K', 'Sea of Thieves')")]
        product: String,
        #[arg(short, long, help = "Store marketplace region code (e.g. 'us', 'neutral')")]
        market: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Display download URLs instead of downloading"
        )]
        dry_run: bool,
    },
    #[command(about = "Dump CIKs for use with XvdTool")]
    License {
        #[clap(help = "Content Id of a license")]
        content_id: String,
        #[clap(help = "A path where to dump CIKs")]
        ciks: String,
        #[arg(short, long)]
        market: Option<String>,
    },
    #[command(about = "Extract locally stored msixvc file")]
    Extract {
        path: String,
        destination: String,
        #[arg(short, long)]
        market: Option<String>,
    },
    #[command(
        alias = "signin",
        alias = "auth",
        about = "Sign into Microsoft account via webview authentication window"
    )]
    Login,
    #[command(
        alias = "signout",
        about = "Sign out and clear local credentials"
    )]
    Logout {
        #[arg(long, default_value_t = false, help = "Remove device license")]
        device: bool,
    },
    #[command(about = "Display current Microsoft account, Xbox Live profile, and entitlement status")]
    Status,

    #[command(about = "Download and extract the game through streaming algorithm")]
    Streaming {
        source: String,
        destination: String,
        #[arg(
            long,
            default_value_t = false,
            help = "Attempt to skip downloading NTFS metadata to be faster while missing some files"
        )]
        try_skip_ntfs: bool,
        #[arg(short, long)]
        parallel: Option<usize>,
        #[arg(short, long)]
        market: Option<String>,
    },
    #[cfg(unix)]
    #[command(
        alias = "play",
        alias = "launch",
        alias = "start",
        about = "Play or run an installed game with xodus wine / Proton"
    )]
    Run {
        #[clap(help = "Game directory path, game title, or product ID (e.g. 'Sea of Thieves', '9P2N57MC619K')")]
        source: String,
        #[arg(short, long, default_value = "wine", help = "Wine / Proton binary to use (default: wine)")]
        wine: String,
        #[arg(short, long, help = "Specific executable name to launch within game directory")]
        exe: Option<String>,
        #[arg(short, long, help = "Store market region code")]
        market: Option<String>,
    },

    #[command(alias = "ui", about = "Launch the graphical desktop user interface")]
    Gui,

    #[command(about = "Manage Xbox Live cloud saves (pull/push/status)")]
    Save {
        #[command(subcommand)]
        action: SaveAction,
    },

    #[command(about = "Manage Xbox Live multiplayer sessions and matchmaking")]
    Mpsd {
        #[command(subcommand)]
        action: MpsdAction,
    },

    #[command(about = "Generate or decrypt base64-encoded CLEP challenge data")]
    Clep {
        #[command(subcommand)]
        action: ClepAction,
    },
    #[command(about = "Uninstall an installed game and clean up local runtime cache")]
    Uninstall {
        #[clap(help = "Path to installed game directory, or product title/ID")]
        target: String,
        #[arg(long, default_value_t = false, help = "Skip syncing cloud saves to Xbox Live before removal")]
        skip_save_sync: bool,
        #[arg(long, default_value_t = false, help = "Also remove Proton compatdata prefix")]
        remove_compatdata: bool,
    },
    #[command(about = "Decode SPLicenseBlock")]
    SpLicense {
        block: String,
    },
}

#[derive(Subcommand, Debug)]
enum MpsdAction {
    #[command(about = "List active multiplayer sessions for a title")]
    List {
        #[clap(help = "Path to game directory or MSIXVC container")]
        source: String,
        #[arg(short, long, help = "Session template name (e.g. LobbySession, GameSession)")]
        template: Option<String>,
    },
    #[command(about = "Queue for SmartMatch matchmaking in a hopper")]
    Match {
        #[clap(help = "Path to game directory or MSIXVC container")]
        source: String,
        #[clap(help = "Matchmaking hopper name (e.g. QuickMatch, Ranked)")]
        hopper: String,
    },
}

#[derive(Subcommand, Debug)]
enum SaveAction {
    #[command(about = "Pull cloud saves from Xbox Live to local storage")]
    Pull {
        #[clap(help = "Path to game directory or MSIXVC container")]
        source: String,
    },
    #[command(about = "Push local saves to Xbox Live cloud storage")]
    Push {
        #[clap(help = "Path to game directory or MSIXVC container")]
        source: String,
    },
    #[command(about = "Show local and cloud save status")]
    Status {
        #[clap(long, help = "Output as JSON")]
        json: bool,
        #[clap(help = "Path to game directory or MSIXVC container")]
        source: String,
    },
}

#[derive(Subcommand, Debug)]
enum ClepAction {
    #[command(
        about = "Generate a base64-encoded CLEP challenge (V2 and V4) from SMBIOS/disk serial data"
    )]
    Generate {
        #[arg(
            long,
            help = "Base64-encoded SMBIOS data (up to 256 bytes, zero-padded)"
        )]
        smbios: Option<String>,
        #[arg(
            long,
            help = "Base64-encoded disk serial (up to 64 bytes, zero-padded)"
        )]
        disk_serial: Option<String>,
    },
    #[command(about = "Decrypt a base64-encoded CLEP challenge back into its plaintext fields")]
    Decrypt {
        #[clap(help = "Base64-encoded, obfuscated CLEP challenge data (2048 bytes)")]
        data: String,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "xodus",
    version,
    about = "Xodus - Native Xbox Game Pass & Microsoft Store GDK runtime for Linux",
    long_about = "Xodus allows downloading, managing, and running Xbox Game Pass and Microsoft Store games natively on Linux with Proton, Wine, and XGameRuntime."
)]
struct CliArgs {
    #[arg(
        short = 'l',
        long,
        help = "Sign into Microsoft account via webview authentication window"
    )]
    login: bool,

    #[arg(
        short = 'd',
        long,
        value_name = "PRODUCT",
        help = "Download a game package by Product ID / BigId or Title (e.g. '9P2N57MC619K', 'Sea of Thieves')"
    )]
    download: Option<String>,

    #[arg(
        short = 'p',
        long,
        value_name = "TARGET",
        help = "Play / launch an installed game by title, product ID, or path"
    )]
    play: Option<String>,

    #[arg(
        short = 'r',
        long,
        value_name = "TARGET",
        help = "Run / launch an installed game by title, product ID, or path (alias for --play)"
    )]
    run: Option<String>,

    #[arg(
        short = 's',
        long,
        help = "Display current Microsoft account, Xbox Live profile, and entitlement status"
    )]
    status: bool,

    #[arg(
        short = 'g',
        long,
        help = "Launch the graphical user interface"
    )]
    gui: bool,

    #[arg(
        short = 'w',
        long,
        default_value = "wine",
        help = "Wine / Proton binary or wrapper command to use when launching"
    )]
    wine: String,

    #[arg(
        short = 'e',
        long,
        help = "Specific executable name to launch within game directory"
    )]
    exe: Option<String>,

    #[arg(
        short = 'm',
        long,
        help = "Store marketplace region code (e.g. 'us', 'neutral')"
    )]
    market: Option<String>,

    #[arg(
        long,
        default_value_t = false,
        help = "Display download URLs instead of downloading (used with --download)"
    )]
    dry_run: bool,

    #[command(subcommand)]
    command: Option<SubCommand>,
}

fn launch_gui() -> ExitCode {
    let mut candidates = Vec::new();
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("xodus-gui"));
        }
    }
    candidates.push(std::path::PathBuf::from("/usr/bin/xodus-gui"));
    candidates.push(std::path::PathBuf::from("xodus-gui"));

    for cand in candidates {
        if cand.exists() || cand.to_string_lossy() == "xodus-gui" {
            if let Ok(mut child) = std::process::Command::new(&cand).spawn() {
                if let Ok(status) = child.wait() {
                    if status.success() {
                        return ExitCode::SUCCESS;
                    }
                }
            }
        }
    }
    eprintln!("[XODUS] Error: xodus-gui binary not found.");
    ExitCode::FAILURE
}

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init_from_env("XODUS_LOG");
    let client = reqwest::ClientBuilder::new()
        .user_agent(format!("xodus-cli/{}", env!("CARGO_PKG_VERSION")))
        .danger_accept_invalid_certs(true)
        .connection_verbose(true)
        .build()
        .unwrap();
    let args = CliArgs::parse();

    // Map top-level command-line flags (--login, --download, --play, --run, --status, --gui) to subcommands
    let target_command = if args.login {
        Some(SubCommand::Login)
    } else if let Some(product) = args.download {
        Some(SubCommand::Download {
            product,
            market: args.market.clone(),
            dry_run: args.dry_run,
        })
    } else if let Some(source) = args.play.or(args.run) {
        Some(SubCommand::Run {
            source,
            wine: args.wine.clone(),
            exe: args.exe.clone(),
            market: args.market.clone(),
        })
    } else if args.status {
        Some(SubCommand::Status)
    } else if args.gui {
        Some(SubCommand::Gui)
    } else {
        args.command
    };

    let Some(cmd) = target_command else {
        // If no flags or subcommands given, show help
        use clap::CommandFactory;
        let mut cmd = CliArgs::command();
        let _ = cmd.print_help();
        println!();
        return ExitCode::SUCCESS;
    };

    xodus::secrets::init_secrets().expect("Unable to initialize credentials");
    let tokens = TokenManager::with_keychain_and_memory();
    xodus::tokens::device::ensure_device_credentials(&client, &tokens).await;

    let code = match cmd {
        SubCommand::Download {
            product,
            market,
            dry_run,
        } => commands::download::run(&client, &tokens, product, market, dry_run).await,
        SubCommand::License {
            content_id,
            market,
            ciks,
        } => {
            commands::license::run(
                &client,
                &tokens,
                content_id,
                market.unwrap_or("neutral".to_string()),
                ciks,
            )
            .await
        }
        SubCommand::Login => commands::login::run(&client, &tokens).await,
        SubCommand::Logout { device } => commands::logout::run(&tokens, device).await,
        SubCommand::Status => commands::status::run(&client, &tokens).await,
        SubCommand::Gui => launch_gui(),

        SubCommand::Extract {
            path,
            destination,
            market,
        } => {
            commands::extract::run(
                &client,
                &tokens,
                path,
                destination,
                market.unwrap_or("neutral".to_string()),
            )
            .await
        }
        SubCommand::Streaming {
            source,
            destination,
            try_skip_ntfs,
            market,
            parallel,
        } => {
            commands::streaming::run(
                &client,
                &tokens,
                source,
                destination,
                try_skip_ntfs,
                parallel,
                market,
            )
            .await
        }
        #[cfg(unix)]
        SubCommand::Run {
            source,
            wine,
            exe,
            market,
        } => commands::run::run(&client, &tokens, source, wine, exe, market).await,
        SubCommand::Save { action } => match action {
            SaveAction::Pull { source } => commands::save::pull(&client, &tokens, source).await,
            SaveAction::Push { source } => commands::save::push(&client, &tokens, source).await,
            SaveAction::Status { json, source } => commands::save::status(&client, &tokens, json, source).await,
        },
        SubCommand::Mpsd { action } => match action {
            MpsdAction::List { source, template } => {
                commands::mpsd::list(&client, &tokens, source, template).await
            }
            MpsdAction::Match { source, hopper } => {
                commands::mpsd::matchmake(&client, &tokens, source, hopper).await
            }
        },
        SubCommand::Clep { action } => match action {

            ClepAction::Generate {
                smbios,
                disk_serial,
            } => commands::clep::generate(smbios, disk_serial),
            ClepAction::Decrypt { data } => commands::clep::decrypt(data),
        },
        SubCommand::Uninstall {
            target,
            skip_save_sync,
            remove_compatdata,
        } => {
            commands::uninstall::run(
                &client,
                &tokens,
                target,
                skip_save_sync,
                remove_compatdata,
            )
            .await
        }
        SubCommand::SpLicense { block } => commands::splicense::run(block),
    };


    xodus::secrets::destroy_secrets();

    code
}
