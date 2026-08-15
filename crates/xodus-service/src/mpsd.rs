use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use xodus::api::xbox::auth::get_xsts_auth_header;
use xodus::api::xbox::get_or_request_xsts;
use xodus::api::xbox::mpsd::{
    MatchmakingTicketRequest, MatchmakingTicketResponse, MpsdClient, MultiplayerSession,
    SessionReference,
};
use xodus::tokens::TokenManager;

#[derive(Default, Clone)]
pub struct MpsdManager {
    sessions: Arc<RwLock<HashMap<String, MultiplayerSession>>>,
    tickets: Arc<RwLock<HashMap<String, MatchmakingTicketResponse>>>,
}

impl MpsdManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create_or_join(
        &self,
        tokens: Arc<TokenManager>,
        scid: String,
        template: String,
        session_name: String,
        session: MultiplayerSession,
    ) -> Option<MultiplayerSession> {
        let key = format!("{scid}:{template}:{session_name}");
        {
            let mut w = self.sessions.write().await;
            w.insert(key.clone(), session.clone());
        }

        let client = reqwest::Client::new();
        if let Ok(xsts) = get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await {
            let auth = get_xsts_auth_header(xsts);
            let mpsd = MpsdClient::new(&client);
            if let Ok(Some(remote_session)) = mpsd
                .create_or_update_session(&auth, &scid, &template, &session_name, &session)
                .await
            {
                let mut w = self.sessions.write().await;
                w.insert(key, remote_session.clone());
                return Some(remote_session);
            }
        }
        Some(session)
    }

    pub async fn get_session(&self, scid: &str, template: &str, session_name: &str) -> Option<MultiplayerSession> {
        let key = format!("{scid}:{template}:{session_name}");
        let r = self.sessions.read().await;
        r.get(&key).cloned()
    }

    pub async fn start_matchmaking(
        &self,
        tokens: Arc<TokenManager>,
        scid: String,
        hopper: String,
        session_ref: SessionReference,
    ) -> Option<String> {
        let client = reqwest::Client::new();
        let xsts = get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await.ok()?;
        let auth = get_xsts_auth_header(xsts);
        let mpsd = MpsdClient::new(&client);

        let req = MatchmakingTicketRequest {
            give_up_duration: 120,
            preserve_session: "Never".to_string(),
            ticket_session_ref: session_ref,
        };

        if let Ok(Some(ticket)) = mpsd.create_match_ticket(&auth, &scid, &hopper, &req).await {
            let ticket_id = ticket.ticket_id.clone();
            {
                let mut w = self.tickets.write().await;
                w.insert(ticket_id.clone(), ticket.clone());
            }

            // Spawn background polling task
            let tickets_map = self.tickets.clone();
            let tid = ticket_id.clone();
            let scid_c = scid.clone();
            let hopper_c = hopper.clone();
            let auth_c = auth.clone();
            tokio::spawn(async move {
                let bg_client = reqwest::Client::new();
                let bg_mpsd = MpsdClient::new(&bg_client);
                for _ in 0..30 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    if let Ok(Some(updated)) = bg_mpsd.get_match_ticket(&auth_c, &scid_c, &hopper_c, &tid).await {
                        let status = updated.status.clone().unwrap_or_default();
                        {
                            let mut w = tickets_map.write().await;
                            w.insert(tid.clone(), updated);
                        }
                        if status == "Found" || status == "Expired" || status == "Canceled" {
                            break;
                        }
                    }
                }
            });


            Some(ticket_id)
        } else {
            None
        }
    }

    pub async fn get_ticket_status(&self, ticket_id: &str) -> Option<MatchmakingTicketResponse> {
        let r = self.tickets.read().await;
        r.get(ticket_id).cloned()
    }
}
