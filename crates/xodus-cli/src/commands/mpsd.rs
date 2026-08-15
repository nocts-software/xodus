use std::path::PathBuf;
use std::process::ExitCode;
use xodus::api::xbox::auth::get_xsts_auth_header;
use xodus::api::xbox::get_or_request_xsts;
use xodus::api::xbox::mpsd::{
    MatchmakingTicketRequest, MpsdClient, SessionReference,
};
use xodus::tokens::TokenManager;

pub async fn list(
    client: &reqwest::Client,
    tokens: &TokenManager,
    source: String,
    template: Option<String>,
) -> ExitCode {
    println!("Querying Xbox Live MPSD sessions for: {source}");
    let scid = resolve_scid(&source).await;
    let template_name = template.unwrap_or_else(|| "default".to_string());

    let xsts = match get_or_request_xsts(client, tokens, "http://xboxlive.com").await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get XSTS auth token: {e}");
            return ExitCode::FAILURE;
        }
    };

    let auth_header = get_xsts_auth_header(xsts);
    let mpsd = MpsdClient::new(client);

    println!("Listing sessions for SCID {scid} (Template: {template_name})...");
    match mpsd.query_sessions(&auth_header, &scid, &template_name).await {
        Ok(res) => {
            if res.results.is_empty() {
                println!("No active open sessions found.");
            } else {
                println!("Found {} active session(s):", res.results.len());
                for item in res.results {
                    println!(
                        "  - Session: {:<32} | Members: {}",
                        item.session_ref.session_name, item.member_count
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to query MPSD sessions: {e}");
            ExitCode::FAILURE
        }
    }
}

pub async fn matchmake(
    client: &reqwest::Client,
    tokens: &TokenManager,
    source: String,
    hopper: String,
) -> ExitCode {
    println!("Initiating Xbox Live SmartMatch matchmaking for: {source} (Hopper: {hopper})");
    let scid = resolve_scid(&source).await;

    let xsts = match get_or_request_xsts(client, tokens, "http://xboxlive.com").await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get XSTS auth token: {e}");
            return ExitCode::FAILURE;
        }
    };

    let auth_header = get_xsts_auth_header(xsts);
    let mpsd = MpsdClient::new(client);

    let session_ref = SessionReference {
        scid: scid.clone(),
        session_template_name: "MatchSession".to_string(),
        session_name: format!("match-{}", uuid::Uuid::new_v4()),
    };

    let req = MatchmakingTicketRequest {
        give_up_duration: 120,
        preserve_session: "Never".to_string(),
        ticket_session_ref: session_ref,
    };

    match mpsd.create_match_ticket(&auth_header, &scid, &hopper, &req).await {
        Ok(Some(ticket)) => {
            println!("Matchmaking Ticket Created: {}", ticket.ticket_id);
            println!("Estimated Wait Time: {}s", ticket.estimated_wait_time);
            println!("Polling for match status (Press Ctrl+C to cancel)...");

            for i in 1..=20 {
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                if let Ok(Some(status)) = mpsd
                    .get_match_ticket(&auth_header, &scid, &hopper, &ticket.ticket_id)
                    .await
                {
                    let st = status.status.unwrap_or_else(|| "Searching".to_string());
                    println!("  [{i:02}] Status: {st}");
                    if st == "Found" {
                        if let Some(target) = status.target_session_ref {
                            println!("Match Found! Target Session: {}", target.session_name);
                        }
                        return ExitCode::SUCCESS;
                    } else if st == "Expired" || st == "Canceled" {
                        println!("Matchmaking ended with status: {st}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!("Matchmaking ticket was rejected or hopper is unavailable.");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Network error submitting matchmaking ticket: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn resolve_scid(source: &str) -> String {
    let p = PathBuf::from(source);
    let mut title_id = "77BB5AFB".to_string();

    let cfg_path = if p.is_dir() {
        p.join("MicrosoftGame.config")
    } else {
        p.parent().unwrap().join("MicrosoftGame.config")
    };

    if cfg_path.exists() {
        if let Ok(xml) = tokio::fs::read_to_string(&cfg_path).await {
            if let Some(pos) = xml.find("TitleId=\"") {
                let rest = &xml[pos + 9..];
                if let Some(end) = rest.find('"') {
                    title_id = rest[..end].to_string();
                }
            }
        }
    }

    format!("00000000-0000-0000-0000-0000{}", title_id.to_lowercase())
}
