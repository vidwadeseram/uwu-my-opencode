use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;
use crate::error::AppError;
use crate::state::{StateManager, WorkspaceStatus};
use crate::supervisor::ProcessSupervisor;
use crate::tunnel::TunnelManager;
use crate::workspace::WorkspaceManager;

#[derive(Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub state: StateManager,
    pub supervisor: ProcessSupervisor,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    execute_commands: bool,
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub opencode_port: Option<u16>,
    pub ttyd_port: Option<u16>,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct StartWorkspaceResponse {
    pub workspace: WorkspaceResponse,
    pub commands: Vec<crate::workspace::CommandResult>,
    pub browser_url: Option<String>,
}

#[derive(Serialize)]
pub struct StopWorkspaceResponse {
    pub workspace: WorkspaceResponse,
    pub commands: Vec<crate::workspace::CommandResult>,
}

#[derive(Deserialize)]
pub struct PreviewRequest {
    #[serde(default = "default_preview_port")]
    pub port: u16,
}

fn default_preview_port() -> u16 {
    3000
}

#[derive(Serialize)]
pub struct PreviewResponse {
    pub workspace_id: String,
    pub local_port: u16,
    pub tunnel_url: Option<String>,
    pub command: String,
    pub executed: bool,
}

impl From<&crate::state::Workspace> for WorkspaceResponse {
    fn from(ws: &crate::state::Workspace) -> Self {
        Self {
            id: ws.id.clone(),
            name: ws.name.clone(),
            path: ws.path.to_string_lossy().to_string(),
            opencode_port: ws.opencode_port,
            ttyd_port: ws.ttyd_port,
            status: format!("{:?}", ws.status).to_lowercase(),
            created_at: ws.created_at.to_rfc3339(),
        }
    }
}

pub fn create_router(ctx: AppContext) -> Router {
    Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(ctx)
}

async fn health(State(ctx): State<AppContext>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        execute_commands: ctx.config.execute_commands,
    })
}

async fn list_workspaces(
    State(ctx): State<AppContext>,
) -> Result<Json<Vec<WorkspaceResponse>>, AppError> {
    let workspaces = ctx.state.list_workspaces().await;
    let responses: Vec<WorkspaceResponse> = workspaces.iter().map(WorkspaceResponse::from).collect();
    Ok(Json(responses))
}

async fn create_workspace(
    State(ctx): State<AppContext>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), AppError> {
    if req.name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    if req.name.contains('/') || req.name.contains('\\') || req.name.contains("..") {
        return Err(AppError::BadRequest(
            "name must not contain path separators or '..'".into(),
        ));
    }

    let ws = ctx
        .state
        .create_workspace(&req.name, &ctx.config.workspace_root)
        .await?;

    Ok((StatusCode::CREATED, Json(WorkspaceResponse::from(&ws))))
}

async fn start_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<StartWorkspaceResponse>, AppError> {
    let ws = ctx
        .state
        .get_workspace(&id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    if ws.status == WorkspaceStatus::Running {
        return Err(AppError::Conflict("workspace is already running".into()));
    }

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let port = ws.opencode_port.unwrap_or(4100);
    let ttyd_port = ws.ttyd_port.unwrap_or(7681);

    let result = manager
        .start_workspace(&ws.name, &ws.path, port, ttyd_port)
        .await?;

    let ws = ctx
        .state
        .update_workspace_status(&id, WorkspaceStatus::Running)
        .await?;

    Ok(Json(StartWorkspaceResponse {
        workspace: WorkspaceResponse::from(&ws),
        commands: result.commands,
        browser_url: result.browser_url,
    }))
}

async fn stop_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<StopWorkspaceResponse>, AppError> {
    let ws = ctx
        .state
        .get_workspace(&id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let commands = manager.stop_workspace(&ws.name).await?;

    let tunnels = ctx.state.list_tunnels(&id).await;
    let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
    for t in &tunnels {
        tunnel_mgr.stop_tunnel(&id, t.local_port).await;
    }
    ctx.state.remove_all_tunnels_for_workspace(&id).await;

    let ws = ctx
        .state
        .update_workspace_status(&id, WorkspaceStatus::Stopped)
        .await?;

    Ok(Json(StopWorkspaceResponse {
        workspace: WorkspaceResponse::from(&ws),
        commands,
    }))
}

async fn delete_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<WorkspaceResponse>, AppError> {
    let ws = ctx
        .state
        .get_workspace(&id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    if ws.status == WorkspaceStatus::Running {
        let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
        let _ = manager.stop_workspace(&ws.name).await;

        let tunnels = ctx.state.list_tunnels(&id).await;
        let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
        for t in &tunnels {
            tunnel_mgr.stop_tunnel(&id, t.local_port).await;
        }
    }

    let ws = ctx.state.delete_workspace(&id).await?;

    if ctx.config.execute_commands && ws.path.exists() {
        let _ = tokio::fs::remove_dir_all(&ws.path).await;
    }

    Ok(Json(WorkspaceResponse::from(&ws)))
}

async fn create_preview(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<PreviewRequest>,
) -> Result<(StatusCode, Json<PreviewResponse>), AppError> {
    let ws = ctx
        .state
        .get_workspace(&id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let result = tunnel_mgr.start_tunnel(&ws.id, req.port).await?;

    ctx.state
        .add_tunnel(&ws.id, req.port, result.tunnel_url.clone())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(PreviewResponse {
            workspace_id: ws.id,
            local_port: req.port,
            tunnel_url: result.tunnel_url,
            command: result.command,
            executed: result.executed,
        }),
    ))
}

async fn delete_preview(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ws = ctx
        .state
        .get_workspace(&id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
    tunnel_mgr.stop_tunnel(&ws.id, req.port).await;

    ctx.state.remove_tunnel(&ws.id, req.port).await?;

    Ok(Json(serde_json::json!({
        "stopped": true,
        "workspace_id": ws.id,
        "port": req.port,
    })))
}

async fn list_previews(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::state::Tunnel>>, AppError> {
    let _ = ctx
        .state
        .get_workspace(&id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let tunnels = ctx.state.list_tunnels(&id).await;
    Ok(Json(tunnels))
}
