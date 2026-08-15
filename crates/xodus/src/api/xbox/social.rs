use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub xuid: String,
    #[serde(rename = "Gamertag", default)]
    pub gamertag: String,
    #[serde(rename = "displayPicRaw", default)]
    pub display_pic_raw: Option<String>,
    #[serde(default)]
    pub presence_state: Option<String>,
    #[serde(default)]
    pub presence_text: Option<String>,
    #[serde(default)]
    pub presence_details: Vec<PresenceDetail>,
    #[serde(default)]
    pub title_history: Option<TitleHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresenceDetail {
    #[serde(default)]
    pub title_id: Option<String>,
    #[serde(default)]
    pub title_name: Option<String>,
    #[serde(default)]
    pub presence_text: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TitleHistory {
    #[serde(default)]
    pub last_time_played: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PeopleHubResponse {
    #[serde(default)]
    pub people: Vec<Person>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPresenceRequest {
    pub state: String,
}

pub struct SocialClient<'a> {
    client: &'a reqwest::Client,
    people_url: String,
    presence_url: String,
}

impl<'a> SocialClient<'a> {
    pub fn new(client: &'a reqwest::Client) -> Self {
        Self {
            client,
            people_url: "https://peoplehub.xboxlive.com".to_string(),
            presence_url: "https://userpresence.xboxlive.com".to_string(),
        }
    }

    /// Fetch user's friends list and social presence
    pub async fn get_friends(&self, auth_header: &str) -> reqwest::Result<Vec<Person>> {
        let url = format!(
            "{}/users/me/people/social/decoration/detail,presenceDetail,preferredColor",
            self.people_url
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "1")
            .header("Accept", "application/json")
            .send()
            .await?;

        if resp.status().is_success() {
            let res: PeopleHubResponse = resp.json().await.unwrap_or_default();
            Ok(res.people)
        } else {
            log::warn!("PeopleHub query failed with status: {}", resp.status());
            Ok(Vec::new())
        }
    }

    /// Update user presence state (e.g. Active, Away, Inactive)
    pub async fn set_presence(&self, auth_header: &str, state: &str) -> reqwest::Result<bool> {
        let url = format!("{}/users/me/devices/current/titles/current", self.presence_url);
        let body = SetPresenceRequest {
            state: state.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "3")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        Ok(resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT)
    }
}
