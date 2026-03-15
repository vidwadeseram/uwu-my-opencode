use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub opencode_port: Option<u16>,
    pub ttyd_port: Option<u16>,
    pub tmux_window: Option<u32>,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    Created,
    Starting,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub workspace_id: String,
    pub local_port: u16,
    pub tunnel_url: Option<String>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub workspaces: Vec<Workspace>,
    pub tunnels: Vec<Tunnel>,
    #[serde(default)]
    pub next_opencode_port: u16,
    #[serde(default)]
    pub next_ttyd_port: u16,
}

#[derive(Clone)]
pub struct StateManager {
    state: Arc<RwLock<AppState>>,
    file_path: PathBuf,
    port_range_start: u16,
    port_range_end: u16,
    ttyd_port_start: u16,
}

impl StateManager {
    pub fn new(
        file_path: PathBuf,
        port_range_start: u16,
        port_range_end: u16,
        ttyd_port_start: u16,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
            file_path,
            port_range_start,
            port_range_end,
            ttyd_port_start,
        }
    }

    pub async fn load(&self) -> Result<(), AppError> {
        if self.file_path.exists() {
            let data = tokio::fs::read_to_string(&self.file_path).await?;
            let mut state: AppState = serde_json::from_str(&data)?;
            if state.next_opencode_port < self.port_range_start {
                state.next_opencode_port = self.port_range_start;
            }
            if state.next_ttyd_port < self.ttyd_port_start {
                state.next_ttyd_port = self.ttyd_port_start;
            }
            *self.state.write().await = state;
        } else {
            let mut state = AppState::default();
            state.next_opencode_port = self.port_range_start;
            state.next_ttyd_port = self.ttyd_port_start;
            *self.state.write().await = state;
        }
        Ok(())
    }

    async fn persist(&self) -> Result<(), AppError> {
        let state = self.state.read().await;
        let data = serde_json::to_string_pretty(&*state)?;
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&self.file_path, data).await?;
        Ok(())
    }

    pub async fn list_workspaces(&self) -> Vec<Workspace> {
        self.state.read().await.workspaces.clone()
    }

    pub async fn get_workspace(&self, id: &str) -> Option<Workspace> {
        self.state
            .read()
            .await
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .cloned()
    }

    pub async fn create_workspace(
        &self,
        name: &str,
        workspace_root: &Path,
    ) -> Result<Workspace, AppError> {
        let mut state = self.state.write().await;

        if state.workspaces.iter().any(|w| w.name == name) {
            return Err(AppError::Conflict(format!(
                "workspace '{}' already exists",
                name
            )));
        }

        let id = Uuid::new_v4().to_string();
        let path = workspace_root.join(&name);
        let port = state.next_opencode_port;
        let ttyd_port = state.next_ttyd_port;

        if port > self.port_range_end {
            return Err(AppError::BadRequest(
                "no available ports in range".to_string(),
            ));
        }

        state.next_opencode_port = port + 1;
        state.next_ttyd_port = ttyd_port + 1;

        let workspace = Workspace {
            id,
            name: name.to_string(),
            path,
            opencode_port: Some(port),
            ttyd_port: Some(ttyd_port),
            tmux_window: Some(state.workspaces.len() as u32),
            status: WorkspaceStatus::Created,
            created_at: Utc::now(),
        };

        state.workspaces.push(workspace.clone());
        drop(state);
        self.persist().await?;
        Ok(workspace)
    }

    pub async fn update_workspace_status(
        &self,
        id: &str,
        status: WorkspaceStatus,
    ) -> Result<Workspace, AppError> {
        let mut state = self.state.write().await;
        let ws = state
            .workspaces
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;
        ws.status = status;
        let ws = ws.clone();
        drop(state);
        self.persist().await?;
        Ok(ws)
    }

    pub async fn add_tunnel(
        &self,
        workspace_id: &str,
        local_port: u16,
        tunnel_url: Option<String>,
    ) -> Result<Tunnel, AppError> {
        let mut state = self.state.write().await;

        if !state.workspaces.iter().any(|w| w.id == workspace_id) {
            return Err(AppError::NotFound(format!(
                "workspace '{workspace_id}' not found"
            )));
        }

        if state
            .tunnels
            .iter()
            .any(|t| t.workspace_id == workspace_id && t.local_port == local_port)
        {
            return Err(AppError::Conflict(format!(
                "tunnel for port {local_port} already exists in workspace '{workspace_id}'"
            )));
        }

        let tunnel = Tunnel {
            id: Uuid::new_v4().to_string(),
            workspace_id: workspace_id.to_string(),
            local_port,
            tunnel_url,
            started_at: Utc::now(),
        };

        state.tunnels.push(tunnel.clone());
        drop(state);
        self.persist().await?;
        Ok(tunnel)
    }

    pub async fn list_tunnels(&self, workspace_id: &str) -> Vec<Tunnel> {
        self.state
            .read()
            .await
            .tunnels
            .iter()
            .filter(|t| t.workspace_id == workspace_id)
            .cloned()
            .collect()
    }

    pub async fn remove_tunnel(&self, workspace_id: &str, local_port: u16) -> Result<(), AppError> {
        let mut state = self.state.write().await;
        let before = state.tunnels.len();
        state
            .tunnels
            .retain(|t| !(t.workspace_id == workspace_id && t.local_port == local_port));
        if state.tunnels.len() == before {
            return Err(AppError::NotFound(format!(
                "no tunnel on port {} for workspace '{}'",
                local_port, workspace_id
            )));
        }
        drop(state);
        self.persist().await
    }

    pub async fn remove_all_tunnels_for_workspace(&self, workspace_id: &str) {
        let mut state = self.state.write().await;
        state.tunnels.retain(|t| t.workspace_id != workspace_id);
        drop(state);
        let _ = self.persist().await;
    }

    pub async fn delete_workspace(&self, id: &str) -> Result<Workspace, AppError> {
        let mut state = self.state.write().await;
        let idx = state
            .workspaces
            .iter()
            .position(|w| w.id == id)
            .ok_or_else(|| AppError::NotFound(format!("workspace '{}' not found", id)))?;
        let ws = state.workspaces.remove(idx);
        state.tunnels.retain(|t| t.workspace_id != id);
        drop(state);
        self.persist().await?;
        Ok(ws)
    }
}
