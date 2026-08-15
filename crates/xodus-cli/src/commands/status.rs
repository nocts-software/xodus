use std::process::ExitCode;
use xodus::tokens::TokenManager;

pub async fn run(client: &reqwest::Client, tokens: &TokenManager) -> ExitCode {
    println!("=== Xodus Authentication & Xbox Live Status ===");

    // 1. Device License
    match tokens.get_device_license() {
        Ok(dev) => println!("• Device ID: {}", dev.device_id),
        Err(_) => println!("• Device License: Not registered (run login)"),
    }


    // 2. User info
    match tokens.get_user() {
        Ok(user) => {
            println!("• Microsoft Account: {}", user.username);
            println!("• PUID: {}", user.puid);

            // 3. Query Xbox Live XSTS and Profile
            match xodus::api::xbox::get_or_request_xsts(client, tokens, "http://xboxlive.com").await {
                Ok(xsts) => {
                    let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                    println!("• Xbox Live XSTS: Authenticated (XBL3.0)");

                    // Profile
                    match xodus::api::xbox::get_user_profile(client, &auth_header).await {
                        Ok(Some(profile)) => {
                            println!("• Gamertag: {}", profile.gamertag);
                            println!("• Avatar URL: {}", profile.display_pic);
                            println!("• Gamerscore: {}", profile.gamerscore);
                            println!("• Account Tier: {}", profile.tier);
                        }
                        Ok(None) => println!("• Xbox Live Profile: No profile data returned"),
                        Err(e) => println!("• Xbox Live Profile Error: {e}"),
                    }

                    // Friends
                    let social = xodus::api::xbox::SocialClient::new(client);
                    match social.get_friends(&auth_header).await {
                        Ok(friends) => {
                            println!("\n=== Xbox Live Friends ({}) ===", friends.len());
                            for f in friends {
                                let state = f.presence_state.as_deref().unwrap_or("Offline");
                                let text = f.presence_text.as_deref().unwrap_or("Offline");
                                println!("  - {} [{}]: {}", f.gamertag, state, text);
                            }
                        }
                        Err(e) => println!("• Friends query failed: {e}"),
                    }

                    // Collections / Entitlements
                    match xodus::api::xbox::get_user_collections(client, &auth_header).await {
                        Ok(items) => {
                            println!("\n=== Microsoft Store Entitlements ({}) ===", items.len());
                            for item in items.iter().take(10) {
                                println!("  - Product ID: {} (Type: {:?})", item.product_id, item.product_type);
                            }
                            if items.len() > 10 {
                                println!("  ... and {} more licensed titles", items.len() - 10);
                            }
                        }
                        Err(e) => println!("• Collections query failed: {e}"),
                    }
                }
                Err(e) => {
                    println!("• Xbox Live XSTS Token Request Failed: {e}");
                    println!("  Please run 'xodus login' to sign in with your Microsoft account.");
                }
            }
        }
        Err(_) => {
            println!("• Microsoft Account: Not signed in.");
            println!("  Please run 'xodus login' to authenticate with your Xbox / Microsoft credentials.");
        }
    }

    ExitCode::SUCCESS
}
