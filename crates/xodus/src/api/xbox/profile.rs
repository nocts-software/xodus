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
#[serde(rename_all = "camelCase")]
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

/// Fetch real gamer picture from Xbox Live CDN and cache locally
pub async fn fetch_or_cache_gamer_picture(
    client: &reqwest::Client,
    auth_header: &str,
    xuid: &str,
) -> Vec<u8> {
    let cache_dir = std::env::var("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("xodus/avatars");
    let cache_path = cache_dir.join(format!("{}.png", xuid));

    if let Ok(data) = std::fs::read(&cache_path) {
        if !data.is_empty() {
            return data;
        }
    }

    if let Ok(Some(profile)) = get_user_profile(client, auth_header).await {
        if !profile.display_pic.is_empty() {
            if let Ok(resp) = client.get(&profile.display_pic).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        let vec = bytes.to_vec();
                        let _ = std::fs::create_dir_all(&cache_dir);
                        let _ = std::fs::write(&cache_path, &vec);
                        return vec;
                    }
                }
            }
        }
    }

    vec![0x89, 0x50, 0x4E, 0x47]
}
