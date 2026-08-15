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
    pub id: String,
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
pub async fn get_user_collections(
    client: &reqwest::Client,
    auth_header: &str,
) -> reqwest::Result<Vec<CollectionItem>> {
    let url = "https://collections.mp.microsoft.com/v8.0/collections/browse";
    let mut all_items = Vec::new();
    let mut continuation_token: Option<String> = None;
    let mut seen_ids = HashSet::new();

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
            .post(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&body_map)
            .send()
            .await?;

        if resp.status().is_success() {
            let res: CollectionsBrowseResponse = resp.json().await.unwrap_or_default();
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
        return Ok(all_items);
    }

    // Fallback to inventory.xboxlive.com with pagination
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

        if inv_resp.status().is_success() {
            let inv: InventoryResponse = inv_resp.json().await.unwrap_or_default();
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
pub async fn enrich_products_catalog(
    client: &reqwest::Client,
    product_ids: &[String],
) -> Vec<GameCatalogItem> {
    let mut catalog_items = Vec::new();

    for chunk in product_ids.chunks(20) {
        let big_ids = chunk.join(",");
        let url = format!(
            "https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={big_ids}&market=US&languages=en-us"
        );

        if let Ok(resp) = client.get(&url).header("MS-CV", "0.1").send().await {
            if resp.status().is_success() {
                let dcat: DisplayCatalogResponse = resp.json().await.unwrap_or_default();
                for prod in dcat.products {
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
                                "https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=600&auto=format&fit=crop&q=80".to_string()
                            });

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
    }

    catalog_items
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
