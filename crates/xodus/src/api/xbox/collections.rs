use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const DEFAULT_COVER_URL: &str = "https://assets.xboxservices.com/assets/default_boxart.png";

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
    #[serde(rename = "entitlementType", default)]
    pub entitlement_type: Option<String>,
    #[serde(rename = "status", default)]
    pub status: Option<String>,
    #[serde(rename = "isEntitlementValid", default)]
    pub is_entitlement_valid: Option<bool>,
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
    #[serde(rename = "ProductType", default)]
    pub product_type: Option<String>,
    #[serde(rename = "ProductKind", default)]
    pub product_kind: Option<String>,
    #[serde(rename = "ProductFamilyName", default)]
    pub product_family_name: Option<String>,
    #[serde(rename = "LocalizedProperties", default)]
    pub localized_properties: Vec<LocalizedProperty>,
    #[serde(rename = "AllowedPlatforms", default)]
    pub allowed_platforms: Option<Vec<String>>,
    #[serde(rename = "Properties", default)]
    pub properties: Option<serde_json::Value>,
    #[serde(rename = "DisplaySkuAvailabilities", default)]
    pub display_sku_availabilities: Option<Vec<serde_json::Value>>,
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

/// Query Microsoft TitleHub and Microsoft Collections APIs with full metadata and resolve all user-owned games
pub async fn get_user_owned_catalog_items(
    client: &reqwest::Client,
    auth_header: &str,
    licensing_auth_header: Option<&str>,
    xuid: Option<&str>,
    db: Option<&crate::db::Database>,
) -> Vec<GameCatalogItem> {
    let mut catalog_items = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut store_big_ids = Vec::new();
    let mut direct_titles = Vec::new();

    // 1. Query TitleHub endpoints (Xbox Live title history & entitlements with full metadata)
    let mut titlehub_urls = Vec::new();
    if let Some(id) = xuid {
        titlehub_urls.push(format!("https://titlehub.xboxlive.com/users/xuid({id})/titles/titlehistory/decoration/scid,image,detail"));
    }
    titlehub_urls.push("https://titlehub.xboxlive.com/users/me/titles/titlehistory/decoration/scid,image,detail".to_string());

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
                            let raw_name = t.get("name").and_then(|x| x.as_str()).unwrap_or("");
                            let t_type = t.get("type").and_then(|n| n.as_str()).unwrap_or("");
                            if raw_name.is_empty() || t_type != "Game" {
                                continue;
                            }

                            let devices: Vec<String> = t.get("devices").and_then(|d| d.as_array())
                                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default();

                            let is_pure_win32 = devices.len() == 1 && devices[0] == "Win32";
                            let is_pure_360 = devices.len() == 1 && devices[0] == "Xbox360";
                            if is_pure_win32 || is_pure_360 {
                                continue;
                            }

                            let modern_id = t.get("modernTitleId").and_then(|n| n.as_str()).unwrap_or("");
                            let pfn = t.get("pfn").and_then(|n| n.as_str()).unwrap_or("");

                            let mut pc_product_id = String::new();
                            let mut any_product_id = String::new();

                            if let Some(detail) = t.get("detail") {
                                if let Some(availabilities) = detail.get("availabilities").and_then(|a| a.as_array()) {
                                    for av in availabilities {
                                        let pid = av.get("ProductId").and_then(|p| p.as_str()).unwrap_or("");
                                        if pid.len() == 12 && pid.chars().all(|c| c.is_ascii_alphanumeric()) {
                                            any_product_id = pid.to_string();
                                            if let Some(platforms) = av.get("Platforms").and_then(|p| p.as_array()) {
                                                for p in platforms {
                                                    if let Some(p_str) = p.as_str() {
                                                        if p_str == "PC" || p_str == "Desktop" {
                                                            pc_product_id = pid.to_string();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            let final_pid = if !pc_product_id.is_empty() {
                                pc_product_id
                            } else if !any_product_id.is_empty() {
                                any_product_id
                            } else {
                                String::new()
                            };

                            if !final_pid.is_empty() {
                                if seen_ids.insert(final_pid.clone()) {
                                    store_big_ids.push(final_pid);
                                }
                            } else if !pfn.is_empty() || !modern_id.is_empty() {
                                let target_id = if !pfn.is_empty() { pfn.to_string() } else { modern_id.to_string() };
                                if seen_ids.insert(target_id.clone()) {
                                    let developer = t.get("detail")
                                        .and_then(|d| d.get("developerName").and_then(|dev| dev.as_str()))
                                        .or_else(|| t.get("detail").and_then(|d| d.get("publisherName").and_then(|p| p.as_str())))
                                        .unwrap_or("Xbox Game Studios")
                                        .to_string();

                                    let cover = t.get("displayImage").and_then(|x| x.as_str())
                                        .or_else(|| {
                                            t.get("images")
                                                .and_then(|arr| arr.as_array())
                                                .and_then(|a| a.first())
                                                .and_then(|img| img.get("url").and_then(|u| u.as_str()))
                                        })
                                        .map(|s| {
                                            if s.starts_with("//") { format!("https:{}", s) } else { s.to_string() }
                                        })
                                        .unwrap_or_else(|| DEFAULT_COVER_URL.to_string());

                                    direct_titles.push(GameCatalogItem {
                                        id: target_id.clone(),
                                        product_id: target_id.clone(),
                                        title: raw_name.to_string(),
                                        developer,
                                        license_type: "owned".to_string(),
                                        installed: false,
                                        size: "Standard".to_string(),
                                        path: format!("/mnt/w11/XboxGames/{}", target_id),
                                        cover,
                                        cloud_synced: true,
                                        last_played: "Licensed".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    // 2. Query Microsoft Collections API for Store BigIDs if accessible
    let collection_urls = [
        "https://collections.mp.microsoft.com/v8.0/collections/users/me/browse",
        "https://collections.mp.microsoft.com/v8.0/collections/browse",
    ];

    let store_auth = licensing_auth_header.unwrap_or(auth_header);
    for url in &collection_urls {
        let mut continuation_token: Option<String> = None;
        loop {
            let mut body_map = serde_json::json!({
                "market": "US",
                "locale": "en-US",
                "maxResults": 200,
                "validityType": "Valid",
                "entitlementFilters": [
                    "Game",
                    "Durable"
                ]
            });

            if let Some(token) = &continuation_token {
                if let Some(obj) = body_map.as_object_mut() {
                    obj.insert("continuationToken".to_string(), serde_json::json!(token));
                }
            }

            if let Ok(resp) = client
                .post(*url)
                .header("Authorization", store_auth)
                .header("Content-Type", "application/json")
                .json(&body_map)
                .send()
                .await
            {
                if resp.status().is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    let res: CollectionsBrowseResponse = serde_json::from_str(&text).unwrap_or_default();
                    for item in res.items {
                        if item.product_id.len() == 12 && item.product_id.chars().all(|c| c.is_ascii_alphanumeric()) {
                            if seen_ids.insert(item.product_id.clone()) {
                                store_big_ids.push(item.product_id);
                            }
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
            } else {
                break;
            }
        }
    }

    // 3. Batch-enrich all valid Store BigIDs via DisplayCatalog
    if !store_big_ids.is_empty() {
        let enriched = enrich_products_catalog(client, &store_big_ids).await;
        for mut item in enriched {
            item.license_type = "owned".to_string();
            catalog_items.push(item);
        }
    }

    // Append direct titles that had no 12-char ProductId
    catalog_items.extend(direct_titles);

    // 4. Clean and update SQLite user_entitlements
    if let Some(database) = db {
        let _ = database.clean_invalid_entitlements();
        let to_cache: Vec<_> = catalog_items.iter().map(|item| {
            crate::db::CachedEntitlement {
                xuid: "me".to_string(),
                product_id: item.product_id.clone(),
                sku_id: None,
                title: Some(item.title.clone()),
                entitlement_type: "owned".to_string(),
                acquired_date: None,
                updated_at: 0,
            }
        }).collect();
        let _ = database.replace_user_entitlements("me", &to_cache);
    }

    catalog_items
}

/// Query Microsoft Collections API with full pagination across all pages
pub async fn get_user_collections(
    client: &reqwest::Client,
    auth_header: &str,
    _xuid: Option<&str>,
) -> reqwest::Result<Vec<CollectionItem>> {
    let mut all_items = Vec::new();
    let mut seen_ids = HashSet::new();

    // Query Microsoft Collections browse endpoints with Valid filter
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
                "validityType": "Valid",
                "entitlementFilters": [
                    "Game",
                    "Durable"
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
        if !all_items.is_empty() {
            break;
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

    // Fallback: purchase.mp.microsoft.com/v8.0/users/me/entitlements
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
            #[derive(serde::Deserialize, Default)]
            struct PurchaseItem {
                #[serde(alias = "productId", default)]
                product_id: String,
                #[serde(alias = "entitlementType", alias = "productType", default)]
                product_type: Option<String>,
            }
            #[derive(serde::Deserialize, Default)]
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
                            ..Default::default()
                        });
                    }
                }
            }
        }
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
                        ..Default::default()
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
                    cover: cached.poster_url.unwrap_or_else(|| DEFAULT_COVER_URL.to_string()),
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
                // 1. Check Product Family / Type (must be a Game)
                let is_game = prod.product_type.as_deref().map(|t| t.eq_ignore_ascii_case("Game")).unwrap_or(false)
                    || prod.product_kind.as_deref().map(|k| k.eq_ignore_ascii_case("Game")).unwrap_or(false)
                    || prod.product_family_name.as_deref().map(|f| f.eq_ignore_ascii_case("Games")).unwrap_or(false);

                if !is_game {
                    continue;
                }

                // 2. Check PC Platform compatibility
                if let Some(ref platforms) = prod.allowed_platforms {
                    if !platforms.is_empty() && !platforms.iter().any(|p| {
                        let l = p.to_lowercase();
                        l.contains("windows") || l.contains("desktop") || l.contains("pc") || l.contains("win32") || l.contains("all")
                    }) {
                        // Console-only title without PC packages -> skip
                        continue;
                    }
                }

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
                            DEFAULT_COVER_URL.to_string()
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

/// Check if the user has an active PC Game Pass or Xbox Game Pass Ultimate subscription and determine tier
pub async fn check_user_gamepass_subscription(
    client: &reqwest::Client,
    auth_header: &str,
    xuid: Option<&str>,
) -> (bool, Option<String>) {
    let collections = get_user_collections(client, auth_header, xuid).await.unwrap_or_default();

    for item in &collections {
        let pid = item.product_id.to_uppercase();
        if pid == "CFQ7TTC0KHS0" {
            return (true, Some("Ultimate".to_string()));
        } else if pid == "CFQ7TTC0KGQ8" || pid == "9P1N75Q4K9Q8" || pid == "CFQ7TTC0K5DH" {
            return (true, Some("PC Game Pass".to_string()));
        } else if pid == "CFQ7TTC0P85B" {
            return (true, Some("Premium".to_string()));
        } else if pid == "CFQ7TTC0K5DJ" {
            return (true, Some("Standard".to_string()));
        } else if pid == "CFQ7TTC0K6L8" {
            return (true, Some("Core".to_string()));
        }
    }

    for item in &collections {
        if let Some(ref ptype) = item.product_type {
            if ptype.eq_ignore_ascii_case("Pass") || ptype.eq_ignore_ascii_case("Subscription") {
                return (true, Some("Game Pass".to_string()));
            }
        }
    }

    (false, None)
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

    #[tokio::test]
    async fn test_inspect_user_collections() {
        use crate::models::secrets::Token;
        crate::secrets::init_secrets().ok();
        let client = reqwest::Client::new();
        let tokens = crate::tokens::TokenManager::with_keychain_and_memory();
        
        let Token::Legacy(dev_token) = tokens.get_device_sts_token().unwrap() else { panic!() };
        let Token::Legacy(user_token) = tokens.get_user_sts_token().unwrap() else { panic!() };
        let user = tokens.get_user().unwrap();

        let coll_outcome = crate::api::live::exchange_user_token(
            &client,
            user_token.clone(),
            user.username.clone(),
            dev_token.clone(),
            None,
            Some("Silent".to_string()),
            "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
            &[(
                "scope=service::collections.mp.microsoft.com::MBI_SSL".to_string(),
                Some(crate::models::soap::PolicyReference::token_broker()),
            )],
        )
        .await
        .unwrap();

        let Token::Compact(coll_token) = (match coll_outcome {
            crate::models::live::ExchangeUserTokenOutcome::Issued(
                crate::models::soap::BodyContent::RequestSecurityTokenResponseCollection(mut c),
            ) => Token::from(c.security_tokens.remove(0)),
            crate::models::live::ExchangeUserTokenOutcome::Issued(
                crate::models::soap::BodyContent::RequestSecurityTokenResponse(t),
            ) => Token::from(*t),
            _ => panic!(),
        }) else { panic!() };

        let xbl_xsts = crate::api::xbox::get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await.unwrap();
        let xuid = xbl_xsts.xuid().map(|x| x.to_string()).unwrap_or_default();

        println!("User PUID: {}, XUID: {}", user.puid, xuid);

        let ep = "https://collections.mp.microsoft.com/v8.0/collections/b2bLicensePreview";

        let mut all_pids = Vec::new();
        let mut seen = HashSet::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut body = serde_json::json!({
                "market": "US",
                "locale": "en-US",
                "maxResults": 200,
                "beneficiaries": [
                    {
                        "identityType": "msa",
                        "identityValue": coll_token,
                        "localTicketReference": user.puid
                    }
                ]
            });

            if let Some(token) = &continuation_token {
                body.as_object_mut().unwrap().insert("continuationToken".to_string(), serde_json::json!(token));
            }

            let resp = client.post(ep).header("Authorization", format!("{coll_token}")).header("Content-Type", "application/json").json(&body).send().await.unwrap();
            let tx = resp.text().await.unwrap_or_default();
            let val: serde_json::Value = serde_json::from_str(&tx).unwrap();

            if let Some(items) = val.get("items").and_then(|i| i.as_array()) {
                for it in items {
                    let pid = it.get("productId").and_then(|p| p.as_str()).unwrap_or("");
                    let is_trial = it.get("isTrial").and_then(|t| t.as_bool()).unwrap_or(false);
                    if !is_trial && pid.len() == 12 && pid.chars().all(|c| c.is_ascii_alphanumeric()) {
                        if seen.insert(pid.to_string()) {
                            all_pids.push(pid.to_string());
                        }
                    }
                }
            }

            let next_token = val.get("continuationToken").and_then(|t| t.as_str()).map(|s| s.to_string());
            if next_token.is_some() && next_token != continuation_token {
                continuation_token = next_token;
            } else {
                break;
            }
        }

        println!("=== TOTAL UNIQUE OWNED PRODUCT IDS ACROSS ALL PAGES: {} ===", all_pids.len());

        let enriched = crate::api::xbox::enrich_products_catalog(&client, &all_pids).await;
        println!("TOTAL RESOLVED USER OWNED PC GAMES: {}", enriched.len());

        for (i, item) in enriched.iter().enumerate() {
            println!("[{i}] {:<40} | ProductId: {:<12} | Dev: {:<20}", item.title, item.product_id, item.developer);
        }
    }
}
