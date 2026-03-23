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

#[derive(Deserialize)]
struct FrontendManifest {
    #[serde(default)]
    frontends: Vec<FrontendDefinition>,
}

#[derive(Deserialize)]
struct FrontendDefinition {
    port: u16,
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
        .route("/commander", get(commander_index))
        .route("/logout", get(logout))
        .nest_service("/static", ServeDir::new(static_dir))
        .route("/api/vm", get(vm_info))
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
        response.size_mb = Some(directory_size_bytes(&ws.path).await / (1024 * 1024));
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

async fn directory_size_bytes(path: &StdPath) -> u64 {
    let mut total = 0_u64;
    let mut stack = vec![PathBuf::from(path)];

    while let Some(current) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&current).await {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            match entry.metadata().await {
                Ok(metadata) if metadata.is_file() => {
                    total = total.saturating_add(metadata.len());
                }
                Ok(metadata) if metadata.is_dir() => stack.push(entry_path),
                _ => {}
            }
        }
    }

    total
}
