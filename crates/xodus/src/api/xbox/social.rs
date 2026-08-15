use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(alias = "id", alias = "xuid", alias = "Id", default)]
    pub xuid: String,
    #[serde(alias = "Gamertag", alias = "gamertag", alias = "modernGamertag", alias = "displayName", alias = "GamertagRaw", default)]
    pub gamertag: String,
    #[serde(alias = "displayPicRaw", alias = "DisplayPicRaw", alias = "displayPic", alias = "DisplayPic", alias = "avatar", default)]
    pub display_pic_raw: Option<String>,
    #[serde(alias = "presenceState", alias = "PresenceState", alias = "state", alias = "Presence", default)]
    pub presence_state: Option<String>,
    #[serde(alias = "presenceText", alias = "PresenceText", alias = "richPresence", alias = "RichPresence", default)]
    pub presence_text: Option<String>,
    #[serde(alias = "presenceDetails", alias = "PresenceDetails", default)]
    pub presence_details: Vec<PresenceDetail>,
    #[serde(default)]
    pub title_history: Option<TitleHistory>,
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresenceDetail {
    #[serde(alias = "TitleId", alias = "title_id", default)]
    pub title_id: Option<String>,
    #[serde(alias = "TitleName", alias = "title_name", default)]
    pub title_name: Option<String>,
    #[serde(alias = "PresenceText", alias = "presence_text", default)]
    pub presence_text: Option<String>,
    #[serde(alias = "IsPrimary", alias = "is_primary", default)]
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
    #[serde(alias = "users", alias = "people", default)]
    pub people: Vec<Person>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SocialPeopleResponse {
    #[serde(alias = "users", alias = "people", default)]
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
        // Try PeopleHub API first
        let endpoints = [
            format!("{}/users/me/people/social/decoration/presenceDetail,multipass,preferredcolor", self.people_url),
            format!("{}/users/me/people/social/decoration/detail,presenceDetail,preferredColor", self.people_url),
            format!("{}/users/me/people/social", self.people_url),
        ];

        for url in &endpoints {
            if let Ok(resp) = self
                .client
                .get(url)
                .header("Authorization", auth_header)
                .header("x-xbl-contract-version", "1")
                .header("Accept", "application/json")
                .send()
                .await
            {
                if resp.status().is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    if let Ok(res) = serde_json::from_str::<PeopleHubResponse>(&text) {
                        if !res.people.is_empty() {
                            return Ok(res.people);
                        }
                    }
                }
            }
        }

        // Fallback to social.xboxlive.com/users/me/people
        let social_url = "https://social.xboxlive.com/users/me/people";
        if let Ok(resp2) = self
            .client
            .get(social_url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "2")
            .header("Accept", "application/json")
            .send()
            .await
        {
            if resp2.status().is_success() {
                let text2 = resp2.text().await.unwrap_or_default();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text2) {
                    let mut list = Vec::new();
                    let mut xuids_to_fetch = Vec::new();
                    let users_arr = v.get("users").or_else(|| v.get("people")).and_then(|x| x.as_array());
                    if let Some(arr) = users_arr {
                        for u in arr {
                            let xuid = u.get("id").or_else(|| u.get("xuid")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
                            let gamertag = u.get("gamertag")
                                .or_else(|| u.get("Gamertag"))
                                .or_else(|| u.get("displayName"))
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let pic = u.get("displayPicRaw")
                                .or_else(|| u.get("DisplayPicRaw"))
                                .or_else(|| u.get("avatar"))
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());
                            let presence_state = u.get("presenceState")
                                .or_else(|| u.get("state"))
                                .or_else(|| u.get("presence"))
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());
                            let presence_text = u.get("presenceText")
                                .or_else(|| u.get("richPresence"))
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());

                            if gamertag.is_empty() && !xuid.is_empty() {
                                xuids_to_fetch.push(xuid.clone());
                            }

                            list.push(Person {
                                xuid,
                                gamertag,
                                display_pic_raw: pic,
                                presence_state,
                                presence_text,
                                presence_details: Vec::new(),
                                title_history: None,
                            });
                        }
                    }

                    // Enrich missing gamertags via batch profile endpoint
                    if !xuids_to_fetch.is_empty() {
                        let batch_url = "https://profile.xboxlive.com/users/batch/profile/settings";
                        let batch_body = serde_json::json!({
                            "userIds": xuids_to_fetch,
                            "settings": ["Gamertag", "GameDisplayPicRaw", "Gamerscore", "PublicGamerpic"]
                        });

                        if let Ok(b_resp) = self
                            .client
                            .post(batch_url)
                            .header("Authorization", auth_header)
                            .header("x-xbl-contract-version", "2")
                            .header("Content-Type", "application/json")
                            .json(&batch_body)
                            .send()
                            .await
                        {
                            let b_status = b_resp.status();
                            let b_text = b_resp.text().await.unwrap_or_default();
                            if b_status.is_success() {
                                if let Ok(b_val) = serde_json::from_str::<serde_json::Value>(&b_text) {
                                    if let Some(prof_users) = b_val.get("profileUsers").and_then(|x| x.as_array()) {
                                        for pu in prof_users {
                                            let id = pu.get("id").and_then(|x| x.as_str()).unwrap_or_default();
                                            if let Some(settings) = pu.get("settings").and_then(|x| x.as_array()) {
                                                let mut gt = None;
                                                let mut pic = None;
                                                for s in settings {
                                                    let name = s.get("id").or_else(|| s.get("name")).and_then(|x| x.as_str()).unwrap_or_default();
                                                    let val = s.get("value").and_then(|x| x.as_str()).unwrap_or_default();
                                                    if name == "Gamertag" && !val.is_empty() {
                                                        gt = Some(val.to_string());
                                                    } else if (name == "GameDisplayPicRaw" || name == "PublicGamerpic") && !val.is_empty() {
                                                        pic = Some(val.to_string());
                                                    }
                                                }
                                                for p in &mut list {
                                                    if p.xuid == id {
                                                        if let Some(ref g) = gt { p.gamertag = g.clone(); }
                                                        if let Some(ref pi) = pic { p.display_pic_raw = Some(pi.clone()); }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !list.is_empty() {
                        return Ok(list);
                    }
                }
            }
        }

        Ok(Vec::new())
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
