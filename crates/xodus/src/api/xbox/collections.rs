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
#[serde(rename_all = "PascalCase")]
pub struct DisplayCatalogResponse {
    #[serde(default)]
    pub products: Vec<DisplayProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DisplayProduct {
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub product_type: Option<String>,
    #[serde(default)]
    pub product_kind: Option<String>,
    #[serde(default)]
    pub product_family_name: Option<String>,
    #[serde(default)]
    pub localized_properties: Vec<LocalizedProperty>,
    #[serde(default)]
    pub allowed_platforms: Option<Vec<String>>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    #[serde(default)]
    pub display_sku_availabilities: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct LocalizedProperty {
    #[serde(default)]
    pub product_title: String,
    #[serde(default)]
    pub developer_name: String,
    #[serde(default)]
    pub publisher_name: String,
    #[serde(default)]
    pub images: Vec<ProductImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ProductImage {
    #[serde(default)]
    pub image_purpose: String,
    #[serde(default)]
    pub uri: String,
}

/// Query Microsoft Collections b2bLicensePreview API with full metadata and resolve genuine user-owned PC games
pub async fn get_user_owned_catalog_items(
    client: &reqwest::Client,
    tokens: Option<&crate::tokens::TokenManager>,
    auth_header: &str,
    _licensing_auth_header: Option<&str>,
    xuid: Option<&str>,
    db: Option<&crate::db::Database>,
) -> Vec<GameCatalogItem> {
    if let Some(ref database) = db {
        let _ = database.clean_invalid_entitlements();
    }

    let mut store_big_ids = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Primary & Authentic Source: Microsoft Store Collections b2bLicensePreview API
    let mut b2b_succeeded = false;
    if let Some(mgr) = tokens {
        if let (Ok(dev_token), Ok(user_token), Ok(user)) = (mgr.get_device_sts_token(), mgr.get_user_sts_token(), mgr.get_user()) {
            if let (crate::models::secrets::Token::Legacy(dev_token), crate::models::secrets::Token::Legacy(user_token)) = (dev_token, user_token) {
                let coll_outcome = crate::api::live::exchange_user_token(
                    client,
                    user_token,
                    user.username,
                    dev_token,
                    None,
                    Some("Silent".to_string()),
                    "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
                    &[(
                        "scope=service::collections.mp.microsoft.com::MBI_SSL".to_string(),
                        Some(crate::models::soap::PolicyReference::token_broker()),
                    )],
                )
                .await;

                if let Ok(crate::models::live::ExchangeUserTokenOutcome::Issued(body_content)) = coll_outcome {
                    let coll_token: Option<String> = match body_content {
                        crate::models::soap::BodyContent::RequestSecurityTokenResponseCollection(mut c) => {
                            let tok: crate::models::secrets::Token = c.security_tokens.remove(0).into();
                            match tok {
                                crate::models::secrets::Token::Compact(s) => Some(s),
                                _ => None,
                            }
                        }
                        crate::models::soap::BodyContent::RequestSecurityTokenResponse(t) => {
                            let tok: crate::models::secrets::Token = (*t).into();
                            match tok {
                                crate::models::secrets::Token::Compact(s) => Some(s),
                                _ => None,
                            }
                        }
                        _ => None,
                    };

                    if let Some(compact_token) = coll_token {
                        let ep = "https://collections.mp.microsoft.com/v8.0/collections/b2bLicensePreview";
                        let mut continuation_token: Option<String> = None;

                        loop {
                            let mut body = serde_json::json!({
                                "market": "US",
                                "locale": "en-US",
                                "maxResults": 200,
                                "beneficiaries": [
                                    {
                                        "identityType": "msa",
                                        "identityValue": compact_token,
                                        "localTicketReference": user.puid
                                    }
                                ]
                            });

                            if let Some(token) = &continuation_token {
                                if let Some(obj) = body.as_object_mut() {
                                    obj.insert("continuationToken".to_string(), serde_json::json!(token));
                                }
                            }

                            if let Ok(resp) = client
                                .post(ep)
                                .header("Authorization", format!("{compact_token}"))
                                .header("Content-Type", "application/json")
                                .json(&body)
                                .send()
                                .await
                            {
                                if resp.status().is_success() {
                                    let tx = resp.text().await.unwrap_or_default();
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tx) {
                                        if let Some(items) = val.get("items").and_then(|i| i.as_array()) {
                                            for it in items {
                                                let pid = it.get("productId").and_then(|p| p.as_str()).unwrap_or("");
                                                let is_trial = it.get("isTrial").and_then(|t| t.as_bool()).unwrap_or(false);
                                                if !is_trial && pid.len() == 12 && pid.chars().all(|c| c.is_ascii_alphanumeric()) {
                                                    if seen_ids.insert(pid.to_string()) {
                                                        store_big_ids.push(pid.to_string());
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

                        if !store_big_ids.is_empty() {
                            b2b_succeeded = true;
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback: Query TitleHub with STRICT PC-only platform constraints
    if !b2b_succeeded && store_big_ids.is_empty() {
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

                                let mut has_pc_platform = devices.iter().any(|d| d == "PC");
                                let mut pc_product_id = String::new();

                                if let Some(detail) = t.get("detail") {
                                    if let Some(attrs) = detail.get("attributes").and_then(|a| a.as_array()) {
                                        for attr in attrs {
                                            if let Some(aname) = attr.get("name").and_then(|n| n.as_str()) {
                                                if aname == "XPA" {
                                                    has_pc_platform = true;
                                                }
                                            }
                                        }
                                    }
                                    if let Some(availabilities) = detail.get("availabilities").and_then(|a| a.as_array()) {
                                        for av in availabilities {
                                            let pid = av.get("ProductId").and_then(|p| p.as_str()).unwrap_or("");
                                            if pid.len() == 12 && pid.chars().all(|c| c.is_ascii_alphanumeric()) {
                                                if let Some(platforms) = av.get("Platforms").and_then(|p| p.as_array()) {
                                                    for p in platforms {
                                                        if let Some(p_str) = p.as_str() {
                                                            if p_str == "PC" || p_str == "Desktop" {
                                                                has_pc_platform = true;
                                                                pc_product_id = pid.to_string();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // STRICT RULE: Must be playable on PC. Console-only games are never added.
                                if has_pc_platform && !pc_product_id.is_empty() {
                                    if seen_ids.insert(pc_product_id.clone()) {
                                        store_big_ids.push(pc_product_id);
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    // 3. Batch Enrich through Display Catalog and cache verified PC games in SQLite
    let enriched = enrich_products_catalog(client, &store_big_ids).await;

    if let Some(ref database) = db {
        if let Some(id) = xuid {
            let mut db_entitlements = Vec::new();
            for item in &enriched {
                db_entitlements.push(crate::db::CachedEntitlement {
                    xuid: id.to_string(),
                    product_id: item.product_id.clone(),
                    sku_id: None,
                    title: Some(item.title.clone()),
                    entitlement_type: "owned".to_string(),
                    acquired_date: None,
                    updated_at: 0,
                });
            }
            if !db_entitlements.is_empty() {
                let _ = database.replace_user_entitlements(id, &db_entitlements);
            }
        }
    }

    enriched
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
                // 1. Check Product Family / Type (must be a Game, never an Application / App / Consumable)
                let ptype = prod.product_type.as_deref().unwrap_or("");
                let pkind = prod.product_kind.as_deref().unwrap_or("");
                let pfamily = prod.product_family_name.as_deref().unwrap_or("");

                if ptype.eq_ignore_ascii_case("Application") || pkind.eq_ignore_ascii_case("Application") || pfamily.eq_ignore_ascii_case("Apps") {
                    continue;
                }
                if ptype.eq_ignore_ascii_case("Consumable") || pkind.eq_ignore_ascii_case("Consumable") {
                    continue;
                }

                let is_game = ptype.eq_ignore_ascii_case("Game")
                    || pkind.eq_ignore_ascii_case("Game")
                    || pfamily.eq_ignore_ascii_case("Games");

                if !is_game {
                    continue;
                }

                // 2. Check PC Platform compatibility
                let is_xpa = prod.properties.as_ref().map(|props| {
                    props.get("XboxPlayAnywhere").and_then(|x| x.as_bool()).unwrap_or(false)
                    || props.get("Attributes").and_then(|a| a.as_array()).map(|attrs| {
                        attrs.iter().any(|attr| {
                            attr.get("Name").and_then(|n| n.as_str()).map(|n| n.eq_ignore_ascii_case("XPA")).unwrap_or(false)
                        })
                    }).unwrap_or(false)
                }).unwrap_or(false);

                let mut has_pc_package = false;
                let mut has_xbox_package = false;

                if let Some(skus) = &prod.display_sku_availabilities {
                    for sku_wrap in skus {
                        if let Some(packages) = sku_wrap.get("Sku").and_then(|s| s.get("Properties")).and_then(|p| p.get("Packages")).and_then(|pk| pk.as_array()) {
                            for pkg in packages {
                                if let Some(plats) = pkg.get("PlatformDependencies").and_then(|pd| pd.as_array()) {
                                    for pd in plats {
                                        if let Some(pname) = pd.get("PlatformName").and_then(|pn| pn.as_str()) {
                                            let l = pname.to_lowercase();
                                            if (l.contains("desktop") || l.contains("universal") || l == "pc" || l == "windows") && !l.contains("xbox") {
                                                has_pc_package = true;
                                            } else if l.contains("xbox") {
                                                has_xbox_package = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let has_pc_allowed_platforms = prod.allowed_platforms.as_ref().map(|plats| {
                    plats.iter().any(|p| {
                        let l = p.to_lowercase();
                        (l.contains("desktop") || l.contains("pc") || l.contains("win32") || l.contains("universal")) && !l.contains("xbox")
                    })
                }).unwrap_or(false);

                if has_xbox_package && !has_pc_package && !is_xpa {
                    // Strictly Xbox console only title
                    continue;
                }

                if !has_pc_package && !is_xpa && !has_pc_allowed_platforms {
                    // No PC package, not XPA, and no PC platform listed -> console only
                    continue;
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
        
        let xbl_xsts = crate::api::xbox::get_or_request_xsts(&client, &tokens, "http://xboxlive.com").await.unwrap();
        let xuid = xbl_xsts.xuid().map(|x| x.to_string()).unwrap_or_default();
        let auth_header = format!("XBL3.0 x={};{}", xbl_xsts.user_hash().unwrap_or_default(), xbl_xsts.token);

        let db = crate::db::Database::open_default().ok();
        let owned = get_user_owned_catalog_items(&client, Some(&tokens), &auth_header, None, Some(&xuid), db.as_ref()).await;
        println!("User has {} owned games:", owned.len());
        for g in &owned {
            println!("  Owned: id='{}', title='{}', path='{}'", g.id, g.title, g.path);
        }

        println!("\n=== TESTING TITLE MGT ENDPOINTS FOR 1717113201 ===");
        let mgt_url = "https://title.mgt.xboxlive.com/titles/1717113201/endpoints?type=1";
        if let Ok(resp) = client.get(mgt_url).send().await {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            println!("Title Mgt (1717113201) -> Status: {status}\nBody: {text}");
        }

        let mgt_url2 = "https://title.mgt.xboxlive.com/titles/default/endpoints?type=1";
        if let Ok(resp) = client.get(mgt_url2).send().await {
            let mgt: crate::models::xbox::TitleMgtResponse = resp.json().await.unwrap_or_else(|_| serde_json::from_str("{}").unwrap());
            println!("Title Mgt has {} endpoints:", mgt.end_points.len());
            for ep in &mgt.end_points {
                if ep.host.contains("rare") || ep.host.contains("athena") || ep.host.contains("discovery") || ep.relying_party.as_deref().map(|r| r.contains("athena") || r.contains("rare")).unwrap_or(false) {
                    println!("  Found Matching Endpoint: host={}, rp={:?}, token_type={:?}", ep.host, ep.relying_party, ep.token_type);
                }
            }
        }

        let Token::Legacy(dev_token) = tokens.get_device_sts_token().unwrap() else { panic!() };
        let Token::Legacy(user_token) = tokens.get_user_sts_token().unwrap() else { panic!() };
        let mut auth = xal::XalAuthenticator::new(
            xal::XalAppParameters {
                client_id: "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
                title_id: Some("1717113201".to_string()),
                auth_scopes: vec![],
                redirect_uri: None,
                client_secret: None,
            },
            xal::client_params::CLIENT_WINDOWS(),
            "RETAIL".to_string(),
        );

        let device_token_resp = crate::api::live::exchange_device_token(
            &client,
            dev_token.clone(),
            "{28C08266-F973-4AE6-FFE4-409B249F138F}".to_string(),
            "scope=service::user.auth.xboxlive.com::MBI_SSL&api-version=2.0".to_owned(),
            Some(crate::models::soap::PolicyReference::token_broker()),
        ).await.unwrap();
        let Token::Compact(compact_dev) = device_token_resp.into() else { panic!() };
        let dt = auth.get_device_token_rps(compact_dev).await.unwrap();
        let title_tok = auth.get_title_token_win(&dt.token, 1717113201).await.unwrap();

        let user_token_resp = crate::api::live::exchange_user_token(
            &client,
            user_token.clone(),
            "USERNAME".to_string(),
            dev_token.clone(),
            None,
            Some("Silent".to_string()),
            "{d6d5a677-0872-4ab0-9442-bb792fce85c5}".to_string(),
            &[(
                "user.auth.xboxlive.com".to_owned(),
                Some(crate::models::soap::PolicyReference::mbi_ssl()),
            )],
        ).await.unwrap();
        let user_token_tok: Token = match user_token_resp {
            crate::models::live::ExchangeUserTokenOutcome::Issued(
                crate::models::soap::BodyContent::RequestSecurityTokenResponseCollection(mut col)
            ) => col.security_tokens.remove(0).into(),
            crate::models::live::ExchangeUserTokenOutcome::Issued(
                crate::models::soap::BodyContent::RequestSecurityTokenResponse(token)
            ) => (*token).into(),
            _ => panic!(),
        };
        let Token::Compact(user_token_compact) = user_token_tok else { panic!() };
        let user_xbl = crate::api::xbox::authenticate_xbox_user(&client, user_token_compact).await.unwrap();
        let xal_user = xal::response::UserToken {
            issue_instant: chrono::Utc::now(),
            not_after: chrono::Utc::now() + chrono::Duration::hours(24),
            token: user_xbl.token.clone(),
            display_claims: None,
        };

        let xsts_token = auth.get_xsts_token(
            Some(&dt),
            Some(&title_tok),
            Some(&xal_user),
            "http://xboxlive.com",
        ).await.unwrap();

        let auth_header = xsts_token.authorization_header_value();
        println!("Full XSTS Auth Header: {}", auth_header);

        let signer = auth.request_signer();
        use p256::ecdsa::SigningKey;
        use p256::ecdsa::signature::hazmat::PrehashSigner;
        use base64::Engine;

        let signing_policy_version: i32 = 1;
        let version_bytes = signing_policy_version.to_be_bytes();
        let now = chrono::Utc::now();
        let filetime_val = (now.timestamp() + 11644473600) * 10000000 + (now.timestamp_subsec_nanos() as i64 / 100);
        let filetime_bytes = filetime_val.to_be_bytes();
        let path_and_query = "/discovery/app/endpoint?tid=1717113201";

        let prehash = xal::RequestSigner::prehash_message_data(
            &version_bytes,
            &filetime_bytes,
            "GET",
            path_and_query,
            &auth_header,
            &[],
            0,
        );

        let signing_key: SigningKey = signer.keypair.clone().into();
        let signature: p256::ecdsa::Signature = signing_key.sign_prehash(&prehash).unwrap();

        let mut sig_bytes = Vec::new();
        sig_bytes.extend_from_slice(&version_bytes);
        sig_bytes.extend_from_slice(&filetime_bytes);
        sig_bytes.extend_from_slice(&signature.to_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&sig_bytes);

        println!("\n=== TESTING rp://athena.prod.msrareservices.com/ ===");
        match auth.get_xsts_token(Some(&dt), Some(&title_tok), Some(&xal_user), "rp://athena.prod.msrareservices.com/").await {
            Ok(tok) => {
                println!("  SUCCESS: got XSTS token for rp://athena.prod.msrareservices.com/ (len={})", tok.token.len());
                let auth_header = tok.authorization_header_value();
                println!("  Auth Header: {}", auth_header);

                let athena_url = "https://discovery.prod.athena.msrareservices.com/discovery/app/endpoint?tid=1717113201";
                let resp = client.get(athena_url)
                    .header("Authorization", &auth_header)
                    .header("User-Agent", "Athena/2.150.9409.0 (WinGDK; Windows 10.0.19045.0)")
                    .send()
                    .await;
                if let Ok(r) = resp {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    println!("Athena with rp://athena.prod.msrareservices.com/ -> Status: {status}\nBody: {body}");
                }

                println!("\n=== TESTING TITLE AUTH PAYLOADS ===");
                let title_auth_url = "https://title.auth.xboxlive.com/title/authenticate";
                
                for test_body in [
                    serde_json::json!({
                        "RelyingParty": "http://auth.xboxlive.com",
                        "TokenType": "JWT",
                        "Properties": {
                            "AuthMethod": "RPS",
                            "SiteName": "user.auth.xboxlive.com",
                            "RpsTicket": format!("t={}", user_token.token),
                            "DeviceToken": dt.token,
                        }
                    }),
                    serde_json::json!({
                        "RelyingParty": "http://auth.xboxlive.com",
                        "TokenType": "JWT",
                        "Properties": {
                            "ProofKey": auth.request_signer().get_proof_key(),
                            "DeviceToken": dt.token,
                            "TitleId": 1717113201,
                            "TitleVersion": "2.150.9409.0",
                        }
                    }),
                    serde_json::json!({
                        "RelyingParty": "http://auth.xboxlive.com",
                        "TokenType": "JWT",
                        "Properties": {
                            "ProofKey": auth.request_signer().get_proof_key(),
                            "DeviceToken": dt.token,
                            "TitleId": 1717113201,
                            "Version": "2.150.9409.0",
                        }
                    }),
                ] {
                    let resp = client.post(title_auth_url)
                        .header("x-xbl-contract-version", "1")
                        .json(&test_body)
                        .send()
                        .await;
                    if let Ok(r) = resp {
                        let status = r.status();
                        let body = r.text().await.unwrap_or_default();
                        println!("Title Auth -> Status: {status}\nBody: {body}");
                    }
                }
            }
            Err(e) => println!("  FAILED for rp://athena.prod.msrareservices.com/: {e}"),
        }
    }
}
