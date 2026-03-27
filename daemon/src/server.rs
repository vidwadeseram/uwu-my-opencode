use axum::{
    extract::Path,
    extract::Query,
    extract::State,
    http::{
        header::{CACHE_CONTROL, EXPIRES, PRAGMA, WWW_AUTHENTICATE},
        StatusCode,
    },
    response::{Html, IntoResponse},
    routing::{delete, get, post},
    Json, Router,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path as StdPath, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::warn;

use crate::commander::{CaptureResponse, CommanderState, Message, SessionInfo};
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
    pub commander: CommanderState,
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
    pub browser_url: Option<String>,
    pub terminal_url: Option<String>,
    pub preview_urls: Vec<PreviewLink>,
    pub size_mb: Option<u64>,
}

#[derive(Serialize)]
pub struct PreviewLink {
    pub local_port: u16,
    pub local_url: String,
    pub public_url: Option<String>,
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

#[derive(Serialize)]
pub struct TmuxTestLogResponse {
    pub workspace: WorkspaceResponse,
    pub sessions: Vec<String>,
    pub log_file: String,
}

#[derive(Serialize)]
pub struct PublishFrontendsResponse {
    pub workspace: WorkspaceResponse,
    pub published_ports: Vec<u16>,
    pub skipped_ports: Vec<u16>,
}

#[derive(Serialize)]
pub struct StopFrontendsResponse {
    pub workspace: WorkspaceResponse,
    pub stopped_ports: Vec<u16>,
}

#[derive(Deserialize)]
struct FrontendManifest {
    #[serde(default)]
    frontends: Vec<FrontendDefinition>,
}

#[derive(Deserialize)]
struct FrontendDefinition {
    port: u16,
}

#[derive(Serialize)]
pub struct TestReportsResponse {
    pub generated_at: String,
    pub workspaces: Vec<TestReportsWorkspace>,
}

#[derive(Serialize)]
pub struct TestReportsWorkspace {
    pub workspace_id: String,
    pub workspace_name: String,
    pub runs: Vec<TestReportRun>,
}

#[derive(Serialize)]
pub struct TestReportRun {
    pub run_id: String,
    pub created_at: Option<String>,
    pub status: String,
    pub pass_rate: f64,
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub blocked: u64,
    pub issue: Option<String>,
    pub html_url: Option<String>,
    pub tested_count: u64,
    pub tested_scope: Option<String>,
    pub quality_warning: Option<String>,
    pub screenshot_files: u64,
    pub video_files: u64,
}

#[derive(Deserialize)]
struct TestReportManifest {
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    summary: Option<TestReportSummary>,
    #[serde(default)]
    tests: Vec<TestReportCase>,
    #[serde(default)]
    screenshots: Vec<TestReportScreenshot>,
    #[serde(default)]
    video: Option<TestReportVideo>,
    #[serde(default)]
    blocker: Option<String>,
}

#[derive(Deserialize)]
struct TestReportVideo {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Deserialize)]
struct TestReportSummary {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    passed: u64,
    #[serde(default)]
    failed: u64,
    #[serde(default)]
    skipped: u64,
    #[serde(default)]
    blocked: u64,
}

#[derive(Deserialize)]
struct TestReportCoverage {
    #[serde(default)]
    route_total: u64,
    #[serde(default)]
    route_covered: u64,
    #[serde(default)]
    button_total: u64,
    #[serde(default)]
    button_covered: u64,
    #[serde(default)]
    form_total: u64,
    #[serde(default)]
    form_covered: u64,
    #[serde(default)]
    functional_total: u64,
    #[serde(default)]
    functional_covered: u64,
}

#[derive(Default)]
struct ArtifactStats {
    files: u64,
    zero_bytes: u64,
}

#[derive(Deserialize)]
struct TestReportCase {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct TestReportScreenshot {
    #[serde(default)]
    test_id: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    description: Option<String>,
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
        let terminal_url = None;

        Self {
            id: ws.id.clone(),
            name: ws.name.clone(),
            path: ws.path.to_string_lossy().to_string(),
            opencode_port: ws.opencode_port,
            ttyd_port: ws.ttyd_port,
            status: format!("{:?}", ws.status).to_lowercase(),
            created_at: ws.created_at.to_rfc3339(),
            browser_url: None,
            terminal_url,
            preview_urls: Vec::new(),
            size_mb: None,
        }
    }
}

#[derive(Serialize)]
struct VmInfoResponse {
    hostname: String,
    cpu_cores: u32,
    cpu_usage_percent: Option<f64>,
    memory_total_mb: u64,
    memory_used_mb: u64,
    disk_total_gb: u64,
    disk_used_gb: u64,
    uptime_seconds: u64,
    os: String,
}

#[derive(Deserialize)]
struct ResetPasswordRequest {
    user: String,
    new_password: String,
}

#[derive(Serialize)]
struct ResetPasswordResponse {
    message: String,
    wrapper_path: String,
}

#[derive(Deserialize)]
struct CommanderSendRequest {
    message: String,
}

#[derive(Deserialize)]
struct CommanderMessagesQuery {
    since: Option<u64>,
}

#[derive(Deserialize)]
struct CommanderSwitchRequest {
    target: String,
}

#[derive(Serialize)]
struct CommanderSwitchResponse {
    target: String,
}

pub fn create_router(ctx: AppContext) -> Router {
    let static_dir = resolve_static_dir();

    Router::new()
        .route("/health", get(health))
        .route("/", get(dashboard_index))
        .route("/test-reports", get(test_reports_index))
        .route(
            "/test-reports/{workspace}/{run_id}/index.html",
            get(test_report_index),
        )
        .route(
            "/test-reports/{workspace}/{run_id}/manifest.json",
            get(test_report_manifest),
        )
        .route(
            "/test-reports/{workspace}/{run_id}/screenshots/{file}",
            get(test_report_asset_screenshot),
        )
        .route(
            "/test-reports/{workspace}/{run_id}/video/{file}",
            get(test_report_asset_video),
        )
        .route(
            "/test-reports/{workspace}/{run_id}/logs/{legacy_run_id}/{*asset_path}",
            get(test_report_asset_legacy),
        )
        .route(
            "/test-reports/{workspace}/{run_id}/detail.html",
            get(test_report_detail),
        )
        .route("/commander", get(commander_index))
        .route("/logout", get(logout))
        .nest_service("/static", ServeDir::new(static_dir))
        .route("/api/vm", get(vm_info))
        .route("/api/test-reports", get(list_test_reports))
        .route(
            "/api/workspaces",
            get(list_workspaces).post(create_workspace),
        )
        .route("/api/workspaces/{id}", delete(delete_workspace))
        .route("/api/workspaces/{id}/start", post(start_workspace))
        .route("/api/workspaces/{id}/stop", post(stop_workspace))
        .route(
            "/api/workspaces/{id}/publish-frontends",
            post(publish_frontends),
        )
        .route("/api/workspaces/{id}/stop-frontends", post(stop_frontends))
        .route(
            "/api/workspaces/{id}/previews",
            get(list_previews)
                .post(create_preview)
                .delete(delete_preview),
        )
        .route("/api/projects", get(list_workspaces))
        .route("/api/projects/{id}/start", post(start_workspace))
        .route("/api/projects/{id}/stop", post(stop_workspace))
        .route(
            "/api/projects/{id}/publish-frontends",
            post(publish_frontends),
        )
        .route("/api/projects/{id}/stop-frontends", post(stop_frontends))
        .route(
            "/api/projects/{id}/tmux-test-log",
            post(create_tmux_test_log),
        )
        .route("/api/projects/{id}", delete(delete_workspace))
        .route("/api/reset-password", post(reset_password))
        .route("/api/commander/send", post(commander_send))
        .route("/api/commander/messages", get(commander_messages))
        .route("/api/commander/sessions", get(commander_sessions))
        .route(
            "/api/commander/session/switch",
            post(commander_switch_session),
        )
        .route("/api/commander/capture", get(commander_capture))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(ctx)
}

async fn dashboard_index() -> Result<Html<String>, AppError> {
    let index_path = resolve_static_dir().join("index.html");
    let html = tokio::fs::read_to_string(&index_path)
        .await
        .map_err(|err| {
            AppError::NotFound(format!(
                "dashboard file not found at '{}': {}",
                index_path.display(),
                err
            ))
        })?;
    Ok(Html(html))
}

async fn commander_index() -> Result<Html<String>, AppError> {
    let commander_path = resolve_static_dir().join("commander.html");
    let html = tokio::fs::read_to_string(&commander_path)
        .await
        .map_err(|err| {
            AppError::NotFound(format!(
                "commander file not found at '{}': {}",
                commander_path.display(),
                err
            ))
        })?;
    Ok(Html(html))
}

async fn test_reports_index() -> Result<Html<String>, AppError> {
    let page_path = resolve_static_dir().join("test-reports.html");
    let html = tokio::fs::read_to_string(&page_path).await.map_err(|err| {
        AppError::NotFound(format!(
            "test reports page not found at '{}': {}",
            page_path.display(),
            err
        ))
    })?;
    Ok(Html(html))
}

async fn logout() -> impl IntoResponse {
    (
        StatusCode::UNAUTHORIZED,
        [
            (
                WWW_AUTHENTICATE,
                "Basic realm=\"uwu workspace\", charset=\"UTF-8\"",
            ),
            (
                CACHE_CONTROL,
                "no-store, no-cache, must-revalidate, proxy-revalidate",
            ),
            (PRAGMA, "no-cache"),
            (EXPIRES, "0"),
        ],
        "Logged out",
    )
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
    sync_state_with_workspace_dirs(&ctx).await?;
    let workspaces = ctx.state.list_workspaces().await;
    let mut responses = Vec::with_capacity(workspaces.len());

    for ws in &workspaces {
        let mut response = workspace_response_with_links(&ctx, ws).await;
        response.size_mb = None;
        responses.push(response);
    }

    Ok(Json(responses))
}

async fn sync_state_with_workspace_dirs(ctx: &AppContext) -> Result<(), AppError> {
    tokio::fs::create_dir_all(&ctx.config.workspace_root).await?;
    let mut entries = tokio::fs::read_dir(&ctx.config.workspace_root).await?;
    let mut found_workspace_dir = false;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        let trimmed = name.trim();
        if trimmed.is_empty() {
            continue;
        }

        found_workspace_dir = true;

        let _ = ctx
            .state
            .ensure_workspace(trimmed, &ctx.config.workspace_root)
            .await;
    }

    if !found_workspace_dir {
        let existing = ctx.state.list_workspaces().await;
        if existing.is_empty() {
            let default_name = "workspace-1";
            let default_path = ctx.config.workspace_root.join(default_name);
            tokio::fs::create_dir_all(&default_path).await?;
            let _ = ctx
                .state
                .ensure_workspace(default_name, &ctx.config.workspace_root)
                .await;
        }
    }

    Ok(())
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

    tokio::fs::create_dir_all(&ws.path).await?;

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    manager.setup_workspace_opencode_files(&ws.path).await?;

    Ok((StatusCode::CREATED, Json(WorkspaceResponse::from(&ws))))
}

async fn start_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<StartWorkspaceResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    if ws.status == WorkspaceStatus::Running {
        return Err(AppError::Conflict("workspace is already running".into()));
    }

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    manager.setup_workspace_opencode_files(&ws.path).await?;
    let port = ws.opencode_port.unwrap_or(4100);
    let ttyd_port = ws.ttyd_port.unwrap_or(7681);

    let result = manager
        .start_workspace(&ws.name, &ws.path, port, ttyd_port)
        .await?;

    let ws = ctx
        .state
        .update_workspace_status(&id, WorkspaceStatus::Running)
        .await?;

    let _ = publish_declared_frontends(&ctx, &ws).await;
    let workspace = workspace_response_with_links(&ctx, &ws).await;

    Ok(Json(StartWorkspaceResponse {
        workspace,
        commands: result.commands,
        browser_url: result.browser_url,
    }))
}

async fn stop_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<StopWorkspaceResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let commands = manager.stop_workspace(&ws.name, &ws.path).await?;

    let tunnels = ctx.state.list_tunnels(&ws.id).await;
    let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
    for t in &tunnels {
        tunnel_mgr.stop_tunnel(&ws.id, t.local_port).await;
    }
    ctx.state.remove_all_tunnels_for_workspace(&ws.id).await;

    let ws = ctx
        .state
        .update_workspace_status(&id, WorkspaceStatus::Stopped)
        .await?;
    let workspace = workspace_response_with_links(&ctx, &ws).await;

    Ok(Json(StopWorkspaceResponse {
        workspace,
        commands,
    }))
}

async fn delete_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<WorkspaceResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    if ws.status == WorkspaceStatus::Running {
        let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
        let _ = manager.stop_workspace(&ws.name, &ws.path).await;

        let tunnels = ctx.state.list_tunnels(&ws.id).await;
        let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
        for t in &tunnels {
            tunnel_mgr.stop_tunnel(&ws.id, t.local_port).await;
        }
    }

    let ws = ctx.state.delete_workspace(&id).await?;

    if ctx.config.execute_commands && ws.path.exists() {
        let _ = tokio::fs::remove_dir_all(&ws.path).await;
    }

    Ok(Json(WorkspaceResponse::from(&ws)))
}

async fn create_tmux_test_log(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<TmuxTestLogResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let result = manager.create_tmux_test_log(&ws.name, &ws.path).await?;

    let workspace = workspace_response_with_links(&ctx, &ws).await;

    Ok(Json(TmuxTestLogResponse {
        workspace,
        sessions: result.sessions,
        log_file: result.log_file,
    }))
}

async fn publish_frontends(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<PublishFrontendsResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let (published_ports, skipped_ports) = publish_declared_frontends(&ctx, &ws).await;
    let workspace = workspace_response_with_links(&ctx, &ws).await;

    Ok(Json(PublishFrontendsResponse {
        workspace,
        published_ports,
        skipped_ports,
    }))
}

async fn stop_frontends(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<StopFrontendsResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let stopped_ports = stop_declared_frontends(&ctx, &ws).await;
    let workspace = workspace_response_with_links(&ctx, &ws).await;

    Ok(Json(StopFrontendsResponse {
        workspace,
        stopped_ports,
    }))
}

async fn stop_declared_frontends(ctx: &AppContext, ws: &crate::state::Workspace) -> Vec<u16> {
    let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let existing_tunnels = ctx.state.list_tunnels(&ws.id).await;
    let declared_ports = declared_frontend_ports(&ws.path).await;
    let mut stopped = Vec::new();

    for tunnel in existing_tunnels {
        if tunnel_mgr.stop_tunnel(&ws.id, tunnel.local_port).await {
            let _ = ctx.state.remove_tunnel(&ws.id, tunnel.local_port).await;
            stopped.push(tunnel.local_port);
        }
    }

    for port in &declared_ports {
        let _ = Command::new("pkill")
            .args(["-f", &format!("next dev.*{}\"", port)])
            .output()
            .await;
        let _ = Command::new("pkill")
            .args(["-f", &format!(":{}", port)])
            .output()
            .await;
    }

    stopped
}

async fn list_test_reports(
    State(ctx): State<AppContext>,
) -> Result<Json<TestReportsResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let list = ctx.state.list_workspaces().await;
    let mut out = Vec::with_capacity(list.len());

    for ws in &list {
        let runs = workspace_test_runs(ws).await;
        out.push(TestReportsWorkspace {
            workspace_id: ws.id.clone(),
            workspace_name: ws.name.clone(),
            runs,
        });
    }

    out.sort_by(|a, b| a.workspace_name.cmp(&b.workspace_name));

    Ok(Json(TestReportsResponse {
        generated_at: chrono::Utc::now().to_rfc3339(),
        workspaces: out,
    }))
}

async fn test_report_index(
    State(ctx): State<AppContext>,
    Path((workspace, run_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let path = resolve_test_report_file(&ctx, &workspace, &run_id, "index.html").await?;
    let content = tokio::fs::read(path).await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        content,
    ))
}

async fn test_report_manifest(
    State(ctx): State<AppContext>,
    Path((workspace, run_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let path = resolve_test_report_file(&ctx, &workspace, &run_id, "manifest.json").await?;
    let content = tokio::fs::read(path).await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        content,
    ))
}

async fn test_report_asset_screenshot(
    State(ctx): State<AppContext>,
    Path((workspace, run_id, file)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    if !safe_file_name(&file) {
        return Err(AppError::BadRequest(
            "invalid screenshot filename".to_string(),
        ));
    }
    let rel = format!("screenshots/{}", file);
    let path = resolve_test_report_file(&ctx, &workspace, &run_id, &rel).await?;
    let content = tokio::fs::read(&path).await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, mime_for(&path))],
        content,
    ))
}

async fn test_report_asset_video(
    State(ctx): State<AppContext>,
    Path((workspace, run_id, file)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    if !safe_file_name(&file) {
        return Err(AppError::BadRequest("invalid video filename".to_string()));
    }
    let rel = format!("video/{}", file);
    let path = resolve_test_report_file(&ctx, &workspace, &run_id, &rel).await?;
    let content = tokio::fs::read(&path).await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, mime_for(&path))],
        content,
    ))
}

async fn test_report_asset_legacy(
    State(ctx): State<AppContext>,
    Path((workspace, run_id, legacy_run_id, asset_path)): Path<(String, String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    if run_id != legacy_run_id {
        return Err(AppError::NotFound("report file not found".to_string()));
    }
    if asset_path.is_empty() {
        return Err(AppError::BadRequest("invalid report path".to_string()));
    }
    let path = resolve_test_report_file(&ctx, &workspace, &run_id, &asset_path).await?;
    let content = tokio::fs::read(&path).await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, mime_for(&path))],
        content,
    ))
}

async fn test_report_detail(
    State(_ctx): State<AppContext>,
    Path((workspace, run_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let static_dir = resolve_static_dir();
    let template_path = static_dir.join("report-detail.html");
    let html = tokio::fs::read_to_string(&template_path)
        .await
        .map_err(|err| {
            AppError::NotFound(format!(
                "detail template not found at '{}': {}",
                template_path.display(),
                err
            ))
        })?;
    let filled = html
        .replace("[[WORKSPACE]]", &workspace)
        .replace("[[RUN_ID]]", &run_id);
    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        filled,
    ))
}

async fn create_preview(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<PreviewRequest>,
) -> Result<(StatusCode, Json<PreviewResponse>), AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
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
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
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
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let tunnels = ctx.state.list_tunnels(&ws.id).await;
    Ok(Json(tunnels))
}

async fn resolve_workspace_by_id_or_name(
    ctx: &AppContext,
    id_or_name: &str,
) -> Result<Option<crate::state::Workspace>, AppError> {
    if let Some(ws) = ctx.state.get_workspace(id_or_name).await {
        return Ok(Some(ws));
    }
    if let Some(ws) = ctx.state.get_workspace_by_name(id_or_name).await {
        return Ok(Some(ws));
    }
    Ok(None)
}

async fn workspace_test_runs(ws: &crate::state::Workspace) -> Vec<TestReportRun> {
    let root = ws.path.join("logs");
    let mut entries = match tokio::fs::read_dir(&root).await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    while let Ok(Some(item)) = entries.next_entry().await {
        let file_type = match item.file_type().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }

        let run_id = item.file_name().to_string_lossy().to_string();
        if !is_test_run_id(&run_id) {
            continue;
        }

        let run_dir = item.path();
        let manifest_path = run_dir.join("manifest.json");
        let coverage_path = run_dir.join("coverage.json");
        let index_path = run_dir.join("index.html");
        let screenshots_dir = run_dir.join("screenshots");
        let video_dir = run_dir.join("video");

        let has_index = tokio::fs::metadata(&index_path)
            .await
            .map(|x| x.is_file())
            .unwrap_or(false);
        let has_screenshots_dir = tokio::fs::metadata(&screenshots_dir)
            .await
            .map(|x| x.is_dir())
            .unwrap_or(false);
        let has_video_dir = tokio::fs::metadata(&video_dir)
            .await
            .map(|x| x.is_dir())
            .unwrap_or(false);

        let screenshot_stats =
            artifact_stats(&screenshots_dir, &["png", "jpg", "jpeg", "webp"]).await;
        let video_stats = artifact_stats(&video_dir, &["mp4", "webm"]).await;

        let report_html = if has_index {
            tokio::fs::read_to_string(&index_path).await.ok()
        } else {
            None
        };

        let manifest = tokio::fs::read_to_string(&manifest_path)
            .await
            .ok()
            .and_then(|raw| serde_json::from_str::<TestReportManifest>(&raw).ok());

        let coverage = tokio::fs::read_to_string(&coverage_path)
            .await
            .ok()
            .and_then(|raw| serde_json::from_str::<TestReportCoverage>(&raw).ok());

        let missing_manifest_screenshot_files =
            manifest_missing_screenshot_files(&run_dir, manifest.as_ref()).await;
        let (manifest_video_missing, manifest_video_zero_bytes) =
            manifest_video_path_flags(&run_dir, manifest.as_ref()).await;

        let total = manifest
            .as_ref()
            .and_then(|x| x.summary.as_ref())
            .map(|x| x.total)
            .unwrap_or(0);
        let passed = manifest
            .as_ref()
            .and_then(|x| x.summary.as_ref())
            .map(|x| x.passed)
            .unwrap_or(0);
        let failed = manifest
            .as_ref()
            .and_then(|x| x.summary.as_ref())
            .map(|x| x.failed)
            .unwrap_or(0);
        let skipped = manifest
            .as_ref()
            .and_then(|x| x.summary.as_ref())
            .map(|x| x.skipped)
            .unwrap_or(0);
        let blocked = manifest
            .as_ref()
            .and_then(|x| x.summary.as_ref())
            .map(|x| x.blocked)
            .unwrap_or(0);

        let pass_rate = if total > 0 {
            ((passed as f64) * 100.0) / (total as f64)
        } else {
            0.0
        };

        let mut issue = Vec::new();
        if manifest.is_none() {
            issue.push("missing manifest.json");
        }
        if !has_index {
            issue.push("missing index.html");
        }
        if !has_screenshots_dir {
            issue.push("missing screenshots/ directory");
        } else if screenshot_stats.files == 0 {
            issue.push("no screenshot files");
        } else if screenshot_stats.zero_bytes > 0 {
            issue.push("screenshot artifact has zero-byte file(s)");
        }
        if missing_manifest_screenshot_files > 0 {
            issue.push("manifest screenshots[] reference missing file(s)");
        }
        if !has_video_dir {
            issue.push("missing video/ directory");
        } else if video_stats.files == 0 {
            issue.push("no video files");
        } else if video_stats.zero_bytes > 0 {
            issue.push("video artifact has zero-byte file(s)");
        }
        if manifest_video_missing {
            issue.push("manifest video.path is missing or points to a missing file");
        } else if manifest_video_zero_bytes {
            issue.push("manifest video.path points to zero-byte file");
        }
        if !tokio::fs::try_exists(&coverage_path).await.unwrap_or(false) {
            issue.push("missing coverage.json");
        }
        if total > 0 && total != passed + failed + skipped + blocked {
            issue.push("summary totals do not add up");
        }

        let raw_status = manifest
            .as_ref()
            .and_then(|x| x.status.clone())
            .unwrap_or_else(|| {
                if failed > 0 {
                    "fail".to_string()
                } else if blocked > 0 {
                    "partial".to_string()
                } else if total > 0 && passed == total {
                    "pass".to_string()
                } else {
                    "partial".to_string()
                }
            });

        let (tested_count, tested_scope, quality_warning) = build_tested_scope_and_quality(
            manifest.as_ref(),
            coverage.as_ref(),
            report_html.as_deref(),
            total,
            passed,
            failed,
            skipped,
            blocked,
            &raw_status,
            &screenshot_stats,
            &video_stats,
            missing_manifest_screenshot_files,
            manifest_video_missing,
            manifest_video_zero_bytes,
        );

        let status = normalize_report_status(
            &raw_status,
            total,
            passed,
            failed,
            skipped,
            blocked,
            !issue.is_empty(),
            quality_warning.is_some(),
        );

        out.push(TestReportRun {
            run_id: run_id.clone(),
            created_at: manifest.as_ref().and_then(|x| x.created_at.clone()),
            status,
            pass_rate,
            total,
            passed,
            failed,
            skipped,
            blocked,
            issue: if issue.is_empty() {
                None
            } else {
                Some(issue.join(", "))
            },
            html_url: if has_index {
                Some(format!("/test-reports/{}/{}/index.html", ws.name, run_id))
            } else {
                None
            },
            tested_count,
            tested_scope,
            quality_warning,
            screenshot_files: screenshot_stats.files,
            video_files: video_stats.files,
        });
    }

    out.sort_by(|a, b| b.run_id.cmp(&a.run_id));
    out
}

fn build_tested_scope_and_quality(
    manifest: Option<&TestReportManifest>,
    coverage: Option<&TestReportCoverage>,
    report_html: Option<&str>,
    total: u64,
    passed: u64,
    failed: u64,
    skipped: u64,
    blocked: u64,
    status: &str,
    screenshot_stats: &ArtifactStats,
    video_stats: &ArtifactStats,
    missing_manifest_screenshot_files: u64,
    manifest_video_missing: bool,
    manifest_video_zero_bytes: bool,
) -> (u64, Option<String>, Option<String>) {
    let mut quality = Vec::new();

    let coverage_scope = coverage.map(|x| {
        format!(
            "routes {}/{}, buttons {}/{}, forms {}/{}, functional {}/{}",
            x.route_covered,
            x.route_total,
            x.button_covered,
            x.button_total,
            x.form_covered,
            x.form_total,
            x.functional_covered,
            x.functional_total
        )
    });

    if let Some(coverage) = coverage {
        if coverage.route_total == 0 {
            quality.push("coverage route_total is zero".to_string());
        }
        if coverage.route_covered < coverage.route_total {
            quality.push("coverage route_covered is less than route_total".to_string());
        }
        if coverage.button_covered > coverage.button_total {
            quality.push("coverage button_covered exceeds button_total".to_string());
        }
        if coverage.form_covered > coverage.form_total {
            quality.push("coverage form_covered exceeds form_total".to_string());
        }
        if coverage.functional_covered > coverage.functional_total {
            quality.push("coverage functional_covered exceeds functional_total".to_string());
        }
        if coverage.button_total == 0 || coverage.form_total == 0 {
            quality.push("button/form coverage totals are zero".to_string());
        }
        if coverage.functional_total == 0 {
            quality.push("functional coverage total is zero".to_string());
        }
        if coverage.functional_covered < coverage.functional_total {
            quality.push("coverage functional_covered is less than functional_total".to_string());
        }
    } else {
        quality.push("missing coverage.json".to_string());
    }

    if total > 0 && total != passed + failed + skipped + blocked {
        quality.push("summary totals do not add up".to_string());
    }
    if blocked > 0 {
        quality.push(format!("{} test(s) blocked", blocked));
    }

    if passed > 0 && screenshot_stats.files == 0 {
        quality.push("passes reported without screenshot files".to_string());
    }
    if screenshot_stats.zero_bytes > 0 {
        quality.push(format!(
            "{} screenshot file(s) are zero bytes",
            screenshot_stats.zero_bytes
        ));
    }
    if video_stats.files == 0 {
        quality.push("missing video files".to_string());
    }
    if video_stats.zero_bytes > 0 {
        quality.push(format!(
            "{} video file(s) are zero bytes",
            video_stats.zero_bytes
        ));
    }
    if missing_manifest_screenshot_files > 0 {
        quality.push(format!(
            "manifest screenshots[] has {} missing file reference(s)",
            missing_manifest_screenshot_files
        ));
    }
    if manifest_video_missing {
        quality.push("manifest video.path is missing or invalid".to_string());
    }
    if manifest_video_zero_bytes {
        quality.push("manifest video.path points to zero-byte file".to_string());
    }
    if report_html
        .map(contains_video_placeholder_keyword)
        .unwrap_or(false)
    {
        quality.push("report HTML contains video placeholder text".to_string());
    }

    let Some(manifest) = manifest else {
        let tested_scope = if let Some(scope) = coverage_scope {
            Some(scope)
        } else if total > 0 {
            Some(format!("{} summarized checks (no per-test list)", total))
        } else {
            None
        };
        return (
            total,
            tested_scope,
            if quality.is_empty() {
                None
            } else {
                Some(quality.join(", "))
            },
        );
    };

    let tested_count = if manifest.tests.is_empty() {
        total
    } else {
        manifest.tests.len() as u64
    };

    let test_scope = if !manifest.tests.is_empty() {
        let labels: Vec<String> = manifest
            .tests
            .iter()
            .map(|case| {
                let id = case
                    .id
                    .as_deref()
                    .or(case.name.as_deref())
                    .unwrap_or("unknown");
                let st = case.status.as_deref().unwrap_or("unknown").to_lowercase();
                format!("{}:{}", id, st)
            })
            .collect();
        Some(compact_scope(&labels, 6))
    } else if total > 0 {
        Some(format!("{} summarized checks (no test IDs)", total))
    } else {
        None
    };

    let tested_scope = match (coverage_scope, test_scope) {
        (Some(a), Some(b)) => Some(format!("{} | {}", a, b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    if manifest.tests.is_empty() && total > 0 {
        quality.push("manifest has summary but no test list".to_string());
    }

    let functional_test_count = manifest
        .tests
        .iter()
        .filter(|case| {
            case.id
                .as_deref()
                .map(|id| id.trim().to_ascii_uppercase().starts_with("FUNC-"))
                .unwrap_or(false)
        })
        .count() as u64;

    if total > 0 && functional_test_count == 0 {
        quality.push("manifest has no FUNC-* functional test entries".to_string());
    }

    if let Some(coverage) = coverage {
        if coverage.functional_total > 0 && functional_test_count < coverage.functional_total {
            quality.push(format!(
                "manifest has fewer FUNC-* entries ({}) than coverage.functional_total ({})",
                functional_test_count, coverage.functional_total
            ));
        }
        if coverage.functional_total > 0 && functional_test_count > coverage.functional_total {
            quality.push(format!(
                "manifest has more FUNC-* entries ({}) than coverage.functional_total ({})",
                functional_test_count, coverage.functional_total
            ));
        }
    }

    if manifest.screenshots.is_empty() && passed > 0 {
        quality.push("manifest has passes but no screenshots[] entries".to_string());
    }

    let blocker_present = manifest
        .blocker
        .as_deref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if blocker_present {
        quality.push("manifest blocker is present".to_string());
    }
    if blocker_present && status.eq_ignore_ascii_case("pass") {
        quality.push("status is pass while blocker is present".to_string());
    }

    let pass_ids: Vec<String> = manifest
        .tests
        .iter()
        .filter(|case| {
            case.status
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("pass"))
                .unwrap_or(false)
        })
        .filter_map(|case| case.id.as_deref())
        .map(normalize_test_key)
        .collect();

    let mut screenshot_counts_by_test = std::collections::HashMap::<String, u64>::new();
    for shot in &manifest.screenshots {
        if let Some(test_id) = shot.test_id.as_deref() {
            let key = normalize_test_key(test_id);
            let count = screenshot_counts_by_test.entry(key).or_insert(0);
            *count += 1;
        }
    }

    let failed_or_blocked_missing_screenshot = manifest
        .tests
        .iter()
        .filter(|case| {
            case.status
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("fail") || v.eq_ignore_ascii_case("blocked"))
                .unwrap_or(false)
        })
        .filter_map(|case| case.id.as_deref())
        .map(normalize_test_key)
        .filter(|id| !screenshot_counts_by_test.contains_key(id))
        .count() as u64;
    if failed_or_blocked_missing_screenshot > 0 {
        quality.push(format!(
            "{} FAIL/BLOCKED test(s) are missing screenshot evidence",
            failed_or_blocked_missing_screenshot
        ));
    }

    let blocked_tests = manifest
        .tests
        .iter()
        .filter(|case| {
            case.status
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("blocked"))
                .unwrap_or(false)
        })
        .count() as u64;
    if blocked_tests > 0 && blocked == 0 {
        quality.push("tests[] has blocked entries but summary.blocked is zero".to_string());
    }

    let mut loading_count = 0u64;
    let mut error_screen_count = 0u64;
    let mut pass_unreliable_count = 0u64;
    let mut wrong_page_name_count = 0u64;
    let mut pass_with_error_text_count = 0u64;

    for case in &manifest.tests {
        let blob = format!(
            "{} {} {}",
            case.id.as_deref().unwrap_or_default(),
            case.name.as_deref().unwrap_or_default(),
            case.error.as_deref().unwrap_or_default()
        );

        let is_pass = case
            .status
            .as_deref()
            .map(|v| v.eq_ignore_ascii_case("pass"))
            .unwrap_or(false);

        if contains_wrong_page_keyword(&blob) {
            wrong_page_name_count += 1;
        }
        if is_pass && (contains_error_screen_keyword(&blob) || contains_loading_keyword(&blob)) {
            pass_with_error_text_count += 1;
        }
    }

    for shot in &manifest.screenshots {
        let blob = format!(
            "{} {} {}",
            shot.test_id.as_deref().unwrap_or_default(),
            shot.path.as_deref().unwrap_or_default(),
            shot.description.as_deref().unwrap_or_default()
        );

        let loading = contains_loading_keyword(&blob);
        let error_screen = contains_error_screen_keyword(&blob);
        let wrong_page_name = contains_wrong_page_keyword(&blob);

        if loading {
            loading_count += 1;
        }
        if error_screen {
            error_screen_count += 1;
        }
        if wrong_page_name {
            wrong_page_name_count += 1;
        }

        if let Some(test_id) = shot.test_id.as_deref() {
            let normalized = normalize_test_key(test_id);
            if pass_ids.iter().any(|x| x == &normalized)
                && (loading || error_screen || wrong_page_name)
            {
                pass_unreliable_count += 1;
            }
        }
    }

    if loading_count > 0 {
        quality.push(format!(
            "{} screenshot(s) mention loading/spinner state",
            loading_count
        ));
    }
    if error_screen_count > 0 {
        quality.push(format!(
            "{} screenshot(s) mention error/404 state",
            error_screen_count
        ));
    }
    if pass_unreliable_count > 0 {
        quality.push(format!(
            "{} PASS screenshot(s) look unreliable",
            pass_unreliable_count
        ));
    }
    if wrong_page_name_count > 0 {
        quality.push(format!(
            "{} reference(s) mention wrong page name",
            wrong_page_name_count
        ));
    }
    if pass_with_error_text_count > 0 {
        quality.push(format!(
            "{} PASS test entry/notes include error-like text",
            pass_with_error_text_count
        ));
    }

    let dashboard_auth_failures = manifest
        .tests
        .iter()
        .filter(|case| {
            case.status
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("fail") || v.eq_ignore_ascii_case("blocked"))
                .unwrap_or(false)
        })
        .map(|case| {
            format!(
                "{} {} {}",
                case.id.as_deref().unwrap_or_default(),
                case.name.as_deref().unwrap_or_default(),
                case.error.as_deref().unwrap_or_default()
            )
        })
        .filter(|blob| contains_dashboard_keyword(blob) && contains_auth_redirect_keyword(blob))
        .count() as u64;
    if dashboard_auth_failures > 0 {
        quality.push(format!(
            "{} dashboard/login failure(s) indicate auth redirect or unauthorized state",
            dashboard_auth_failures
        ));
    }

    (
        tested_count,
        tested_scope,
        if quality.is_empty() {
            None
        } else {
            Some(quality.join(", "))
        },
    )
}

fn normalize_report_status(
    raw_status: &str,
    total: u64,
    passed: u64,
    failed: u64,
    skipped: u64,
    blocked: u64,
    has_issue: bool,
    has_quality_warning: bool,
) -> String {
    let inferred = if failed > 0 {
        "fail"
    } else if blocked > 0 {
        "partial"
    } else if total > 0 && passed == total {
        "pass"
    } else {
        "partial"
    };

    let status = match raw_status.trim().to_lowercase().as_str() {
        "" => inferred.to_string(),
        "completed" | "complete" | "done" | "success" | "ok" => inferred.to_string(),
        "failed" | "error" => "fail".to_string(),
        "blocked" => "blocked".to_string(),
        "running" | "in_progress" | "in-progress" | "queued" => "running".to_string(),
        "pass" | "fail" | "partial" => raw_status.trim().to_lowercase(),
        other => {
            if failed > 0 || blocked > 0 || (total > 0 && passed == total) {
                inferred.to_string()
            } else {
                other.to_string()
            }
        }
    };

    if !status.eq_ignore_ascii_case("pass") {
        return status;
    }

    let accounted = passed + failed + skipped + blocked;
    if has_issue
        || has_quality_warning
        || failed > 0
        || blocked > 0
        || (total > 0 && (accounted != total || passed < total))
    {
        if failed > 0 {
            return "fail".to_string();
        }
        return "partial".to_string();
    }

    status
}

fn compact_scope(items: &[String], limit: usize) -> String {
    if items.is_empty() {
        return String::new();
    }
    if items.len() <= limit {
        return items.join(", ");
    }
    let shown = items[..limit].join(", ");
    format!("{}, +{} more", shown, items.len() - limit)
}

fn contains_loading_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "loading",
        "still loading",
        "spinner",
        "skeleton",
        "loader",
        "shimmer",
        "placeholder",
        "waiting for page",
        "page not ready",
        "progress-only",
        "in progress",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn contains_error_screen_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "404",
        "not found",
        "cannot get",
        "failed",
        "error on",
        "-error",
        "connection refused",
        "service unavailable",
        "timeout",
        "500",
        "502",
        "503",
        "504",
        "internal server error",
        "error page",
        "blocked",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn contains_video_placeholder_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "video recording placeholder",
        "full video recording requires playwright video capture setup",
        "requires playwright video capture setup",
        "video placeholder",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn contains_wrong_page_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["junk-qr-payments", "junk qr payments", "junkqrpayments"]
        .iter()
        .any(|k| lower.contains(k))
}

fn contains_dashboard_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["dashboard", "home/dashboard", "home", "merchant portal"]
        .iter()
        .any(|k| lower.contains(k))
}

fn contains_auth_redirect_keyword(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "requires authentication",
        "redirected to login",
        "unauthorized",
        "forbidden",
        "401",
        "403",
        "session expired",
        "not authenticated",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

async fn manifest_missing_screenshot_files(
    run_dir: &StdPath,
    manifest: Option<&TestReportManifest>,
) -> u64 {
    let Some(manifest) = manifest else {
        return 0;
    };

    let mut missing = 0u64;
    for shot in &manifest.screenshots {
        let Some(path) = shot.path.as_deref() else {
            missing += 1;
            continue;
        };
        let trimmed = path.trim();
        if trimmed.is_empty() {
            missing += 1;
            continue;
        }
        if !tokio::fs::try_exists(&run_dir.join(trimmed))
            .await
            .unwrap_or(false)
        {
            missing += 1;
        }
    }

    missing
}

async fn manifest_video_path_flags(
    run_dir: &StdPath,
    manifest: Option<&TestReportManifest>,
) -> (bool, bool) {
    let Some(manifest) = manifest else {
        return (false, false);
    };

    let Some(path) = manifest
        .video
        .as_ref()
        .and_then(|v| v.path.as_deref())
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return (true, false);
    };

    match tokio::fs::metadata(run_dir.join(path)).await {
        Ok(meta) => (false, meta.len() == 0),
        Err(_) => (true, false),
    }
}

fn normalize_test_key(value: &str) -> String {
    value.trim().to_lowercase().replace('_', "-")
}

async fn artifact_stats(dir: &StdPath, allowed_ext: &[&str]) -> ArtifactStats {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(v) => v,
        Err(_) => return ArtifactStats::default(),
    };

    let mut out = ArtifactStats::default();
    while let Ok(Some(item)) = entries.next_entry().await {
        let file_type = match item.file_type().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }

        let ext = item
            .path()
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_lowercase();

        if !allowed_ext.is_empty() && !allowed_ext.iter().any(|e| *e == ext) {
            continue;
        }

        out.files += 1;

        let size = item.metadata().await.map(|m| m.len()).unwrap_or(0);
        if size == 0 {
            out.zero_bytes += 1;
        }
    }

    out
}

fn is_test_run_id(v: &str) -> bool {
    Regex::new(r"^\d{4}-\d{2}-\d{2}\d{2}-\d{2}-\d{2}$")
        .map(|x| x.is_match(v))
        .unwrap_or(false)
}

fn safe_file_name(v: &str) -> bool {
    !v.is_empty() && !v.contains("..") && !v.contains('/') && !v.contains('\\')
}

async fn resolve_test_report_file(
    ctx: &AppContext,
    workspace: &str,
    run_id: &str,
    relative_path: &str,
) -> Result<PathBuf, AppError> {
    sync_state_with_workspace_dirs(ctx).await?;
    let ws = resolve_workspace_by_id_or_name(ctx, workspace)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{}' not found", workspace)))?;

    if !is_test_run_id(run_id) {
        return Err(AppError::BadRequest("invalid test run id".to_string()));
    }
    if relative_path.contains("..")
        || relative_path.starts_with('/')
        || relative_path.contains('\\')
    {
        return Err(AppError::BadRequest("invalid report path".to_string()));
    }

    let run_dir = ws.path.join("logs").join(run_id);
    if !run_dir.exists() {
        return Err(AppError::NotFound("report run not found".to_string()));
    }

    let target = run_dir.join(relative_path);
    if !target.exists() {
        return Err(AppError::NotFound("report file not found".to_string()));
    }

    Ok(target)
}

fn mime_for(path: &StdPath) -> &'static str {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn frontends_manifest_path(workspace_path: &StdPath) -> PathBuf {
    workspace_path.join(".opencode").join("frontends.json")
}

async fn declared_frontend_ports(workspace_path: &StdPath) -> Vec<u16> {
    let manifest_path = frontends_manifest_path(workspace_path);
    let Ok(raw) = tokio::fs::read_to_string(manifest_path).await else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<FrontendManifest>(&raw) else {
        return Vec::new();
    };

    let mut ports: Vec<u16> = manifest
        .frontends
        .into_iter()
        .map(|item| item.port)
        .filter(|port| *port != 0)
        .collect();
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn build_preview_links(
    tunnels: &[crate::state::Tunnel],
    declared_ports: &[u16],
) -> Vec<PreviewLink> {
    let mut by_port: BTreeMap<u16, Option<String>> = BTreeMap::new();

    for port in declared_ports {
        by_port.entry(*port).or_insert(None);
    }
    for tunnel in tunnels {
        by_port.insert(tunnel.local_port, tunnel.tunnel_url.clone());
    }

    by_port
        .into_iter()
        .map(|(port, public_url)| PreviewLink {
            local_port: port,
            local_url: format!("http://127.0.0.1:{}", port),
            public_url,
        })
        .collect()
}

async fn workspace_response_with_links(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
) -> WorkspaceResponse {
    let tunnels = ctx.state.list_tunnels(&ws.id).await;
    let declared_ports = declared_frontend_ports(&ws.path).await;
    let show_hosted_urls = ws.status == WorkspaceStatus::Running;
    let preview_urls = build_preview_links(&tunnels, &declared_ports)
        .into_iter()
        .map(|mut link| {
            if !show_hosted_urls {
                link.public_url = None;
            }
            link
        })
        .collect();

    let mut response = WorkspaceResponse::from(ws);
    response.preview_urls = preview_urls;
    response.browser_url = if show_hosted_urls {
        tunnels.iter().rev().find_map(|t| t.tunnel_url.clone())
    } else {
        None
    };
    response
}

async fn publish_declared_frontends(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
) -> (Vec<u16>, Vec<u16>) {
    let ports = declared_frontend_ports(&ws.path).await;
    if ports.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let existing_tunnels = ctx.state.list_tunnels(&ws.id).await;

    let tunnel_mgr = TunnelManager::new(ctx.config.clone(), ctx.supervisor.clone());
    let mut published = Vec::new();
    let mut skipped = Vec::new();

    for port in ports {
        if existing_tunnels.iter().any(|item| item.local_port == port) {
            tunnel_mgr.stop_tunnel(&ws.id, port).await;
            let _ = ctx.state.remove_tunnel(&ws.id, port).await;
        }

        match tunnel_mgr.start_tunnel(&ws.id, port).await {
            Ok(result) => {
                if ctx
                    .state
                    .add_tunnel(&ws.id, port, result.tunnel_url.clone())
                    .await
                    .is_ok()
                {
                    published.push(port);
                } else {
                    tunnel_mgr.stop_tunnel(&ws.id, port).await;
                    skipped.push(port);
                }
            }
            Err(err) => {
                warn!(workspace = %ws.name, port, error = %err, "failed to publish frontend tunnel");
                skipped.push(port);
            }
        }
    }

    (published, skipped)
}

async fn vm_info() -> Result<Json<VmInfoResponse>, AppError> {
    let hostname = run_command("hostname", &[])
        .await
        .unwrap_or_else(|| "unknown".to_string());

    let cpu_cores = run_command("nproc", &[])
        .await
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);

    let cpu_usage_percent = sample_cpu_usage_percent().await;

    let (memory_total_mb, memory_used_mb) = parse_memory_info(
        run_command("free", &["-m"])
            .await
            .unwrap_or_default()
            .as_str(),
    );

    let (disk_total_gb, disk_used_gb) = parse_disk_info(
        run_command("df", &["-BG", "/"])
            .await
            .unwrap_or_default()
            .as_str(),
    );

    let uptime_seconds = parse_uptime_seconds(
        run_command("cat", &["/proc/uptime"])
            .await
            .unwrap_or_default()
            .as_str(),
    );

    let os = parse_os_name(
        run_command("cat", &["/etc/os-release"])
            .await
            .unwrap_or_default()
            .as_str(),
    );

    Ok(Json(VmInfoResponse {
        hostname,
        cpu_cores,
        cpu_usage_percent,
        memory_total_mb,
        memory_used_mb,
        disk_total_gb,
        disk_used_gb,
        uptime_seconds,
        os,
    }))
}

async fn reset_password(
    State(ctx): State<AppContext>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>, AppError> {
    let user = req.user.trim();
    let new_password = req.new_password.trim();

    if user.is_empty() || new_password.is_empty() {
        return Err(AppError::BadRequest(
            "user and new_password are required".to_string(),
        ));
    }

    let wrapper_path = resolve_wrapper_script_path(&ctx.config).ok_or_else(|| {
        AppError::NotFound("could not locate daemon wrapper script (run-daemon.sh)".to_string())
    })?;

    let script_content = tokio::fs::read_to_string(&wrapper_path)
        .await
        .map_err(|err| {
            AppError::NotFound(format!(
                "failed to read wrapper script '{}': {}",
                wrapper_path.display(),
                err
            ))
        })?;

    let escaped_user = shell_escaped_double_quote(user);
    let escaped_pass = shell_escaped_double_quote(new_password);

    let updated = replace_wrapper_flag_value(&script_content, "ttyd-user", &escaped_user)?;
    let updated = replace_wrapper_flag_value(&updated, "ttyd-pass", &escaped_pass)?;

    tokio::fs::write(&wrapper_path, updated).await?;

    Ok(Json(ResetPasswordResponse {
        message:
            "Password updated in daemon wrapper. Restart uwu-daemon to apply ttyd credentials."
                .to_string(),
        wrapper_path: wrapper_path.to_string_lossy().to_string(),
    }))
}

async fn commander_send(
    State(ctx): State<AppContext>,
    Json(req): Json<CommanderSendRequest>,
) -> Result<Json<Message>, AppError> {
    let message = ctx.commander.send(req.message).await?;
    Ok(Json(message))
}

async fn commander_messages(
    State(ctx): State<AppContext>,
    Query(query): Query<CommanderMessagesQuery>,
) -> Result<Json<Vec<Message>>, AppError> {
    let since = query.since.unwrap_or(0);
    let _ = ctx.commander.poll_updates().await?;
    let messages = ctx.commander.messages_since(since).await;
    Ok(Json(messages))
}

async fn commander_sessions(
    State(ctx): State<AppContext>,
) -> Result<Json<Vec<SessionInfo>>, AppError> {
    let sessions = ctx.commander.list_sessions().await?;
    Ok(Json(sessions))
}

async fn commander_switch_session(
    State(ctx): State<AppContext>,
    Json(req): Json<CommanderSwitchRequest>,
) -> Result<Json<CommanderSwitchResponse>, AppError> {
    let target = ctx.commander.switch_session(req.target).await?;
    Ok(Json(CommanderSwitchResponse { target }))
}

async fn commander_capture(
    State(ctx): State<AppContext>,
) -> Result<Json<CaptureResponse>, AppError> {
    let capture = ctx.commander.capture().await?;
    Ok(Json(capture))
}

async fn run_command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_memory_info(free_output: &str) -> (u64, u64) {
    for line in free_output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Mem:") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let total = parts[1].parse::<u64>().unwrap_or(0);
                let used = parts[2].parse::<u64>().unwrap_or(0);
                return (total, used);
            }
        }
    }
    (0, 0)
}

fn parse_cpu_times(proc_stat: &str) -> Option<(u64, u64)> {
    let line = proc_stat.lines().find(|line| line.starts_with("cpu "))?;
    let mut fields = line.split_whitespace();
    let _ = fields.next();

    let values: Vec<u64> = fields
        .filter_map(|value| value.parse::<u64>().ok())
        .collect();
    if values.len() < 4 {
        return None;
    }

    let idle = values[3].saturating_add(*values.get(4).unwrap_or(&0));
    let total = values.iter().sum();
    Some((idle, total))
}

async fn sample_cpu_usage_percent() -> Option<f64> {
    let first = tokio::fs::read_to_string("/proc/stat").await.ok()?;
    let (idle1, total1) = parse_cpu_times(&first)?;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let second = tokio::fs::read_to_string("/proc/stat").await.ok()?;
    let (idle2, total2) = parse_cpu_times(&second)?;

    let total_delta = total2.saturating_sub(total1);
    if total_delta == 0 {
        return None;
    }

    let idle_delta = idle2.saturating_sub(idle1);
    let busy_delta = total_delta.saturating_sub(idle_delta);
    let percent = (busy_delta as f64 / total_delta as f64) * 100.0;
    Some((percent * 10.0).round() / 10.0)
}

fn parse_disk_info(df_output: &str) -> (u64, u64) {
    let mut lines = df_output.lines();
    let _ = lines.next();

    if let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let total = parts[1].trim_end_matches('G').parse::<u64>().unwrap_or(0);
            let used = parts[2].trim_end_matches('G').parse::<u64>().unwrap_or(0);
            return (total, used);
        }
    }

    (0, 0)
}

fn parse_uptime_seconds(uptime_output: &str) -> u64 {
    uptime_output
        .split_whitespace()
        .next()
        .and_then(|part| part.parse::<f64>().ok())
        .map(|seconds| seconds as u64)
        .unwrap_or(0)
}

fn parse_os_name(os_release: &str) -> String {
    for line in os_release.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return value.trim_matches('"').to_string();
        }
    }
    "unknown".to_string()
}

fn resolve_static_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(release_dir) = exe.parent() {
            if let Some(target_dir) = release_dir.parent() {
                if let Some(daemon_dir) = target_dir.parent() {
                    let candidate = daemon_dir.join("static");
                    if candidate.exists() {
                        return candidate;
                    }
                }
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let direct = cwd.join("static");
        if direct.exists() {
            return direct;
        }
        let nested = cwd.join("daemon").join("static");
        if nested.exists() {
            return nested;
        }
    }

    PathBuf::from("static")
}

fn resolve_wrapper_script_path(config: &AppConfig) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(release_dir) = exe.parent() {
            if let Some(target_dir) = release_dir.parent() {
                if let Some(daemon_dir) = target_dir.parent() {
                    if let Some(repo_root) = daemon_dir.parent() {
                        candidates.push(repo_root.join("scripts").join("run-daemon.sh"));
                    }
                }
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("scripts").join("run-daemon.sh"));
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("scripts").join("run-daemon.sh"));
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(
            PathBuf::from(home)
                .join("uwu-my-opencode")
                .join("scripts")
                .join("run-daemon.sh"),
        );
    }

    if let Some(daemon_dir) = config.workspace_root.parent() {
        candidates.push(
            daemon_dir
                .join("uwu-my-opencode")
                .join("scripts")
                .join("run-daemon.sh"),
        );
    }

    candidates.into_iter().find(|path| path.exists())
}

fn replace_wrapper_flag_value(
    content: &str,
    flag_name: &str,
    value: &str,
) -> Result<String, AppError> {
    let re = Regex::new(&format!(
        r#"(?m)(--{}\s+")(.*?)("\s*\\?)$"#,
        regex::escape(flag_name)
    ))
    .map_err(|err| AppError::BadRequest(format!("invalid replacement regex: {}", err)))?;

    if !re.is_match(content) {
        return Err(AppError::NotFound(format!(
            "flag '--{}' not found in wrapper script",
            flag_name
        )));
    }

    Ok(re
        .replace_all(content, format!("${{1}}{}${{3}}", value))
        .to_string())
}

fn shell_escaped_double_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
