use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSetting {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUser {
    pub id: String,
    #[serde(default)]
    pub host_id: Option<String>,
    #[serde(default)]
    pub settings: Vec<ProfileSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    #[serde(default)]
    pub profile_users: Vec<ProfileUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    pub xuid: String,
    pub gamertag: String,
    pub display_pic: String,
    pub gamerscore: String,
    pub tier: String,
}

impl UserProfile {
    pub fn from_profile_user(user: &ProfileUser) -> Self {
        let mut gamertag = String::new();
        let mut display_pic = String::new();
        let mut gamerscore = "0".to_string();
        let mut tier = "Gold".to_string();

        for setting in &user.settings {
            match setting.id.as_str() {
                "Gamertag" | "ModernGamertag" => {
                    if gamertag.is_empty() || setting.id == "ModernGamertag" {
                        gamertag = setting.value.clone();
                    }
                }
                "GameDisplayPicRaw" => {
                    display_pic = setting.value.clone();
                }
                "Gamerscore" => {
                    gamerscore = setting.value.clone();
                }
                "AccountTier" => {
                    tier = setting.value.clone();
                }
                _ => {}
            }
        }

        Self {
            xuid: user.id.clone(),
            gamertag,
            display_pic,
            gamerscore,
            tier,
        }
    }
}

/// Query Xbox Live Profile API to retrieve real gamer avatar, gamertag, and stats
pub async fn get_user_profile(
    client: &reqwest::Client,
    auth_header: &str,
) -> reqwest::Result<Option<UserProfile>> {
    let url = "https://profile.xboxlive.com/users/me/profile/settings?settings=Gamertag,ModernGamertag,GameDisplayPicRaw,Gamerscore,AccountTier";
    let resp = client
        .get(url)
        .header("Authorization", auth_header)
        .header("x-xbl-contract-version", "2")
        .header("Accept", "application/json")
        .send()
        .await?;

    if resp.status().is_success() {
        let data: ProfileResponse = resp.json().await.unwrap_or_default();
        if let Some(first) = data.profile_users.first() {
            return Ok(Some(UserProfile::from_profile_user(first)));
        }
    } else {
        log::warn!("Profile query returned HTTP {}", resp.status());
    }
    Ok(None)
}
