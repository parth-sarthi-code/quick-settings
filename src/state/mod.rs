// use std::fmt; // unused

/// Shared application state
/// Updated by IPC listener, read by GTK UI
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub workspaces: Vec<WorkspaceInfo>,
    pub workspace: WorkspaceState,
    pub window: WindowState,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceInfo {
    pub id: u64,
    pub idx: Option<usize>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceState {
    pub current_idx: usize,
    pub current_name: Option<String>,
    pub total_count: usize,
    #[allow(dead_code)]
    pub workspace_id: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct WindowState {
    pub app_name: Option<String>,
    pub title: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get formatted workspace text for display
    pub fn workspace_text(&self) -> String {
        if let Some(name) = &self.workspace.current_name {
            name.clone()
        } else if self.workspace.total_count > 0 {
            format!("{}/{}", self.workspace.current_idx, self.workspace.total_count)
        } else {
            "—".to_string()
        }
    }

    /// Get formatted window title for display
    pub fn window_title(&self) -> String {
        match (&self.window.app_name, &self.window.title) {
            (Some(app), Some(title)) => format!("{}: {}", app, title),
            (Some(app), None) => app.clone(),
            (None, Some(title)) => title.clone(),
            (None, None) => String::new(),
        }
    }
}
