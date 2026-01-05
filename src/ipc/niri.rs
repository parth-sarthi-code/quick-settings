use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::RwLock;

use crate::state::{AppState, WindowState, WorkspaceInfo, WorkspaceState};

// Async listener for niri IPC events

/// Niri IPC event subscription
/// Reference: https://github.com/YaLTeR/niri/wiki/IPC
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum NiriRequest {
    EventStream,
    Workspaces,
    Windows,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum NiriResponse {
    Ok {
        #[serde(rename = "Ok")]
        data: ResponseData,
    },
    Err {
        #[serde(rename = "Err")]
        err: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ResponseData {
    Handled(String),
    Workspaces(Vec<Workspace>),
    Windows(Vec<Window>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum NiriMessage {
    Response(NiriResponse),
    Event(NiriEvent),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum NiriEvent {
    WorkspaceActivated {
        #[serde(flatten)]
        data: WorkspaceActivatedData,
    },
    WorkspacesChanged {
        workspaces: Vec<Workspace>,
    },
    WindowOpenedOrChanged {
        window: Window,
    },
    WindowsChanged {
        windows: Vec<Window>,
    },
    WindowClosed {
        id: u64,
    },
    WindowFocusChanged {
        id: Option<u64>,
    },
    WindowLayoutsChanged {
        changes: Vec<(u64, serde_json::Value)>,
    },
    KeyboardLayoutsChanged {
        keyboard_layouts: serde_json::Value,
    },
    OverviewOpenedOrClosed {
        is_open: bool,
    },
    ConfigLoaded {
        failed: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceActivatedData {
    id: u64,
    focused: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Workspace {
    id: u64,
    idx: Option<usize>,
    name: Option<String>,
    output: Option<String>,
    is_active: bool,
    is_focused: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Window {
    id: u64,
    title: Option<String>,
    app_id: Option<String>,
    workspace_id: Option<u64>,
    is_focused: bool,
}

/// Start listening to niri IPC events
pub async fn start_listener(state: Arc<RwLock<AppState>>) -> Result<()> {
    // Get niri socket path from environment
    let socket_path =
        std::env::var("NIRI_SOCKET").context("NIRI_SOCKET environment variable not set")?;

    // Main event stream connection
    let stream = UnixStream::connect(&socket_path)
        .await
        .context("Failed to connect to niri socket")?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Subscribe to event stream
    let request = NiriRequest::EventStream;
    let request_json = serde_json::to_string(&request)?;
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    println!("Connected to niri IPC, listening for events...");

    // Query initial state - workspaces
    let request = NiriRequest::Workspaces;
    let request_json = serde_json::to_string(&request)?;
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Query initial state - windows
    let request = NiriRequest::Windows;
    let request_json = serde_json::to_string(&request)?;
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read events line by line from main connection
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                eprintln!("niri IPC connection closed");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && !trimmed.contains("Ok")
                    && !trimmed.contains("WindowLayouts")
                {
                    println!("[EVENT] {}", &trimmed[..std::cmp::min(150, trimmed.len())]);
                }
                if let Err(e) = handle_event(&line, &state).await {
                    eprintln!("Error handling event: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error reading from niri socket: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single IPC event
async fn handle_event(line: &str, state: &Arc<RwLock<AppState>>) -> Result<()> {
    let message: NiriMessage =
        serde_json::from_str(line).context("Failed to parse niri message")?;

    match message {
        NiriMessage::Response(resp) => {
            match resp {
                NiriResponse::Ok { data } => {
                    match data {
                        ResponseData::Workspaces(workspaces) => {
                            let mut app_state = state.write().await;
                            let total = workspaces.len();

                            // Store all workspaces for quick lookup on WorkspaceActivated
                            app_state.workspaces = workspaces
                                .iter()
                                .map(|ws| WorkspaceInfo {
                                    id: ws.id,
                                    idx: ws.idx,
                                    name: ws.name.clone(),
                                })
                                .collect();

                            // Update current workspace in app state
                            if let Some(active_ws) = workspaces.iter().find(|ws| ws.idx.is_some()) {
                                app_state.workspace = WorkspaceState {
                                    current_idx: active_ws.idx.unwrap_or(1),
                                    current_name: active_ws.name.clone(),
                                    total_count: total,
                                    workspace_id: Some(active_ws.id),
                                };
                                eprintln!(
                                    "[INIT_WS] {}/{} (id={}, name={:?})",
                                    active_ws.idx.unwrap_or(1),
                                    total,
                                    active_ws.id,
                                    active_ws.name
                                );
                            }
                        }
                        ResponseData::Windows(windows) => {
                            if let Some(focused_window) = windows.iter().find(|w| w.is_focused) {
                                let mut app_state = state.write().await;
                                app_state.window = WindowState {
                                    app_name: focused_window.app_id.clone(),
                                    title: focused_window.title.clone(),
                                };
                                eprintln!(
                                    "[INIT_WINDOW] {} | {}",
                                    focused_window.app_id.as_deref().unwrap_or("(no app)"),
                                    focused_window.title.as_deref().unwrap_or("(no title)")
                                );
                            }
                        }
                        ResponseData::Handled(_) => {
                            // Ignore EventStream subscription confirmation
                        }
                    }
                }
                NiriResponse::Err { err } => {
                    eprintln!("niri IPC error: {}", err);
                }
            }
        }
        NiriMessage::Event(event) => match event {
            NiriEvent::WorkspaceActivated { data } => {
                eprintln!("[WS_ACTIVATED] id={}, focused={}", data.id, data.focused);
                let mut app_state = state.write().await;

                // Look up workspace from our stored list and update if found
                if let Some(ws) = app_state.workspaces.iter().find(|w| w.id == data.id) {
                    let idx = ws.idx.unwrap_or(1);
                    let name = ws.name.clone();
                    let total = app_state.workspaces.len();
                    app_state.workspace = WorkspaceState {
                        current_idx: idx,
                        current_name: name.clone(),
                        total_count: total,
                        workspace_id: Some(ws.id),
                    };
                    eprintln!(
                        "[WS_SWITCH✓] {} → {}/{} (name={:?})",
                        data.id, idx, total, name
                    );
                } else {
                    eprintln!(
                        "[WS_SWITCH✗] id {} NOT in stored list (have {} ws)",
                        data.id,
                        app_state.workspaces.len()
                    );
                }
            }

            NiriEvent::WorkspacesChanged { workspaces } => {
                let mut app_state = state.write().await;
                let total = workspaces.len();

                // Update workspace list
                app_state.workspaces = workspaces
                    .iter()
                    .map(|ws| WorkspaceInfo {
                        id: ws.id,
                        idx: ws.idx,
                        name: ws.name.clone(),
                    })
                    .collect();

                // Find active workspace
                if let Some(active_ws) = workspaces.iter().find(|ws| ws.is_focused || ws.is_active)
                {
                    app_state.workspace = WorkspaceState {
                        current_idx: active_ws.idx.unwrap_or(1),
                        current_name: active_ws.name.clone(),
                        total_count: total,
                        workspace_id: Some(active_ws.id),
                    };
                    eprintln!(
                        "[WS_LIST_CHANGED] {}/{} (id={}, active={}, focused={})",
                        active_ws.idx.unwrap_or(1),
                        total,
                        active_ws.id,
                        active_ws.is_active,
                        active_ws.is_focused
                    );
                }
            }

            NiriEvent::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    let mut app_state = state.write().await;
                    app_state.window = WindowState {
                        app_name: window.app_id.clone(),
                        title: window.title.clone(),
                    };
                    println!(
                        "Focused window: {} - {:?}",
                        window.app_id.as_deref().unwrap_or("?"),
                        window.title
                    );
                }
            }

            NiriEvent::WindowsChanged { windows } => {
                // Find focused window
                if let Some(focused_window) = windows.iter().find(|w| w.is_focused) {
                    let mut app_state = state.write().await;
                    app_state.window = WindowState {
                        app_name: focused_window.app_id.clone(),
                        title: focused_window.title.clone(),
                    };
                    println!(
                        "Focused window: {} - {:?}",
                        focused_window.app_id.as_deref().unwrap_or("?"),
                        focused_window.title
                    );
                }
            }

            NiriEvent::WindowClosed { id } => {
                println!("Window closed: {}", id);
            }

            NiriEvent::WindowFocusChanged { id } => {
                if id.is_none() {
                    // No window focused - clear title
                    let mut app_state = state.write().await;
                    app_state.window = WindowState::default();
                    println!("No window focused");
                }
            }

            // Silently ignore other events
            _ => {}
        },
    }

    Ok(())
}
