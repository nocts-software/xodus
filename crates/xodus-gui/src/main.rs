use std::sync::Arc;
use tao::{
    dpi::{LogicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};
use wry::WebViewBuilder;
use xodus::tokens::TokenManager;

const HTML: &str = include_str!("../ui/index.html");
const CSS: &str = include_str!("../ui/styles.css");
const JS: &str = include_str!("../ui/app.js");

#[derive(Debug)]
enum CustomEvent {
    EvaluateScript(String),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env("XODUS_LOG");
    xodus::secrets::init_secrets().ok();

    let tokens = Arc::new(TokenManager::with_keychain_and_memory());
    let mut event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    
    let window = WindowBuilder::new()
        .with_title("noct's xodus gui")
        .with_decorations(false)
        .with_resizable(true)
        .with_inner_size(Size::Logical(LogicalSize::new(1280.0, 800.0)))
        .with_min_inner_size(Size::Logical(LogicalSize::new(960.0, 600.0)))
        .build(&event_loop)?;

    let window = Arc::new(window);
    let win_ipc = window.clone();

    let tokens_clone = tokens.clone();
    let combined_html = HTML
        .replace("<link rel=\"stylesheet\" href=\"styles.css\">", &format!("<style>{}</style>", CSS))
        .replace("<script src=\"app.js\"></script>", &format!("<script>{}</script>", JS));

    let rt = tokio::runtime::Runtime::new()?;

    let proxy_ipc = proxy.clone();
    let tokens_ipc = tokens.clone();

    let builder = WebViewBuilder::new()
        .with_html(&combined_html)
        .with_ipc_handler(move |req| {
            let body = req.body();
            log::info!("IPC Message: {body}");
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some(cmd) = v.get("cmd").and_then(|c| c.as_str()) {
                    match cmd {
                        "drag_window" => {
                            let _ = win_ipc.drag_window();
                        }
                        "minimize" => {
                            win_ipc.set_minimized(true);
                        }
                        "maximize" => {
                            let is_max = win_ipc.is_maximized();
                            win_ipc.set_maximized(!is_max);
                        }
                        "close" => {
                            std::process::exit(0);
                        }
                        "launch_game" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                rt.spawn(async move {
                                    let child = tokio::process::Command::new("xodus")
                                        .arg("run")
                                        .arg(&path_owned)
                                        .spawn();
                                    if let Ok(mut c) = child {
                                        let _ = c.wait().await;
                                    }
                                });
                            }
                        }
                        "sync_saves" | "pull_save" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                rt.spawn(async move {
                                    let _ = tokio::process::Command::new("xodus")
                                        .arg("save")
                                        .arg("pull")
                                        .arg(&path_owned)
                                        .status()
                                        .await;
                                });
                            }
                        }
                        "push_save" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                let path_owned = path.to_string();
                                rt.spawn(async move {
                                    let _ = tokio::process::Command::new("xodus")
                                        .arg("save")
                                        .arg("push")
                                        .arg(&path_owned)
                                        .status()
                                        .await;
                                });
                            }
                        }
                        "sync_licenses" => {
                            log::info!("Querying Microsoft Account digital licenses and entitlements...");
                            let tokens_clone = tokens_ipc.clone();
                            let proxy_tokio = proxy_ipc.clone();
                            rt.spawn(async move {
                                let client = reqwest::Client::new();
                                if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens_clone, "http://xboxlive.com").await {
                                    let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                                    if let Ok(Some(profile)) = xodus::api::xbox::get_user_profile(&client, &auth_header).await {
                                        if let Ok(json_str) = serde_json::to_string(&profile) {
                                            let script = format!("if (window.setUserData) window.setUserData({json_str});");
                                            let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                        }
                                    }
                                    if let Ok(friends) = xodus::api::xbox::SocialClient::new(&client).get_friends(&auth_header).await {
                                        if !friends.is_empty() {
                                            if let Ok(json_str) = serde_json::to_string(&friends) {
                                                let script = format!("if (window.setFriendsData) window.setFriendsData({json_str});");
                                                let _ = proxy_tokio.send_event(CustomEvent::EvaluateScript(script));
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        "set_presence" => {
                            if let Some(st) = v.get("state").and_then(|s| s.as_str()) {
                                let st_owned = st.to_string();
                                let tokens_clone = tokens_ipc.clone();
                                rt.spawn(async move {
                                    let client = reqwest::Client::new();
                                    if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens_clone, "http://xboxlive.com").await {
                                        let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                                        let _ = xodus::api::xbox::SocialClient::new(&client).set_presence(&auth_header, &st_owned).await;
                                    }
                                });
                            }
                        }
                        "install_game" => {
                            if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
                                log::info!("Preparing package install & decryption for: {path}");
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

    #[cfg(target_os = "linux")]
    let webview = {
        use gtk::prelude::WidgetExt;
        use gtk::prelude::GtkWindowExt;
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get window default vbox");
        let wv = builder.build_gtk(vbox)?;
        window.gtk_window().show_all();
        window.gtk_window().present();
        wv
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    window.set_focus();

    // Trigger initial background sync
    {
        let proxy_init = proxy.clone();
        let tokens_init = tokens_clone.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let client = reqwest::Client::new();
                if let Ok(xsts) = xodus::api::xbox::get_or_request_xsts(&client, &tokens_init, "http://xboxlive.com").await {
                    let auth_header = xodus::api::xbox::get_xsts_auth_header(xsts);
                    if let Ok(Some(profile)) = xodus::api::xbox::get_user_profile(&client, &auth_header).await {
                        if let Ok(json_str) = serde_json::to_string(&profile) {
                            let script = format!("if (window.setUserData) window.setUserData({json_str});");
                            let _ = proxy_init.send_event(CustomEvent::EvaluateScript(script));
                        }
                    }
                    if let Ok(friends) = xodus::api::xbox::SocialClient::new(&client).get_friends(&auth_header).await {
                        if !friends.is_empty() {
                            if let Ok(json_str) = serde_json::to_string(&friends) {
                                let script = format!("if (window.setFriendsData) window.setFriendsData({json_str});");
                                let _ = proxy_init.send_event(CustomEvent::EvaluateScript(script));
                            }
                        }
                    }
                }
            });
        });
    }

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(CustomEvent::EvaluateScript(script)) => {
                let _ = webview.evaluate_script(&script);
            }
            _ => (),
        }
    });

    Ok(())
}
