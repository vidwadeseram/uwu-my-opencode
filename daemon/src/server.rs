use axum::{
    extract::Path,
    extract::Query,
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{delete, get, post},
    Json, Router,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path as StdPath, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

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
    pub size_mb: Option<u64>,
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
        let browser_url = ws
            .ttyd_port
            .map(|port| format!("http://127.0.0.1:{}", port));
        let terminal_url = ws
            .ttyd_port
            .map(|_| format!("/terminal/?arg=attach&arg=-t&arg=ws-{}", ws.name));

        Self {
            id: ws.id.clone(),
            name: ws.name.clone(),
            path: ws.path.to_string_lossy().to_string(),
            opencode_port: ws.opencode_port,
            ttyd_port: ws.ttyd_port,
            status: format!("{:?}", ws.status).to_lowercase(),
            created_at: ws.created_at.to_rfc3339(),
            browser_url,
            terminal_url,
            size_mb: None,
        }
    }
}

#[derive(Serialize)]
struct VmInfoResponse {
    hostname: String,
    cpu_cores: u32,
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
            "/api/workspaces/{id}/previews",
            get(list_previews)
                .post(create_preview)
                .delete(delete_preview),
        )
        .route("/api/projects", get(list_workspaces))
        .route("/api/projects/{id}/start", post(start_workspace))
        .route("/api/projects/{id}/stop", post(stop_workspace))
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
    let mut responses = Vec::with_capacity(workspaces.len());

    for ws in &workspaces {
        let mut response = WorkspaceResponse::from(ws);
        response.size_mb = Some(directory_size_bytes(&ws.path).await / (1024 * 1024));
        responses.push(response);
    }

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

async fn vm_info() -> Result<Json<VmInfoResponse>, AppError> {
    let hostname = run_command("hostname", &[])
        .await
        .unwrap_or_else(|| "unknown".to_string());

    let cpu_cores = run_command("nproc", &[])
        .await
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);

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
