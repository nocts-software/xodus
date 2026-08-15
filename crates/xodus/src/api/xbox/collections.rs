use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameCatalogItem {
    pub id: String,
    pub product_id: String,
    pub title: String,
    pub developer: String,
    pub license_type: String, // "owned" | "gamepass"
    pub installed: bool,
    pub size: String,
    pub path: String,
    pub cover: String,
    pub cloud_synced: bool,
    pub last_played: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SiglResponseItem {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    pub product_id: String,
    pub sku_id: Option<String>,
    pub product_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CollectionsBrowseResponse {
    #[serde(default)]
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InventoryResponse {
    #[serde(default)]
    pub items: Vec<InventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    #[serde(rename = "productId", default)]
    pub product_id: String,
    #[serde(rename = "itemType", default)]
    pub item_type: Option<String>,
}

/// Query Microsoft Collections API (collections.mp.microsoft.com)
pub async fn get_user_collections(
    client: &reqwest::Client,
    auth_header: &str,
) -> reqwest::Result<Vec<CollectionItem>> {
    let url = "https://collections.mp.microsoft.com/v8.0/collections/browse";
    let body = serde_json::json!({
        "market": "US",
        "locale": "en-US",
        "maxResults": 200,
        "validityType": "All",
        "entitlementFilters": [
            "Game",
            "Durable",
            "Pass",
            "Consumable"
        ]
    });

    let resp = client
        .post(url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if resp.status().is_success() {
        let res: CollectionsBrowseResponse = resp.json().await.unwrap_or_default();
        if !res.items.is_empty() {
            return Ok(res.items);
        }
    }

    // Fallback to inventory.xboxlive.com
    let inv_url = "https://inventory.xboxlive.com/users/me/inventory";
    let inv_resp = client
        .get(inv_url)
        .header("Authorization", auth_header)
        .header("x-xbl-contract-version", "4")
        .header("Accept", "application/json")
        .send()
        .await?;

    if inv_resp.status().is_success() {
        let inv: InventoryResponse = inv_resp.json().await.unwrap_or_default();
        let items: Vec<CollectionItem> = inv
            .items
            .into_iter()
            .map(|i| CollectionItem {
                product_id: i.product_id,
                sku_id: None,
                product_type: i.item_type,
            })
            .collect();
        return Ok(items);
    }

    Ok(Vec::new())
}

/// Query PC Game Pass Catalog list
pub async fn get_gamepass_sigl_ids(
    client: &reqwest::Client,
) -> reqwest::Result<Vec<String>> {
    let url = "https://catalog.gamepass.com/sigls/v2?id=29447090-7171-460d-a26b-67e4526fed86&language=en-us&market=US";
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let items: Vec<SiglResponseItem> = resp.json().await.unwrap_or_default();
        let ids = items.into_iter().map(|i| i.id).filter(|id| !id.is_empty()).collect();
        return Ok(ids);
    }
    Ok(Vec::new())
}
