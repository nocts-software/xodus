use inquire::Select;
use xodus::XBOX_LIVE_PACKAGES_PC;
use xodus::api::displaycatalog::find_products_by_id;
use xodus::models::packagespc::{PackageDetails, PackageResponse};
use xodus::models::secrets::Token;
use xodus::tokens::TokenManager;

pub async fn get_content_id(
    client: &reqwest::Client,
    product: String,
    market: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let displaycatalog = find_products_by_id(
        client,
        product.clone(),
        market.clone().unwrap_or("neutral".to_owned()),
        vec!["en".to_string(), "neutral".to_string()],
    )
    .await?;

    let product_details = displaycatalog.product;

    let mut found_package = None;
    let mut subprods: Vec<String> = vec![];
    'o: for availability in &product_details.display_sku_availabilities {
        for package in &availability.sku.properties.packages {
            if package
                .platform_dependencies
                .iter()
                .any(|dep| dep.platform_name == "Windows.Desktop" || dep.platform_name == "Universal")
            {
                found_package = Some(package);
                break 'o;
            }
        }
        for bundled in &availability.sku.properties.bundled_skus {
            if !bundled.big_id.is_empty() && bundled.big_id != product {
                subprods.push(bundled.big_id.clone());
            }
        }
        for availability in &availability.availabilities {
            if let Some(licensing_data) = &availability.licensing_data {
                for satisfies in &licensing_data.satisfying_entitlement_keys {
                    for entitlement_key in &satisfies.entitlement_keys {
                        let key: Vec<&str> = entitlement_key.split(":").collect();
                        if key.len() == 3 && key[0] == "big" && key[1] != product {
                            subprods.push(key[1].to_string());
                        }
                    }
                }
            }
        }
    }
    subprods.retain(|s| s != &product);
    subprods.sort();
    subprods.dedup();

    let Some(package) = found_package else {
        if !subprods.is_empty() {
            // First check if any subprod has a Windows.Desktop package automatically
            for sub in &subprods {
                if let Ok(sub_cat) = find_products_by_id(
                    client,
                    sub.clone(),
                    market.clone().unwrap_or("neutral".to_owned()),
                    vec!["en".to_string(), "neutral".to_string()],
                ).await {
                    for avail in &sub_cat.product.display_sku_availabilities {
                        for pkg in &avail.sku.properties.packages {
                            if pkg.platform_dependencies.iter().any(|dep| dep.platform_name == "Windows.Desktop") {
                                if let Some(cid) = &pkg.content_id {
                                    return Ok(cid.clone());
                                }
                            }
                        }
                    }
                }
            }

            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                if let Ok(item) = Select::new("Select files to download", subprods.clone())
                    .with_page_size(30)
                    .prompt()
                {
                    return Box::pin(get_content_id(client, item, market)).await;
                }
            }

            // Fallback for non-interactive / GUI callers: try the first sub-product
            if let Some(first_sub) = subprods.first() {
                return Box::pin(get_content_id(client, first_sub.clone(), market)).await;
            }
        }

        return Err(Box::new(std::io::Error::other(
            "Windows.Desktop package not found, if you believe this is an error, please report it",
        )));
    };

    let Some(content_id) = &package.content_id else {
        log::error!("ContentId not found, if you believe this is an error, please report it");
        return Err(Box::new(std::io::Error::other(
            "ContentId not found, if you believe this is an error, please report it",
        )));
    };
    Ok(content_id.to_owned())
}

pub async fn get_packages(
    client: &reqwest::Client,
    tokens: &TokenManager,
    content_id: String,
) -> Result<PackageDetails, Box<dyn std::error::Error>> {
    let dev_token = tokens.get_device_sts_token().unwrap();
    let Token::Legacy(dev_token) = dev_token else {
        return Err(Box::new(std::io::Error::other("Invalid STS token")));
    };
    let user_token = tokens.get_user_sts_token().unwrap();
    let Token::Legacy(legacy) = user_token else {
        return Err(Box::new(std::io::Error::other("Unsupported user token")));
    };

    let xsts_token = xodus::api::xbox::run(client, dev_token, legacy, "http://update.xboxlive.com")
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let response = client
        .get(format!(
            "{XBOX_LIVE_PACKAGES_PC}/GetBasePackage/{content_id}"
        ))
        .header("x-xbl-contract-version", "3")
        .header(
            "Authorization",
            xodus::api::xbox::get_xsts_auth_header(xsts_token),
        )
        .send()
        .await?;

    let res: PackageResponse = response.json().await?;

    let PackageResponse::Found(package) = res else {
        return Err(Box::new(std::io::Error::other(
            "Package was not found, is it owned by the user?",
        )));
    };
    Ok(package)
}
