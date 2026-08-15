use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub sigl_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    #[serde(rename = "productId", default)]
    pub product_id: String,
    #[serde(rename = "skuId", default)]
    pub sku_id: Option<String>,
    #[serde(rename = "productType", default)]
    pub product_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CollectionsBrowseResponse {
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    #[serde(default)]
    pub continuation_token: Option<String>,
    #[serde(default)]
    pub paging_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InventoryResponse {
    #[serde(default)]
    pub items: Vec<InventoryItem>,
    #[serde(rename = "continuationToken", default)]
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    #[serde(rename = "productId", default)]
    pub product_id: String,
    #[serde(rename = "itemType", default)]
    pub item_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DisplayCatalogResponse {
    #[serde(rename = "Products", default)]
    pub products: Vec<DisplayProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DisplayProduct {
    #[serde(rename = "ProductId", default)]
    pub product_id: String,
    #[serde(rename = "LocalizedProperties", default)]
    pub localized_properties: Vec<LocalizedProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedProperty {
    #[serde(rename = "ProductTitle", default)]
    pub product_title: String,
    #[serde(rename = "DeveloperName", default)]
    pub developer_name: String,
    #[serde(rename = "PublisherName", default)]
    pub publisher_name: String,
    #[serde(rename = "Images", default)]
    pub images: Vec<ProductImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProductImage {
    #[serde(rename = "ImagePurpose", default)]
    pub image_purpose: String,
    #[serde(rename = "Uri", default)]
    pub uri: String,
}

/// Query Microsoft Collections API with full pagination across all pages
/// Query Microsoft Collections API with full pagination across all pages
pub async fn get_user_collections(
    client: &reqwest::Client,
    auth_header: &str,
    xuid: Option<&str>,
) -> reqwest::Result<Vec<CollectionItem>> {
    let mut all_items = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Try TitleHub endpoints (title history & entitlements on Xbox Live)
    let mut titlehub_urls = Vec::new();
    if let Some(id) = xuid {
        titlehub_urls.push(format!("https://titlehub.xboxlive.com/users/xuid({id})/titles/titlehistory/decoration/scid,image,detail"));
        titlehub_urls.push(format!("https://titlehub.xboxlive.com/users/xuid({id})/titles/titlehistory/decoration/detail"));
    }
    titlehub_urls.push("https://titlehub.xboxlive.com/users/me/titles/titlehistory/decoration/scid,image,detail".to_string());
    titlehub_urls.push("https://titlehub.xboxlive.com/users/me/titles/titlehistory/decoration/detail".to_string());

    for th_url in &titlehub_urls {
        if let Ok(th_resp) = client
            .get(th_url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "2")
            .header("Accept-Language", "en-US")
            .header("Accept", "application/json")
            .send()
            .await
        {
            let th_status = th_resp.status();
            let th_text = th_resp.text().await.unwrap_or_default();
            if th_status.is_success() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&th_text) {
                    if let Some(titles) = v.get("titles").and_then(|t| t.as_array()) {
                        for t in titles {
                        let modern_id = t.get("modernTitleId").and_then(|x| x.as_str()).map(|s| s.to_string());
                        let pfn = t.get("pfn").and_then(|x| x.as_str()).map(|s| s.to_string());
                        let raw_id = t.get("titleId").map(|x| {
                            if let Some(s) = x.as_str() { s.to_string() }
                            else if let Some(n) = x.as_i64() { n.to_string() }
                            else if let Some(u) = x.as_u64() { u.to_string() }
                            else { String::new() }
                        }).filter(|s| !s.is_empty());

                        let target_id = modern_id.or(raw_id).or(pfn);
                        if let Some(id) = target_id {
                            if seen_ids.insert(id.clone()) {
                                let title_type = t.get("type").and_then(|x| x.as_str()).map(|s| s.to_string());
                                all_items.push(CollectionItem {
                                    product_id: id,
                                    sku_id: None,
                                    product_type: title_type,
                                });
                            }
                        }
                    }
                }
            }
        }
        }
    }

    // 2. Try Microsoft Collections browse endpoints
    let collection_urls = [
        "https://collections.mp.microsoft.com/v8.0/collections/users/me/browse",
        "https://collections.mp.microsoft.com/v8.0/collections/browse",
    ];

    for url in &collection_urls {
        let mut continuation_token: Option<String> = None;
        loop {
            let mut body_map = serde_json::json!({
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

            if let Some(token) = &continuation_token {
                if let Some(obj) = body_map.as_object_mut() {
                    obj.insert("continuationToken".to_string(), serde_json::json!(token));
                }
            }

            let resp = client
                .post(*url)
                .header("Authorization", auth_header)
                .header("Content-Type", "application/json")
                .json(&body_map)
                .send()
                .await?;

            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status.is_success() {
                let res: CollectionsBrowseResponse = serde_json::from_str(&text).unwrap_or_default();
                for item in res.items {
                    if !item.product_id.is_empty() && seen_ids.insert(item.product_id.clone()) {
                        all_items.push(item);
                    }
                }

                let next = res.continuation_token.or(res.paging_token);
                if next.is_some() && next != continuation_token {
                    continuation_token = next;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    if !all_items.is_empty() {
        return Ok(all_items);
    }

    // Fallback 1: titlehub.xboxlive.com (Returns all owned/played titles on Xbox & PC)
    let titlehub_url = "https://titlehub.xboxlive.com/users/me/titles/titlehistory/decoration/scid,image,detail";
    if let Ok(th_resp) = client
        .get(titlehub_url)
        .header("Authorization", auth_header)
        .header("x-xbl-contract-version", "2")
        .header("Accept", "application/json")
        .send()
        .await
    {
        let th_status = th_resp.status();
        let th_text = th_resp.text().await.unwrap_or_default();
        if th_status.is_success() {
            #[derive(Deserialize, Default)]
            struct TitleHistoryItem {
                #[serde(alias = "titleId", alias = "pfn", alias = "modernTitleId", default)]
                title_id: Option<String>,
                #[serde(alias = "type", default)]
                title_type: Option<String>,
                #[serde(alias = "name", default)]
                name: Option<String>,
            }
            #[derive(Deserialize, Default)]
            struct TitleHistoryResp {
                #[serde(default)]
                titles: Vec<TitleHistoryItem>,
            }
            if let Ok(th_data) = serde_json::from_str::<TitleHistoryResp>(&th_text) {
                for t in th_data.titles {
                    if let Some(tid) = t.title_id {
                        if !tid.is_empty() && seen_ids.insert(tid.clone()) {
                            all_items.push(CollectionItem {
                                product_id: tid,
                                sku_id: None,
                                product_type: t.title_type,
                            });
                        }
                    }
                }
            }
        }
    }

    if !all_items.is_empty() {
        return Ok(all_items);
    }

    // Fallback 2: purchase.mp.microsoft.com/v8.0/users/me/entitlements
    let purchase_url = "https://purchase.mp.microsoft.com/v8.0/users/me/entitlements";
    if let Ok(p_resp) = client
        .get(purchase_url)
        .header("Authorization", auth_header)
        .header("Accept", "application/json")
        .send()
        .await
    {
        let p_status = p_resp.status();
        let p_text = p_resp.text().await.unwrap_or_default();
        if p_status.is_success() {
            #[derive(Deserialize, Default)]
            struct PurchaseItem {
                #[serde(alias = "productId", default)]
                product_id: String,
                #[serde(alias = "entitlementType", alias = "productType", default)]
                product_type: Option<String>,
            }
            #[derive(Deserialize, Default)]
            struct PurchaseResp {
                #[serde(default)]
                items: Vec<PurchaseItem>,
            }
            if let Ok(p_data) = serde_json::from_str::<PurchaseResp>(&p_text) {
                for item in p_data.items {
                    if !item.product_id.is_empty() && seen_ids.insert(item.product_id.clone()) {
                        all_items.push(CollectionItem {
                            product_id: item.product_id,
                            sku_id: None,
                            product_type: item.product_type,
                        });
                    }
                }
            }
        }
    }

    if !all_items.is_empty() {
        return Ok(all_items);
    }

    // Fallback 3: inventory.xboxlive.com with pagination
    let mut inv_token: Option<String> = None;
    loop {
        let inv_url = if let Some(ref token) = inv_token {
            format!("https://inventory.xboxlive.com/users/me/inventory?continuationToken={token}")
        } else {
            "https://inventory.xboxlive.com/users/me/inventory".to_string()
        };

        let inv_resp = client
            .get(&inv_url)
            .header("Authorization", auth_header)
            .header("x-xbl-contract-version", "4")
            .header("Accept", "application/json")
            .send()
            .await?;

        let inv_status = inv_resp.status();
        let inv_text = inv_resp.text().await.unwrap_or_default();
        if inv_status.is_success() {
            let inv: InventoryResponse = serde_json::from_str(&inv_text).unwrap_or_default();
            for item in inv.items {
                if !item.product_id.is_empty() && seen_ids.insert(item.product_id.clone()) {
                    all_items.push(CollectionItem {
                        product_id: item.product_id,
                        sku_id: None,
                        product_type: item.item_type,
                    });
                }
            }

            if let Some(next) = inv.continuation_token {
                if Some(&next) != inv_token.as_ref() {
                    inv_token = Some(next);
                    continue;
                }
            }
        }
        break;
    }

    Ok(all_items)
}

/// Query Microsoft Display Catalog to enrich a list of product IDs with official titles, box art, and developer info
/// Caches all retrieved metadata in SQLite database to prevent redundant HTTP requests.
pub async fn enrich_products_catalog(
    client: &reqwest::Client,
    product_ids: &[String],
) -> Vec<GameCatalogItem> {
    let db = crate::db::Database::open_default().ok();
    let mut catalog_items = Vec::new();
    let mut missing_ids = Vec::new();

    // 1. Check local SQLite cache first
    for pid in product_ids {
        let mut found = false;
        if let Some(ref database) = db {
            if let Ok(Some(cached)) = database.get_catalog_product(pid) {
                catalog_items.push(GameCatalogItem {
                    id: cached.product_id.clone(),
                    product_id: cached.product_id.clone(),
                    title: cached.title,
                    developer: cached.developer,
                    license_type: "owned".to_string(),
                    installed: false,
                    size: cached.size_in_bytes.map(|s| format!("{:.1} GB", s as f64 / 1_073_741_824.0)).unwrap_or_else(|| "Standard".to_string()),
                    path: format!("/mnt/w11/XboxGames/{}", cached.product_id),
                    cover: cached.poster_url.unwrap_or_else(|| "https://shared.steamstatic.com/store_item_assets/steam/apps/1817230/library_600x900.jpg".to_string()),
                    cloud_synced: true,
                    last_played: "Licensed".to_string(),
                });
                found = true;
            }
        }
        if !found {
            missing_ids.push(pid.clone());
        }
    }

    if missing_ids.is_empty() {
        return catalog_items;
    }

    // 2. Concurrently fetch missing items from Display Catalog API in batches of 20
    let chunks: Vec<Vec<String>> = missing_ids.chunks(20).map(|c| c.to_vec()).collect();
    let mut tasks = Vec::new();

    for chunk in chunks {
        let client_clone = client.clone();
        tasks.push(tokio::spawn(async move {
            let big_ids = chunk.join(",");
            let url = format!(
                "https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={big_ids}&market=US&languages=en-us"
            );
            if let Ok(resp) = client_clone.get(&url).header("MS-CV", "0.1").send().await {
                if resp.status().is_success() {
                    let dcat: DisplayCatalogResponse = resp.json().await.unwrap_or_default();
                    return dcat.products;
                }
            }
            Vec::new()
        }));
    }

    for task in tasks {
        if let Ok(products) = task.await {
            for prod in products {
                if let Some(prop) = prod.localized_properties.first() {
                    let title = if !prop.product_title.is_empty() {
                        prop.product_title.clone()
                    } else {
                        prod.product_id.clone()
                    };

                    let developer = if !prop.developer_name.is_empty() {
                        prop.developer_name.clone()
                    } else if !prop.publisher_name.is_empty() {
                        prop.publisher_name.clone()
                    } else {
                        "Xbox Game Studios".to_string()
                    };

                    let poster_url = prop
                        .images
                        .iter()
                        .find(|img| {
                            img.image_purpose == "Poster"
                                || img.image_purpose == "BoxArt"
                                || img.image_purpose == "TitledHeroArt"
                                || img.image_purpose == "FeaturePromotionalSquareArt"
                        })
                        .map(|img| {
                            if img.uri.starts_with("//") {
                                format!("https:{}", img.uri)
                            } else {
                                img.uri.clone()
                            }
                        })
                        .unwrap_or_else(|| {
                            "https://shared.steamstatic.com/store_item_assets/steam/apps/1817230/library_600x900.jpg".to_string()
                        });

                    // Cache in SQLite database
                    if let Some(ref database) = db {
                        let _ = database.save_catalog_product(&crate::db::CachedCatalogProduct {
                            product_id: prod.product_id.clone(),
                            title: title.clone(),
                            developer: developer.clone(),
                            publisher: prop.publisher_name.clone(),
                            description: String::new(),
                            poster_url: Some(poster_url.clone()),
                            hero_url: None,
                            package_family_name: None,
                            content_id: None,
                            size_in_bytes: None,
                            raw_json: None,
                            updated_at: 0,
                            ttl: 604800,
                        });
                    }

                    catalog_items.push(GameCatalogItem {
                        id: prod.product_id.clone(),
                        product_id: prod.product_id.clone(),
                        title,
                        developer,
                        license_type: "owned".to_string(),
                        installed: false,
                        size: "Standard".to_string(),
                        path: format!("/mnt/w11/XboxGames/{}", prod.product_id),
                        cover: poster_url,
                        cloud_synced: true,
                        last_played: "Licensed".to_string(),
                    });
                }
            }
        }
    }

    catalog_items
}

/// Query official Microsoft PC Game Pass Catalog (500+ PC titles)
pub async fn get_gamepass_catalog_ids(
    client: &reqwest::Client,
) -> reqwest::Result<Vec<String>> {
    let url = "https://catalog.gamepass.com/sigls/v2?id=fdd9e2a7-0fee-49f6-ad69-4354098401ff&language=en-us&market=US";
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let items: Vec<SiglResponseItem> = resp.json().await.unwrap_or_default();
        let ids = items.into_iter().filter_map(|i| i.id).filter(|id| !id.is_empty()).collect();
        return Ok(ids);
    }
    Ok(Vec::new())
}

pub async fn get_gamepass_sigl_ids(
    client: &reqwest::Client,
) -> reqwest::Result<Vec<String>> {
    get_gamepass_catalog_ids(client).await
}

/// Check if the user has an active PC Game Pass or Xbox Game Pass Ultimate subscription
pub async fn check_user_gamepass_subscription(
    client: &reqwest::Client,
    auth_header: &str,
    xuid: Option<&str>,
) -> bool {
    let collections = get_user_collections(client, auth_header, xuid).await.unwrap_or_default();
    
    // Known Game Pass subscription product IDs
    let gamepass_product_ids: HashSet<&str> = [
        "CFQ7TTC0KGQ8", // PC Game Pass
        "CFQ7TTC0KHS0", // Xbox Game Pass Ultimate
        "9P1N75Q4K9Q8", // PC Game Pass (Alt)
        "CFQ7TTC0K6L8", // Xbox Game Pass Core
    ].into_iter().collect();

    for item in &collections {
        if gamepass_product_ids.contains(item.product_id.as_str()) {
            return true;
        }
        if let Some(ref ptype) = item.product_type {
            if ptype.eq_ignore_ascii_case("Pass") || ptype.eq_ignore_ascii_case("Subscription") {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gamepass_catalog_and_enrichment() {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64)")
            .build()
            .unwrap();
        let ids = get_gamepass_catalog_ids(&client).await.unwrap();
        assert!(ids.len() > 100, "Should return over 100 Game Pass IDs");
        let sample = &ids[..10];
        let enriched = enrich_products_catalog(&client, sample).await;
        assert_eq!(enriched.len(), 10, "Should enrich all 10 sample products");
        assert!(!enriched[0].title.is_empty(), "Product should have non-empty title");
    }
}
