use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReference {
    pub scid: String,
    pub session_template_name: String,
    pub session_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionMember {
    #[serde(default)]
    pub gamertag: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub secure_device_address: Option<String>,
    #[serde(default)]
    pub constants: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MultiplayerSession {
    #[serde(default)]
    pub members_info: Option<MembersInfo>,
    #[serde(default)]
    pub members: HashMap<String, SessionMember>,
    #[serde(default)]
    pub constants: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub servers: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MembersInfo {
    #[serde(default)]
    pub first: u32,
    #[serde(default)]
    pub next: u32,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub accepted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQueryItem {
    pub session_ref: SessionReference,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub member_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionQueryResult {
    #[serde(default)]
    pub results: Vec<SessionQueryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchmakingTicketRequest {
    pub give_up_duration: u32,
    pub preserve_session: String,
    pub ticket_session_ref: SessionReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchmakingTicketResponse {
    pub ticket_id: String,
    pub estimated_wait_time: u32,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub target_session_ref: Option<SessionReference>,
}

pub struct MpsdClient<'a> {
    client: &'a reqwest::Client,
    base_url: String,
    match_url: String,
}

impl<'a> MpsdClient<'a> {
    pub fn new(client: &'a reqwest::Client) -> Self {
        Self {
            client,
            base_url: "https://sessiondirectory.xboxlive.com".to_string(),
            match_url: "https://matchmaking.xboxlive.com".to_string(),
        }
    }

    /// Create or update an MPSD multiplayer session
    pub async fn create_or_update_session(
        &self,
        auth_header: &str,
        scid: &str,
        template_name: &str,
        session_name: &str,
        session_body: &MultiplayerSession,
    ) -> reqwest::Result<Option<MultiplayerSession>> {
        let url = format!(
            "{}/serviceconfigs/{}/sessiontemplates/{}/sessions/{}",
            self.base_url, scid, template_name, session_name
        );
        let resp = self
            .client
            .put(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "107")
            .header("Content-Type", "application/json")
            .json(session_body)
            .send()
            .await?;

        if resp.status().is_success() || resp.status() == reqwest::StatusCode::CREATED {
            Ok(resp.json().await.ok())
        } else {
            log::warn!("MPSD session create/update failed with status: {}", resp.status());
            Ok(None)
        }
    }

    /// Get current state of an MPSD session
    pub async fn get_session(
        &self,
        auth_header: &str,
        scid: &str,
        template_name: &str,
        session_name: &str,
    ) -> reqwest::Result<Option<MultiplayerSession>> {
        let url = format!(
            "{}/serviceconfigs/{}/sessiontemplates/{}/sessions/{}",
            self.base_url, scid, template_name, session_name
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "107")
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(Some(resp.json().await?))
        } else {
            Ok(None)
        }
    }

    /// Leave a session
    pub async fn leave_session(
        &self,
        auth_header: &str,
        scid: &str,
        template_name: &str,
        session_name: &str,
        member_id: u32,
    ) -> reqwest::Result<bool> {
        let url = format!(
            "{}/serviceconfigs/{}/sessiontemplates/{}/sessions/{}/members/{}",
            self.base_url, scid, template_name, session_name, member_id
        );
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "107")
            .send()
            .await?;

        Ok(resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT)
    }

    /// Query available open sessions
    pub async fn query_sessions(
        &self,
        auth_header: &str,
        scid: &str,
        template_name: &str,
    ) -> reqwest::Result<SessionQueryResult> {
        let url = format!(
            "{}/serviceconfigs/{}/sessiontemplates/{}/sessions?fields=members,properties",
            self.base_url, scid, template_name
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "107")
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await.unwrap_or_default())
        } else {
            Ok(SessionQueryResult::default())
        }
    }

    /// Submit a matchmaking ticket (SmartMatch)
    pub async fn create_match_ticket(
        &self,
        auth_header: &str,
        scid: &str,
        hopper_name: &str,
        ticket_req: &MatchmakingTicketRequest,
    ) -> reqwest::Result<Option<MatchmakingTicketResponse>> {
        let url = format!(
            "{}/serviceconfigs/{}/hoppers/{}/tickets",
            self.match_url, scid, hopper_name
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "107")
            .header("Content-Type", "application/json")
            .json(ticket_req)
            .send()
            .await?;

        if resp.status().is_success() || resp.status() == reqwest::StatusCode::CREATED {
            Ok(resp.json().await.ok())
        } else {
            log::warn!("SmartMatch ticket request failed with status: {}", resp.status());
            Ok(None)
        }
    }

    /// Poll matchmaking ticket status
    pub async fn get_match_ticket(
        &self,
        auth_header: &str,
        scid: &str,
        hopper_name: &str,
        ticket_id: &str,
    ) -> reqwest::Result<Option<MatchmakingTicketResponse>> {
        let url = format!(
            "{}/serviceconfigs/{}/hoppers/{}/tickets/{}",
            self.match_url, scid, hopper_name, ticket_id
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "107")
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await.ok())
        } else {
            Ok(None)
        }
    }
}
