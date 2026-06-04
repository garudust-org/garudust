// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::TcpListener;
use std::sync::Mutex;

use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Holds the spawned `garudust-server` sidecar so it lives for the app's
/// lifetime and can be killed cleanly on exit.
struct Sidecar(Mutex<Option<CommandChild>>);

/// Ask the OS for a free loopback port by binding to :0 and reading it back.
/// Small TOCTOU window before the sidecar binds it again, acceptable for a
/// single-user desktop app on localhost.
fn free_loopback_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(38123)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Sidecar(Mutex::new(None)))
        .setup(|app| {
            let port = free_loopback_port();
            let gateway = format!("http://127.0.0.1:{port}");

            // Spawn the bundled gateway, bound to loopback only — the desktop
            // app never exposes the agent to the network.
            let (mut rx, child) = app
                .shell()
                .sidecar("garudust-server")?
                .args(["--port", &port.to_string()])
                .spawn()?;
            app.state::<Sidecar>().0.lock().unwrap().replace(child);

            // Forward sidecar logs to our stderr for debugging.
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stderr(line) | CommandEvent::Stdout(line) => {
                            eprintln!("[gateway] {}", String::from_utf8_lossy(&line));
                        }
                        _ => {}
                    }
                }
            });

            // Build the window in code (not config) so the gateway origin is
            // injected *before* any page script runs — the SPA reads
            // `window.__GARUDUST_GATEWAY__` at module load.
            let init = format!("window.__GARUDUST_GATEWAY__ = {};", json_str(&gateway));
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Garudust")
                .inner_size(1100.0, 760.0)
                .min_inner_size(720.0, 480.0)
                .initialization_script(&init)
                .build()?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Garudust desktop")
        .run(|app, event| {
            // Kill the gateway sidecar when the app exits so it is not orphaned.
            if let RunEvent::ExitRequested { .. } | RunEvent::Exit = event {
                if let Some(child) = app.state::<Sidecar>().0.lock().unwrap().take() {
                    let _ = child.kill();
                }
            }
        });
}

/// JSON-encode a string so it can be embedded safely in the init script.
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}
