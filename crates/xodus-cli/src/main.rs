use std::process::ExitCode;

use clap::{Parser, Subcommand};
use xodus::tokens::TokenManager;

mod commands;
mod license;
mod package;
mod webview;

#[derive(Subcommand)]
enum SubCommand {
    #[command(about = "Download msixvc or xsp files fo given game")]
    Download {
        product: String,
        #[arg(short, long)]
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
    Login,
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
            help = "Attempt to skip downloading NTFS metadata to be faste while missing some files"
        )]
        try_skip_ntfs: bool,
        #[arg(short, long)]
        parallel: Option<usize>,
        #[arg(short, long)]
        market: Option<String>,
    },
    #[cfg(unix)]
    #[command(about = "Run a Game in-place with xodus wine")]
    Run {
        source: String,
        #[arg(short, long, default_value = "wine", help = "Wine / Proton binary to use (default: wine)")]
        wine: String,
        #[arg(short, long)]
        exe: Option<String>,
        #[arg(short, long)]
        market: Option<String>,
    },

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
    #[command(about = "Decode SPLicenseBlock")]
    SpLicense {
        block: String,
    },
}

#[derive(Subcommand)]
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


#[derive(Subcommand)]
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
        #[clap(help = "Path to game directory or MSIXVC container")]
        source: String,
    },
}


#[derive(Subcommand)]
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

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: SubCommand,
}

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init_from_env("XODUS_LOG");
    let client = reqwest::ClientBuilder::new()
        .user_agent(format!("xodus-cli/{}", env!("CARGO_PKG_VERSION")))
        .connection_verbose(true)
        .build()
        .unwrap();
    let args = CliArgs::parse();

    xodus::secrets::init_secrets().expect("Unable to initialize credentials");
    let tokens = TokenManager::with_keychain_and_memory();
    xodus::tokens::device::ensure_device_credentials(&client, &tokens).await;

    let code = match args.command {
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
            SaveAction::Status { source } => commands::save::status(&client, &tokens, source).await,
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
        SubCommand::SpLicense { block } => commands::splicense::run(block),
    };


    xodus::secrets::destroy_secrets();

    code
}
