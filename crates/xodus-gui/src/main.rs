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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env("XODUS_LOG");
    xodus::secrets::init_secrets().ok();

    let tokens = Arc::new(TokenManager::with_keychain_and_memory());
    let mut event_loop = EventLoopBuilder::new().build();
    
    let window = WindowBuilder::new()
        .with_title("noct's xodus gui")
        .with_decorations(false)
        .with_resizable(true)
        .with_inner_size(Size::Logical(LogicalSize::new(1280.0, 800.0)))
        .with_min_inner_size(Size::Logical(LogicalSize::new(960.0, 600.0)))
        .build(&event_loop)?;

    let window = Arc::new(window);
    let win_ipc = window.clone();

    let _tokens_clone = tokens.clone();
    let combined_html = HTML
        .replace("<link rel=\"stylesheet\" href=\"styles.css\">", &format!("<style>{}</style>", CSS))
        .replace("<script src=\"app.js\"></script>", &format!("<script>{}</script>", JS));

    let rt = tokio::runtime::Runtime::new()?;

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
                        _ => {}
                    }
                }
            }
        });

    #[cfg(target_os = "linux")]
    let _webview = {
        use gtk::prelude::WidgetExt;
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get window default vbox");
        let wv = builder.build_gtk(vbox)?;
        window.gtk_window().show_all();
        wv
    };
    #[cfg(not(target_os = "linux"))]
    let _webview = builder.build(&window)?;

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });

    Ok(())
}
