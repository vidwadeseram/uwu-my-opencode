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
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path as StdPath, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio_postgres::NoTls;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

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
    pub run_locks: Arc<Mutex<HashSet<String>>>,
    pub run_processes: Arc<Mutex<HashMap<String, u32>>>,
}

struct WorkspaceRunLockGuard {
    workspace_id: String,
    locks: Arc<Mutex<HashSet<String>>>,
    run_processes: Arc<Mutex<HashMap<String, u32>>>,
}

impl Drop for WorkspaceRunLockGuard {
    fn drop(&mut self) {
        if let Ok(mut locks) = self.locks.lock() {
            locks.remove(&self.workspace_id);
        }
        if let Ok(mut run_processes) = self.run_processes.lock() {
            run_processes.remove(&self.workspace_id);
        }
    }
}

struct WorkspaceRunProcessGuard {
    workspace_id: String,
    run_processes: Arc<Mutex<HashMap<String, u32>>>,
}

impl Drop for WorkspaceRunProcessGuard {
    fn drop(&mut self) {
        if let Ok(mut run_processes) = self.run_processes.lock() {
            run_processes.remove(&self.workspace_id);
        }
    }
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
    pub repos: Vec<WorkspaceRepo>,
    pub target: WorkspaceTargetContextResponse,
    pub structure: WorkspaceStructureStatus,
    pub init_last_status: Option<String>,
    pub init_last_message: Option<String>,
    pub init_last_at: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct WorkspaceRepo {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Clone, Default)]
pub struct WorkspaceTargetContextResponse {
    pub repo: Option<String>,
}

#[derive(Serialize, Clone, Default)]
pub struct WorkspaceStructureStatus {
    pub ready: bool,
    pub stale: bool,
    pub missing: Vec<String>,
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
pub struct SetWorkspaceTargetRequest {
    pub repo: Option<String>,
}

#[derive(Serialize)]
pub struct InitWorkspaceResponse {
    pub workspace: WorkspaceResponse,
    pub created_paths: Vec<String>,
}

#[derive(Deserialize)]
pub struct ToonValidationRequest {
    pub file: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ToonValidationIssue {
    pub code: String,
    pub path: String,
    pub reason: String,
    pub hint: String,
}

#[derive(Serialize)]
pub struct ToonValidationResponse {
    pub valid: bool,
    pub errors: Vec<ToonValidationIssue>,
}

#[derive(Deserialize)]
pub struct RunToonSuiteRequest {
    pub suite_file: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default = "default_true")]
    pub ensure_workspace_started: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct RunToonSuiteResponse {
    pub run_id: String,
    pub status: String,
    pub report_url: String,
    pub artifacts_dir: String,
    pub total_cases: u64,
    pub passed_cases: u64,
    pub failed_cases: u64,
    pub blocked_cases: u64,
}

#[derive(Serialize)]
pub struct CancelToonRunResponse {
    pub workspace_id: String,
    pub cancelled: bool,
    pub message: String,
}

#[derive(Deserialize, Default)]
pub struct FillToonDataRequest {
    #[serde(default)]
    pub include_repo_paths: bool,
}

#[derive(Serialize)]
pub struct FillToonDataResponse {
    pub workspace: WorkspaceResponse,
    pub target_root: String,
    pub scanned_repos: usize,
    pub updated_files: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct InferToonCasesRequest {
    #[serde(default)]
    pub max_cases: Option<usize>,
    #[serde(default)]
    pub dispatch_to_commander: bool,
}

#[derive(Serialize, Clone)]
pub struct InferredToonCase {
    pub id: String,
    pub title: String,
    pub method: String,
    pub path: String,
    pub source_repo: String,
    pub source_file: String,
    pub generated_case_file: String,
}

#[derive(Serialize)]
pub struct InferToonCasesResponse {
    pub workspace: WorkspaceResponse,
    pub target_root: String,
    pub scanned_repos: usize,
    pub inferred_cases: Vec<InferredToonCase>,
    pub updated_files: Vec<String>,
    pub prompt: String,
    pub commander_dispatched: bool,
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
            repos: Vec::new(),
            target: WorkspaceTargetContextResponse {
                repo: ws.target.repo.clone(),
            },
            structure: WorkspaceStructureStatus::default(),
            init_last_status: ws.init_last_status.clone(),
            init_last_message: ws.init_last_message.clone(),
            init_last_at: ws.init_last_at.as_ref().map(|v| v.to_rfc3339()),
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
        .route("/api/workspaces/{id}/init", post(init_workspace))
        .route("/api/workspaces/{id}/target", post(set_workspace_target))
        .route(
            "/api/workspaces/{id}/toon/validate",
            post(validate_toon_payload),
        )
        .route("/api/workspaces/{id}/toon/fill", post(fill_toon_data))
        .route("/api/workspaces/{id}/toon/infer", post(infer_toon_cases))
        .route("/api/workspaces/{id}/toon/run", post(run_toon_suite))
        .route("/api/workspaces/{id}/toon/cancel", post(cancel_toon_run))
        .route(
            "/api/mcp/workspaces/{id}/toon/validate",
            post(validate_toon_payload),
        )
        .route("/api/mcp/workspaces/{id}/toon/fill", post(fill_toon_data))
        .route(
            "/api/mcp/workspaces/{id}/toon/infer",
            post(infer_toon_cases),
        )
        .route("/api/mcp/workspaces/{id}/toon/run", post(run_toon_suite))
        .route(
            "/api/mcp/workspaces/{id}/toon/cancel",
            post(cancel_toon_run),
        )
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
        .route("/api/projects/{id}/init", post(init_workspace))
        .route("/api/projects/{id}/target", post(set_workspace_target))
        .route(
            "/api/projects/{id}/toon/validate",
            post(validate_toon_payload),
        )
        .route("/api/projects/{id}/toon/fill", post(fill_toon_data))
        .route("/api/projects/{id}/toon/infer", post(infer_toon_cases))
        .route("/api/projects/{id}/toon/run", post(run_toon_suite))
        .route("/api/projects/{id}/toon/cancel", post(cancel_toon_run))
        .route(
            "/api/mcp/projects/{id}/toon/validate",
            post(validate_toon_payload),
        )
        .route("/api/mcp/projects/{id}/toon/fill", post(fill_toon_data))
        .route("/api/mcp/projects/{id}/toon/infer", post(infer_toon_cases))
        .route("/api/mcp/projects/{id}/toon/run", post(run_toon_suite))
        .route("/api/mcp/projects/{id}/toon/cancel", post(cancel_toon_run))
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
    let static_dir = resolve_static_dir();
    let template_path = static_dir.join("test-reports.template.html");
    let fallback_path = static_dir.join("test-reports.html");

    let html = match tokio::fs::read_to_string(&template_path).await {
        Ok(template) => template
            .replace("[[REPORTS_API]]", "/api/test-reports")
            .replace("[[REPORTS_TITLE]]", "Workspace Test Reports"),
        Err(_) => tokio::fs::read_to_string(&fallback_path)
            .await
            .map_err(|err| {
                AppError::NotFound(format!(
                    "test reports page not found at '{}' or '{}': {}",
                    template_path.display(),
                    fallback_path.display(),
                    err
                ))
            })?,
    };
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

    let _ = ctx
        .state
        .record_workspace_init(
            &ws.id,
            "success",
            Some("workspace scaffold initialized".to_string()),
        )
        .await;
    let ws = resolve_workspace_by_id_or_name(&ctx, &ws.id)
        .await?
        .ok_or_else(|| AppError::NotFound("workspace not found after create".to_string()))?;
    let response = workspace_response_with_links(&ctx, &ws).await;

    Ok((StatusCode::CREATED, Json(response)))
}

async fn init_workspace(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<InitWorkspaceResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;
    let init_root = resolve_workspace_target_root(&ws)
        .await?
        .unwrap_or_else(|| ws.path.clone());

    let before = workspace_structure_status(&init_root).await;
    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    match manager.setup_workspace_opencode_files(&init_root).await {
        Ok(()) => {
            let _ = ctx
                .state
                .record_workspace_init(
                    &ws.id,
                    "success",
                    Some(format!(
                        "workspace scaffold initialized at {}",
                        init_root.display()
                    )),
                )
                .await;
        }
        Err(err) => {
            let _ = ctx
                .state
                .record_workspace_init(&ws.id, "failed", Some(err.to_string()))
                .await;
            return Err(err);
        }
    }

    let current = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;
    let current_root = resolve_workspace_target_root(&current)
        .await?
        .unwrap_or_else(|| current.path.clone());
    let after = workspace_structure_status(&current_root).await;
    let mut created_paths = Vec::new();
    for item in before.missing {
        if !after.missing.contains(&item) {
            created_paths.push(item);
        }
    }

    let workspace = workspace_response_with_links(&ctx, &current).await;
    Ok(Json(InitWorkspaceResponse {
        workspace,
        created_paths,
    }))
}

fn sanitize_toon_scalar(input: &str) -> String {
    input
        .replace(',', "_")
        .replace('\n', " ")
        .replace('\r', " ")
        .trim()
        .to_string()
}

fn upsert_markdown_generated_section(
    existing: &str,
    start_marker: &str,
    end_marker: &str,
    generated: &str,
) -> String {
    if let (Some(start), Some(end)) = (existing.find(start_marker), existing.find(end_marker)) {
        if end > start {
            let mut out = String::new();
            out.push_str(&existing[..start]);
            out.push_str(generated);
            out.push_str(&existing[end + end_marker.len()..]);
            return out;
        }
    }

    let mut out = existing.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(generated);
    out.push('\n');
    out
}

async fn fill_toon_data(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<FillToonDataRequest>,
) -> Result<Json<FillToonDataResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let target_root = resolve_workspace_target_root(&ws).await?.ok_or_else(|| {
        AppError::BadRequest(
            "set Repo/Test Context Repository first, then run toon fill".to_string(),
        )
    })?;

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    manager.setup_workspace_opencode_files(&target_root).await?;

    let repos = discover_workspace_repos(&ws.path).await;
    let selected_repo_value = ws
        .target
        .repo
        .as_ref()
        .map(|v| sanitize_toon_scalar(v))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "default".to_string());

    let mut repo_items: Vec<String> = repos
        .iter()
        .map(|r| {
            if req.include_repo_paths {
                sanitize_toon_scalar(&format!("{}|{}", r.name, r.path))
            } else {
                sanitize_toon_scalar(&r.name)
            }
        })
        .filter(|v| !v.is_empty())
        .collect();
    repo_items.sort();
    repo_items.dedup();
    if repo_items.is_empty() {
        repo_items.push(sanitize_toon_scalar(
            target_root
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("selected-repo"),
        ));
    }

    let repos_collection = format!("repos[{}]: {}", repo_items.len(), repo_items.join(","));

    let setup_toon_content = format!(
        "context:\n  kind: setup\n  version: 1.0\n  task: bootstrap selected repository\n  repo: {repo}\n{repos_collection}\ncommands[2]{{id,run,required}}:\n  1,install_dependencies,true\n  2,start_services,true\nchecks[1]{{id,run,expect}}:\n  1,curl -fsS http://127.0.0.1:8080/health,ok\n",
        repo = selected_repo_value,
        repos_collection = repos_collection
    );

    let tests_toon_content = format!(
        "context:\n  kind: suite\n  version: 1.0\n  task: smoke suite\n  repo: {repo}\ncases[1]{{id,file,required}}:\n  1,test_cases/login-flow.toon,true\nartifacts[2]: manifest,coverage\n{repos_collection}\n",
        repo = selected_repo_value,
        repos_collection = repos_collection
    );

    let generated_at = chrono::Utc::now().to_rfc3339();
    let repo_lines = repos
        .iter()
        .map(|r| format!("- {} ({})", r.name, r.path))
        .collect::<Vec<_>>()
        .join("\n");

    let generated_section = format!(
        "<!-- AUTO_REPO_SCAN_START -->\n## Auto Repository Scan\nGenerated at: {generated_at}\n\nSelected Repo Context: `{selected}`\n\nDiscovered repositories ({count}):\n{repo_lines}\n<!-- AUTO_REPO_SCAN_END -->",
        generated_at = generated_at,
        selected = target_root.display(),
        count = repos.len(),
        repo_lines = repo_lines
    );

    let setup_md_path = target_root.join("SETUP.md");
    let test_md_path = target_root.join("TEST.md");
    let setup_toon_path = target_root.join("setup").join("default.toon");
    let tests_toon_path = target_root.join("tests").join("smoke.toon");

    let setup_md_existing = tokio::fs::read_to_string(&setup_md_path)
        .await
        .unwrap_or_default();
    let test_md_existing = tokio::fs::read_to_string(&test_md_path)
        .await
        .unwrap_or_default();
    let setup_md_updated = upsert_markdown_generated_section(
        &setup_md_existing,
        "<!-- AUTO_REPO_SCAN_START -->",
        "<!-- AUTO_REPO_SCAN_END -->",
        &generated_section,
    );
    let test_md_updated = upsert_markdown_generated_section(
        &test_md_existing,
        "<!-- AUTO_REPO_SCAN_START -->",
        "<!-- AUTO_REPO_SCAN_END -->",
        &generated_section,
    );

    tokio::fs::write(&setup_toon_path, setup_toon_content).await?;
    tokio::fs::write(&tests_toon_path, tests_toon_content).await?;
    tokio::fs::write(&setup_md_path, setup_md_updated).await?;
    tokio::fs::write(&test_md_path, test_md_updated).await?;

    let current = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;
    let workspace = workspace_response_with_links(&ctx, &current).await;

    Ok(Json(FillToonDataResponse {
        workspace,
        target_root: target_root.display().to_string(),
        scanned_repos: repos.len(),
        updated_files: vec![
            "SETUP.md".to_string(),
            "TEST.md".to_string(),
            "setup/default.toon".to_string(),
            "tests/smoke.toon".to_string(),
        ],
    }))
}

#[derive(Clone)]
struct RouteCandidate {
    method: String,
    path: String,
    source_repo: String,
    source_file: String,
}

fn normalize_inferred_path(raw: &str) -> Option<String> {
    let mut path = raw.trim().to_string();
    if path.is_empty() {
        return None;
    }
    if let Some((head, _)) = path.split_once('?') {
        path = head.to_string();
    }
    if !path.starts_with('/') {
        path = format!("/{}", path);
    }
    path = path.replace('{', "").replace('}', "");
    let param_re = Regex::new(r":([A-Za-z_][A-Za-z0-9_-]*)").ok()?;
    path = param_re.replace_all(&path, "1").to_string();
    let bracket_re = Regex::new(r"\[([A-Za-z_][A-Za-z0-9_-]*)\]").ok()?;
    path = bracket_re.replace_all(&path, "1").to_string();
    while path.contains("//") {
        path = path.replace("//", "/");
    }
    if path.len() > 180 {
        path.truncate(180);
    }
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn route_title(repo: &str, method: &str, path: &str) -> String {
    let business_hint = if path.contains("payment") {
        "payment flow"
    } else if path.contains("inventory") {
        "inventory flow"
    } else if path.contains("customer") {
        "customer flow"
    } else if path.contains("auth") || path.contains("identity") {
        "identity flow"
    } else if path.contains("loyalty") {
        "loyalty flow"
    } else {
        "core route"
    };
    format!("{} {} {} ({})", method, path, business_hint, repo)
}

fn push_route_candidate(
    out: &mut Vec<RouteCandidate>,
    repo_name: &str,
    source_file: &str,
    method: &str,
    raw_path: &str,
) {
    if let Some(path) = normalize_inferred_path(raw_path) {
        out.push(RouteCandidate {
            method: method.trim().to_ascii_uppercase(),
            path,
            source_repo: repo_name.to_string(),
            source_file: source_file.to_string(),
        });
    }
}

fn infer_method_from_context(context: &str) -> String {
    let lower = context.to_ascii_lowercase();
    if lower.contains("post")
        || lower.contains("create")
        || lower.contains("signup")
        || lower.contains("register")
        || lower.contains("login")
    {
        "POST".to_string()
    } else if lower.contains("put") || lower.contains("update") {
        "PUT".to_string()
    } else if lower.contains("delete") || lower.contains("remove") {
        "DELETE".to_string()
    } else if lower.contains("patch") {
        "PATCH".to_string()
    } else {
        "GET".to_string()
    }
}

fn route_priority_score(route: &RouteCandidate) -> i32 {
    let path = route.path.to_ascii_lowercase();
    let source = format!(
        "{} {}",
        route.source_repo.to_ascii_lowercase(),
        route.source_file.to_ascii_lowercase()
    );

    let mut score = 0i32;
    if path.starts_with("/api") {
        score += 8;
    }
    if route.method == "POST" {
        score += 6;
    }
    if route.method == "PUT" || route.method == "PATCH" {
        score += 4;
    }

    for (kw, weight) in [
        ("login", 40),
        ("signup", 40),
        ("register", 35),
        ("kyc", 40),
        ("otp", 30),
        ("verify", 25),
        ("password", 25),
        ("auth", 30),
        ("identity", 25),
        ("customer", 20),
        ("inventory", 20),
        ("payment", 30),
        ("checkout", 25),
        ("order", 25),
        ("refund", 25),
        ("wallet", 20),
        ("loyalty", 20),
    ] {
        if path.contains(kw) || source.contains(kw) {
            score += weight;
        }
    }

    if path.matches('/').count() > 3 {
        score += 3;
    }

    score
}

fn extract_openapi_route_candidates(
    repo_name: &str,
    source_file: &str,
    content: &str,
) -> Vec<RouteCandidate> {
    let mut out = Vec::new();
    let path_line_re = Regex::new(r"^\s{0,14}(/[^:\s]+):\s*$").expect("valid openapi path regex");
    let method_line_re = Regex::new(r"^\s{2,18}(get|post|put|delete|patch):\s*$")
        .expect("valid openapi method regex");

    let mut in_paths_block = false;
    let mut current_path: Option<String> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "paths:" {
            in_paths_block = true;
            current_path = None;
            continue;
        }
        if !in_paths_block {
            continue;
        }

        if !line.starts_with(' ') && !trimmed.is_empty() && trimmed != "paths:" {
            in_paths_block = false;
            current_path = None;
            continue;
        }

        if let Some(caps) = path_line_re.captures(line) {
            let path = caps.get(1).map(|v| v.as_str()).unwrap_or_default();
            current_path = normalize_inferred_path(path);
            continue;
        }

        if let Some(caps) = method_line_re.captures(line) {
            let method = caps.get(1).map(|v| v.as_str()).unwrap_or("get");
            if let Some(path) = current_path.as_deref() {
                push_route_candidate(&mut out, repo_name, source_file, method, path);
            }
        }
    }

    out
}

fn extract_route_candidates_from_content(
    repo_name: &str,
    source_file: &str,
    content: &str,
) -> Vec<RouteCandidate> {
    let mut out = Vec::new();
    let axum_re = Regex::new(r#"\.route\(\s*\"([^\"]+)\"\s*,\s*(get|post|put|delete|patch)"#)
        .expect("valid axum route regex");
    let express_re =
        Regex::new(r#"(?:app|router)\.(get|post|put|delete|patch)\(\s*['\"]([^'\"]+)['\"]"#)
            .expect("valid express route regex");
    let fastapi_re =
        Regex::new(r#"@(?:app|router)\.(get|post|put|delete|patch)\(\s*['\"]([^'\"]+)['\"]"#)
            .expect("valid fastapi route regex");
    let spring_mapping_re = Regex::new(
        r#"@(?P<ann>GetMapping|PostMapping|PutMapping|DeleteMapping|PatchMapping)\(\s*(?P<args>[^)]*)\)"#,
    )
    .expect("valid spring mapping regex");
    let spring_request_re = Regex::new(r#"@RequestMapping\(\s*(?P<args>[^)]*)\)"#)
        .expect("valid request mapping regex");
    let spring_method_re = Regex::new(r#"RequestMethod\.(GET|POST|PUT|DELETE|PATCH)"#)
        .expect("valid request method regex");
    let literal_re = Regex::new(r#"['\"]([^'\"]+)['\"]"#).expect("valid literal regex");
    let gin_re =
        Regex::new(r#"(?:router|r|group)\.(GET|POST|PUT|DELETE|PATCH)\(\s*['\"]([^'\"]+)['\"]"#)
            .expect("valid gin route regex");
    let jaxrs_pair_re = Regex::new(
        r#"(?s)@(?P<method>GET|POST|PUT|DELETE|PATCH)\b.*?@Path\(\s*['\"](?P<path>[^'\"]+)['\"]\s*\)"#,
    )
    .expect("valid jaxrs pair regex");
    let quoted_route_re =
        Regex::new(r#"['\"](/[^'\"\s]{1,180})['\"]"#).expect("valid quoted route regex");
    let next_handler_re =
        Regex::new(r"export\s+(?:async\s+)?function\s+(GET|POST|PUT|DELETE|PATCH)")
            .expect("valid next handler regex");

    let source_file_lower = source_file.to_ascii_lowercase();
    let business_path_keywords = [
        "login",
        "signup",
        "register",
        "kyc",
        "auth",
        "identity",
        "otp",
        "customer",
        "inventory",
        "payment",
        "checkout",
        "order",
        "refund",
        "wallet",
        "loyalty",
    ];

    for caps in axum_re.captures_iter(content) {
        let path = caps.get(1).map(|v| v.as_str()).unwrap_or_default();
        let method = caps.get(2).map(|v| v.as_str()).unwrap_or("get");
        push_route_candidate(&mut out, repo_name, source_file, method, path);
    }
    for caps in express_re.captures_iter(content) {
        let method = caps.get(1).map(|v| v.as_str()).unwrap_or("get");
        let path = caps.get(2).map(|v| v.as_str()).unwrap_or_default();
        push_route_candidate(&mut out, repo_name, source_file, method, path);
    }
    for caps in fastapi_re.captures_iter(content) {
        let method = caps.get(1).map(|v| v.as_str()).unwrap_or("get");
        let path = caps.get(2).map(|v| v.as_str()).unwrap_or_default();
        push_route_candidate(&mut out, repo_name, source_file, method, path);
    }
    for caps in spring_mapping_re.captures_iter(content) {
        let ann = caps.name("ann").map(|v| v.as_str()).unwrap_or("GetMapping");
        let method = ann.trim_end_matches("Mapping");
        let args = caps.name("args").map(|v| v.as_str()).unwrap_or_default();
        let mut found = false;
        for lit in literal_re.captures_iter(args) {
            let path = lit.get(1).map(|v| v.as_str()).unwrap_or_default();
            if path.starts_with('/') {
                push_route_candidate(&mut out, repo_name, source_file, method, path);
                found = true;
            }
        }
        if !found {
            push_route_candidate(&mut out, repo_name, source_file, method, "/");
        }
    }
    for caps in spring_request_re.captures_iter(content) {
        let args = caps.name("args").map(|v| v.as_str()).unwrap_or_default();
        let method = spring_method_re
            .captures(args)
            .and_then(|m| m.get(1).map(|v| v.as_str()))
            .unwrap_or("GET");
        let mut found = false;
        for lit in literal_re.captures_iter(args) {
            let path = lit.get(1).map(|v| v.as_str()).unwrap_or_default();
            if path.starts_with('/') {
                push_route_candidate(&mut out, repo_name, source_file, method, path);
                found = true;
            }
        }
        if !found {
            push_route_candidate(&mut out, repo_name, source_file, method, "/");
        }
    }
    for caps in gin_re.captures_iter(content) {
        let method = caps.get(1).map(|v| v.as_str()).unwrap_or("GET");
        let path = caps.get(2).map(|v| v.as_str()).unwrap_or_default();
        push_route_candidate(&mut out, repo_name, source_file, method, path);
    }
    for caps in jaxrs_pair_re.captures_iter(content) {
        let method = caps.get(1).map(|v| v.as_str()).unwrap_or("GET");
        let path = caps.get(2).map(|v| v.as_str()).unwrap_or_default();
        push_route_candidate(&mut out, repo_name, source_file, method, path);
    }

    if source_file_lower.ends_with(".yml")
        || source_file_lower.ends_with(".yaml")
        || source_file_lower.ends_with(".json")
        || source_file_lower.contains("openapi")
        || source_file_lower.contains("swagger")
    {
        out.extend(extract_openapi_route_candidates(
            repo_name,
            source_file,
            content,
        ));
    }

    for caps in quoted_route_re.captures_iter(content) {
        let Some(route_match) = caps.get(1) else {
            continue;
        };
        let route = route_match.as_str();
        let route_lower = route.to_ascii_lowercase();
        let looks_business = route_lower.starts_with("/api")
            || business_path_keywords
                .iter()
                .any(|kw| route_lower.contains(kw));
        if !looks_business {
            continue;
        }

        let start = route_match.start().saturating_sub(96);
        let end = (route_match.end() + 96).min(content.len());
        let context = content.get(start..end).unwrap_or(content);
        let method = infer_method_from_context(context);
        push_route_candidate(&mut out, repo_name, source_file, &method, route);
    }

    if source_file.ends_with("/route.ts")
        || source_file.ends_with("/route.js")
        || source_file.ends_with("/route.tsx")
    {
        let method = next_handler_re
            .captures(content)
            .and_then(|caps| caps.get(1).map(|v| v.as_str().to_ascii_uppercase()))
            .unwrap_or_else(|| "GET".to_string());
        if let Some(app_api_pos) = source_file.find("/app/api/") {
            let p = &source_file[app_api_pos + "/app/api/".len()..];
            let p = p
                .trim_end_matches("/route.ts")
                .trim_end_matches("/route.js")
                .trim_end_matches("/route.tsx");
            if let Some(path) = normalize_inferred_path(p) {
                out.push(RouteCandidate {
                    method,
                    path,
                    source_repo: repo_name.to_string(),
                    source_file: source_file.to_string(),
                });
            }
        }
    }

    out
}

async fn infer_routes_from_workspace_repos(
    ws: &crate::state::Workspace,
    repos: &[WorkspaceRepo],
    max_cases: usize,
) -> Vec<RouteCandidate> {
    let mut out = Vec::new();
    let mut seen = HashSet::<String>::new();
    let discovery_cap = max_cases.saturating_mul(10).clamp(120, 4000);
    let allowed_ext = HashSet::<&str>::from([
        "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt", "kts", "yaml", "yml", "json",
    ]);

    for repo in repos {
        if out.len() >= discovery_cap {
            break;
        }

        let repo_path = PathBuf::from(&repo.path);
        let mut queue = VecDeque::new();
        queue.push_back((repo_path.clone(), 0usize));

        while let Some((dir, depth)) = queue.pop_front() {
            if depth > 9 || out.len() >= discovery_cap {
                continue;
            }

            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(v) => v,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_type = match entry.file_type().await {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    let name = path
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or_default();
                    if [
                        ".git",
                        "node_modules",
                        "target",
                        "build",
                        "dist",
                        ".next",
                        ".venv",
                    ]
                    .contains(&name)
                    {
                        continue;
                    }
                    queue.push_back((path, depth + 1));
                    continue;
                }

                if !file_type.is_file() {
                    continue;
                }

                let ext = path
                    .extension()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default();
                if !allowed_ext.contains(ext) {
                    continue;
                }

                let content = match tokio::fs::read_to_string(&path).await {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let source_file = path
                    .strip_prefix(&ws.path)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                for route in
                    extract_route_candidates_from_content(&repo.name, &source_file, &content)
                {
                    let key = format!("{}|{}|{}", route.source_repo, route.method, route.path);
                    if seen.insert(key) {
                        out.push(route);
                        if out.len() >= discovery_cap {
                            break;
                        }
                    }
                }
            }
        }
    }

    out.sort_by(|a, b| {
        route_priority_score(b)
            .cmp(&route_priority_score(a))
            .then_with(|| a.source_repo.cmp(&b.source_repo))
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.method.cmp(&b.method))
    });
    if out.len() > max_cases {
        out.truncate(max_cases);
    }

    out
}

async fn infer_toon_cases(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<InferToonCasesRequest>,
) -> Result<Json<InferToonCasesResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;
    let target_root = resolve_workspace_target_root(&ws).await?.ok_or_else(|| {
        AppError::BadRequest(
            "set Repo/Test Context Repository first, then run toon infer".to_string(),
        )
    })?;

    let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
    manager.setup_workspace_opencode_files(&target_root).await?;

    let repos = discover_workspace_repos(&ws.path).await;
    let max_cases = req.max_cases.unwrap_or(120).clamp(1, 300);
    let inferred = infer_routes_from_workspace_repos(&ws, &repos, max_cases).await;
    if inferred.is_empty() {
        return Err(AppError::BadRequest(
            "no route/business-logic candidates were inferred from workspace repos".to_string(),
        ));
    }

    let test_cases_dir = target_root.join("test_cases");
    if let Ok(mut entries) = tokio::fs::read_dir(&test_cases_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            if name.starts_with("auto-logic-") && name.ends_with(".toon") {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
    }

    let mut inferred_cases = Vec::new();
    let mut generated_case_refs = Vec::new();
    for (idx, route) in inferred.iter().enumerate() {
        let case_id = format!("FUNC-AUTO-{:03}", idx + 1);
        let case_file = format!("auto-logic-{:03}.toon", idx + 1);
        let case_rel = format!("test_cases/{}", case_file);
        let case_path = target_root.join(&case_rel);
        let title = route_title(&route.source_repo, &route.method, &route.path);

        let case_content = format!(
            "context:\n  kind: case\n  version: 1.0\n  id: {case_id}\n  task: {title}\nsteps[2]{{id,action,target,value}}:\n  1,api-request,{path},{method}\n  2,assert-status,200,\n",
            case_id = case_id,
            title = sanitize_toon_scalar(&title),
            path = sanitize_toon_scalar(&route.path),
            method = sanitize_toon_scalar(&route.method)
        );
        tokio::fs::write(&case_path, case_content).await?;
        generated_case_refs.push(case_rel.clone());

        inferred_cases.push(InferredToonCase {
            id: case_id,
            title,
            method: route.method.clone(),
            path: route.path.clone(),
            source_repo: route.source_repo.clone(),
            source_file: route.source_file.clone(),
            generated_case_file: case_rel,
        });
    }

    let suite_path = target_root.join("tests").join("smoke.toon");
    let existing_suite = tokio::fs::read_to_string(&suite_path)
        .await
        .unwrap_or_default();
    let mut manual_case_refs = extract_case_refs_from_toon(&existing_suite).unwrap_or_default();
    manual_case_refs.retain(|v| !v.starts_with("test_cases/auto-logic-"));
    manual_case_refs.extend(generated_case_refs.iter().cloned());
    manual_case_refs.sort();
    manual_case_refs.dedup();

    let selected_repo_value = ws
        .target
        .repo
        .as_ref()
        .map(|v| sanitize_toon_scalar(v))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "default".to_string());

    let suite_rows = manual_case_refs
        .iter()
        .enumerate()
        .map(|(idx, file)| format!("  {},{},true", idx + 1, sanitize_toon_scalar(file)))
        .collect::<Vec<_>>()
        .join("\n");

    let suite_content = format!(
        "context:\n  kind: suite\n  version: 1.0\n  task: inferred business-logic suite\n  repo: {repo}\ncases[{count}]{{id,file,required}}:\n{rows}\nartifacts[2]: manifest,coverage\n",
        repo = selected_repo_value,
        count = manual_case_refs.len(),
        rows = suite_rows
    );
    tokio::fs::write(&suite_path, suite_content).await?;

    let generated_at = chrono::Utc::now().to_rfc3339();
    let inferred_lines = inferred_cases
        .iter()
        .take(120)
        .map(|c| {
            format!(
                "- {} {} ({}) from `{}`",
                c.method, c.path, c.source_repo, c.source_file
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let generated_section = format!(
        "<!-- AUTO_INFER_CASES_START -->\n## Auto Inferred Business Logic Cases\nGenerated at: {generated_at}\n\nSelected Repo Context: `{selected}`\n\nInferred routes/cases ({count}):\n{inferred_lines}\n<!-- AUTO_INFER_CASES_END -->",
        generated_at = generated_at,
        selected = target_root.display(),
        count = inferred_cases.len(),
        inferred_lines = inferred_lines
    );

    let setup_md_path = target_root.join("SETUP.md");
    let test_md_path = target_root.join("TEST.md");
    let setup_md_existing = tokio::fs::read_to_string(&setup_md_path)
        .await
        .unwrap_or_default();
    let test_md_existing = tokio::fs::read_to_string(&test_md_path)
        .await
        .unwrap_or_default();
    let setup_md_updated = upsert_markdown_generated_section(
        &setup_md_existing,
        "<!-- AUTO_INFER_CASES_START -->",
        "<!-- AUTO_INFER_CASES_END -->",
        &generated_section,
    );
    let test_md_updated = upsert_markdown_generated_section(
        &test_md_existing,
        "<!-- AUTO_INFER_CASES_START -->",
        "<!-- AUTO_INFER_CASES_END -->",
        &generated_section,
    );
    tokio::fs::write(&setup_md_path, setup_md_updated).await?;
    tokio::fs::write(&test_md_path, test_md_updated).await?;

    let prompt = format!(
        "Analyze all repositories under `{workspace}` and refine generated .toon cases in `{target}`.\n\
Focus on real business logic paths (auth, customer, inventory, payment, loyalty), tighten expected status/body checks, and add missing critical regressions.\n\
Inferred candidates ({count}) include:\n{lines}\n\
Update only these files: SETUP.md, TEST.md, tests/smoke.toon, test_cases/auto-logic-*.toon.",
        workspace = ws.path.display(),
        target = target_root.display(),
        count = inferred_cases.len(),
        lines = inferred_cases
            .iter()
            .take(30)
            .map(|c| format!("- {} {} ({})", c.method, c.path, c.source_repo))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let mut commander_dispatched = false;
    if req.dispatch_to_commander {
        let _ = ctx.commander.send(prompt.clone()).await?;
        commander_dispatched = true;
    }

    let current = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;
    let workspace = workspace_response_with_links(&ctx, &current).await;

    Ok(Json(InferToonCasesResponse {
        workspace,
        target_root: target_root.display().to_string(),
        scanned_repos: repos.len(),
        inferred_cases,
        updated_files: vec![
            "SETUP.md".to_string(),
            "TEST.md".to_string(),
            "tests/smoke.toon".to_string(),
            "test_cases/auto-logic-*.toon".to_string(),
        ],
        prompt,
        commander_dispatched,
    }))
}

async fn set_workspace_target(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<SetWorkspaceTargetRequest>,
) -> Result<Json<WorkspaceResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let repos = discover_workspace_repos(&ws.path).await;
    let selected_repo = if let Some(repo) = req
        .repo
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        let resolved = repos
            .iter()
            .find(|r| r.path == repo || r.name == repo)
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "unknown repo '{}'; expected one of discovered repo paths or names",
                    repo
                ))
            })?;
        Some(resolved.path.clone())
    } else {
        None
    };

    let target = crate::state::WorkspaceTargetContext {
        repo: selected_repo,
    };

    let updated = ctx.state.update_workspace_target(&ws.id, target).await?;
    let workspace = workspace_response_with_links(&ctx, &updated).await;
    Ok(Json(workspace))
}

fn push_toon_issue(
    errors: &mut Vec<ToonValidationIssue>,
    code: &str,
    path: &str,
    reason: &str,
    hint: &str,
) {
    errors.push(ToonValidationIssue {
        code: code.to_string(),
        path: path.to_string(),
        reason: reason.to_string(),
        hint: hint.to_string(),
    });
}

fn compact_validation_errors(errors: &[ToonValidationIssue], limit: usize) -> String {
    errors
        .iter()
        .take(limit)
        .map(|e| format!("{}({}): {}", e.code, e.path, e.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

fn acquire_workspace_run_lock(
    ctx: &AppContext,
    workspace_id: &str,
) -> Result<WorkspaceRunLockGuard, AppError> {
    let mut locks = ctx
        .run_locks
        .lock()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("run lock is poisoned")))?;
    info!(
        workspace_id = workspace_id,
        lock_ptr = ?Arc::as_ptr(&ctx.run_locks),
        current_lock_count = locks.len(),
        "attempting workspace toon run lock"
    );
    if !locks.insert(workspace_id.to_string()) {
        return Err(AppError::Conflict(
            "a toon run is already in progress for this workspace".to_string(),
        ));
    }
    info!(
        workspace_id = workspace_id,
        lock_ptr = ?Arc::as_ptr(&ctx.run_locks),
        current_lock_count = locks.len(),
        "workspace toon run lock acquired"
    );

    Ok(WorkspaceRunLockGuard {
        workspace_id: workspace_id.to_string(),
        locks: Arc::clone(&ctx.run_locks),
        run_processes: Arc::clone(&ctx.run_processes),
    })
}

async fn terminate_runner_pid(pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .await;
}

async fn terminate_workspace_runner_process(ctx: &AppContext, workspace_id: &str) -> Option<u32> {
    let pid = {
        let run_processes = ctx.run_processes.lock().ok()?;
        run_processes.get(workspace_id).copied()
    };
    if let Some(pid) = pid {
        terminate_runner_pid(pid).await;
    }
    pid
}

async fn validate_toon_payload(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<ToonValidationRequest>,
) -> Result<(StatusCode, Json<ToonValidationResponse>), AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    let mut errors = Vec::new();
    if !req.file.ends_with(".toon") {
        push_toon_issue(
            &mut errors,
            "invalid_file",
            "file",
            "toon validation only accepts .toon files",
            "set `file` to a path ending with .toon",
        );
    }

    if req.content.trim().is_empty() {
        push_toon_issue(
            &mut errors,
            "empty_content",
            "content",
            "toon content must not be empty",
            "provide DSL text with a context block and typed collections",
        );
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(ToonValidationResponse {
                valid: false,
                errors,
            }),
        ));
    }

    let normalized_file = req.file.replace('\\', "/");
    let is_schema_file =
        normalized_file.starts_with(".toon/") || normalized_file.contains("/.toon/");
    let kind_from_path = if normalized_file.starts_with("setup/")
        || normalized_file.contains("/setup/")
    {
        Some("setup")
    } else if normalized_file.starts_with("tests/") || normalized_file.contains("/tests/") {
        Some("suite")
    } else if normalized_file.starts_with("test_cases/") || normalized_file.contains("/test_cases/")
    {
        Some("case")
    } else {
        None
    };

    let lines: Vec<&str> = req.content.lines().collect();
    let context_start = lines.iter().position(|line| line.trim() == "context:");
    if context_start.is_none() {
        push_toon_issue(
            &mut errors,
            "missing_context",
            "context",
            "context block is required",
            "add a top-level `context:` block",
        );
    }

    let mut context: BTreeMap<String, String> = BTreeMap::new();
    if let Some(start) = context_start {
        let mut idx = start + 1;
        while idx < lines.len() {
            let line = lines[idx];
            if line.trim().is_empty() {
                idx += 1;
                continue;
            }
            if !line.starts_with("  ") {
                break;
            }

            let trimmed = line.trim();
            if let Some((k, v)) = trimmed.split_once(':') {
                context.insert(k.trim().to_string(), v.trim().to_string());
            } else {
                push_toon_issue(
                    &mut errors,
                    "invalid_context_line",
                    &format!("context.line{}", idx + 1),
                    "context lines must use key:value format",
                    "use `  key: value` in the context block",
                );
            }
            idx += 1;
        }

        if context.is_empty() {
            push_toon_issue(
                &mut errors,
                "empty_context",
                "context",
                "context block must define at least one key:value entry",
                "add entries like `kind: suite` and `version: 1.0`",
            );
        }
    }

    let kind = context.get("kind").map(|v| v.as_str());
    match kind {
        Some("setup") | Some("suite") | Some("case") => {}
        Some("schema") if is_schema_file => {}
        _ => push_toon_issue(
            &mut errors,
            "invalid_kind",
            "context.kind",
            "kind must be one of setup, suite, case (or schema for .toon schema files)",
            "set `context.kind` to setup, suite, case, or schema",
        ),
    }

    if let Some(expected_kind) = kind_from_path {
        if kind != Some(expected_kind) {
            push_toon_issue(
                &mut errors,
                "path_kind_mismatch",
                "context.kind",
                "context.kind does not match file location",
                "use setup/*.toon => setup, tests/*.toon => suite, test_cases/*.toon => case",
            );
        }
    }

    if context.get("version").map(|v| v.as_str()) != Some("1.0") {
        push_toon_issue(
            &mut errors,
            "invalid_version",
            "context.version",
            "version must be 1.0",
            "set `context.version: 1.0`",
        );
    }

    let repos = discover_workspace_repos(&ws.path).await;
    let repo_names: HashSet<String> = repos
        .iter()
        .flat_map(|r| [r.name.clone(), r.path.clone()])
        .collect();
    if let Some(repo) = context.get("repo") {
        if !repo.is_empty() && repo != "default" && !repo_names.contains(repo) {
            push_toon_issue(
                &mut errors,
                "unknown_repo",
                "context.repo",
                "repo must reference a discovered workspace repository",
                "set to `default` or one of GET /api/workspaces/{id} repos",
            );
        }
    }

    let collection_re = Regex::new(r"^([A-Za-z_][A-Za-z0-9_-]*)\[(\d+)\](?:\{([^}]*)\})?:\s*(.*)$")
        .map_err(|err| AppError::Internal(anyhow::anyhow!(err.to_string())))?;
    let case_ref_re = Regex::new(r"test_cases/[A-Za-z0-9_./-]+\.toon")
        .map_err(|err| AppError::Internal(anyhow::anyhow!(err.to_string())))?;

    let mut collection_sizes: BTreeMap<String, usize> = BTreeMap::new();
    let mut case_refs: Vec<String> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(caps) = collection_re.captures(trimmed) else {
            continue;
        };

        let name = caps.get(1).map(|v| v.as_str()).unwrap_or_default();
        let expected = caps
            .get(2)
            .and_then(|v| v.as_str().parse::<usize>().ok())
            .unwrap_or(0);
        let inline_tail = caps.get(4).map(|v| v.as_str().trim()).unwrap_or_default();

        let mut actual = 0usize;
        if !inline_tail.is_empty() {
            actual = inline_tail
                .split(',')
                .filter(|part| !part.trim().is_empty())
                .count();
            if name == "cases" {
                for case_match in case_ref_re.find_iter(inline_tail) {
                    case_refs.push(case_match.as_str().to_string());
                }
            }
        } else {
            let mut row_idx = idx + 1;
            let mut saw_numbered_rows = false;
            while row_idx < lines.len() {
                let row = lines[row_idx];
                if row.trim().is_empty() {
                    row_idx += 1;
                    continue;
                }
                if !row.starts_with("  ") {
                    break;
                }

                let row_trim = row.trim();
                if row_trim
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    actual += 1;
                    saw_numbered_rows = true;
                    if name == "cases" {
                        for case_match in case_ref_re.find_iter(row_trim) {
                            case_refs.push(case_match.as_str().to_string());
                        }
                    }
                } else if !saw_numbered_rows && !row_trim.contains(':') {
                    actual += 1;
                }
                row_idx += 1;
            }
        }

        collection_sizes.insert(name.to_string(), actual);

        if expected != actual {
            push_toon_issue(
                &mut errors,
                "count_mismatch",
                &format!("{name}[{expected}]"),
                "declared item count does not match provided rows/items",
                "update [count] or provided rows/items so they match",
            );
        }
    }

    let validation_root = resolve_workspace_target_root(&ws)
        .await?
        .unwrap_or_else(|| ws.path.clone());

    let effective_kind = kind.or(kind_from_path);
    match effective_kind {
        Some("setup") => {
            if collection_sizes.is_empty() {
                push_toon_issue(
                    &mut errors,
                    "missing_collections",
                    "collections",
                    "setup toon requires at least one typed collection",
                    "add a collection like commands[2]{id,run}: ...",
                );
            }
        }
        Some("suite") => {
            if !collection_sizes.contains_key("cases") {
                push_toon_issue(
                    &mut errors,
                    "missing_cases",
                    "cases",
                    "suite toon requires cases[count] section",
                    "add cases[count]{...}: section",
                );
            }
            if case_refs.is_empty() {
                push_toon_issue(
                    &mut errors,
                    "missing_case_reference",
                    "cases",
                    "suite must include at least one test_cases/*.toon reference",
                    "add references like test_cases/login-flow.toon",
                );
            }
            for (idx, rel) in case_refs.iter().enumerate() {
                let path = validation_root.join(rel);
                if !tokio::fs::try_exists(path).await.unwrap_or(false) {
                    push_toon_issue(
                        &mut errors,
                        "missing_case_file",
                        &format!("cases.ref[{idx}]"),
                        "referenced test case file does not exist",
                        "create the file under test_cases/ or update the reference",
                    );
                }
            }
        }
        Some("case") => {
            if collection_sizes.is_empty() {
                push_toon_issue(
                    &mut errors,
                    "missing_collections",
                    "collections",
                    "case toon requires at least one typed collection",
                    "add a collection like steps[5]{id,action,target}: ...",
                );
            }
        }
        Some("schema") if is_schema_file => {}
        _ => {}
    }

    let valid = errors.is_empty();
    let status = if valid {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    Ok((status, Json(ToonValidationResponse { valid, errors })))
}

fn safe_relative_toon_path(input: &str) -> bool {
    let normalized = input.trim().replace('\\', "/");
    !normalized.is_empty()
        && !normalized.starts_with('/')
        && !normalized.contains("..")
        && normalized.ends_with(".toon")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

async fn resolve_existing_path_within_root(
    root: &StdPath,
    relative: &str,
    label: &str,
) -> Result<PathBuf, AppError> {
    let root_real = tokio::fs::canonicalize(root).await?;
    let candidate = root.join(relative);
    if !tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
        return Err(AppError::BadRequest(format!(
            "{} not found: {}",
            label, relative
        )));
    }
    let candidate_real = tokio::fs::canonicalize(&candidate).await?;
    if !candidate_real.starts_with(&root_real) {
        return Err(AppError::BadRequest(format!(
            "{} must stay within workspace root",
            label
        )));
    }
    Ok(candidate_real)
}

async fn resolve_workspace_target_root(
    ws: &crate::state::Workspace,
) -> Result<Option<PathBuf>, AppError> {
    let workspace_real = tokio::fs::canonicalize(&ws.path).await?;
    let has_target_repo = ws
        .target
        .repo
        .as_ref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    if !has_target_repo {
        return Ok(None);
    }

    let mut root = workspace_real.clone();

    if let Some(repo) = ws
        .target
        .repo
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        let repo_path = PathBuf::from(repo);
        root = if repo_path.is_absolute() {
            repo_path
        } else {
            workspace_real.join(repo_path)
        };
    }

    if !tokio::fs::try_exists(&root).await.unwrap_or(false) {
        return Ok(None);
    }

    let root_real = tokio::fs::canonicalize(&root).await?;
    if !root_real.starts_with(&workspace_real) {
        return Err(AppError::BadRequest(
            "workspace target root must stay within workspace directory".to_string(),
        ));
    }

    Ok(Some(root_real))
}

async fn resolve_suite_path_for_workspace(
    ws: &crate::state::Workspace,
    suite_file: &str,
) -> Result<(PathBuf, PathBuf), AppError> {
    let workspace_real = tokio::fs::canonicalize(&ws.path).await?;
    if let Some(target_real) = resolve_workspace_target_root(ws).await? {
        if let Ok(path) =
            resolve_existing_path_within_root(&target_real, suite_file, "suite file").await
        {
            return Ok((target_real, path));
        }
    }

    if let Ok(path) =
        resolve_existing_path_within_root(&workspace_real, suite_file, "suite file").await
    {
        return Ok((workspace_real, path));
    }

    Err(AppError::BadRequest(format!(
        "suite file not found in target root or workspace root: {}",
        suite_file
    )))
}

fn parse_localhost_port_from_url(url: &str) -> Option<u16> {
    let re = Regex::new(r"^https?://(?:127\.0\.0\.1|localhost):(\d+)(?:/.*)?$").ok()?;
    let caps = re.captures(url.trim())?;
    caps.get(1)?.as_str().parse::<u16>().ok()
}

async fn is_allowed_base_url_for_workspace(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
    url: &str,
) -> bool {
    let Some(port) = parse_localhost_port_from_url(url) else {
        return false;
    };

    let mut allowed_ports = HashSet::<u16>::new();
    allowed_ports.insert(8080);
    for tunnel in ctx.state.list_tunnels(&ws.id).await {
        allowed_ports.insert(tunnel.local_port);
    }
    for frontend_port in declared_frontend_ports(&ws.path).await {
        allowed_ports.insert(frontend_port);
    }

    allowed_ports.contains(&port)
}

fn extract_case_refs_from_toon(content: &str) -> Result<Vec<String>, AppError> {
    let rows = parse_toon_collection_rows(content, "cases")
        .map_err(|err| AppError::BadRequest(format!("invalid cases collection: {}", err)))?;
    let mut refs: Vec<String> = rows
        .iter()
        .flat_map(|row| row.iter())
        .map(|col| col.trim())
        .filter(|value| value.starts_with("test_cases/") && value.ends_with(".toon"))
        .map(|value| value.to_string())
        .collect();
    refs.sort();
    refs.dedup();
    Ok(refs)
}

#[derive(Clone, Serialize)]
struct ToonCaseStep {
    action: String,
    target: String,
    value: String,
}

struct ParsedToonCase {
    id: String,
    name: String,
    steps: Vec<ToonCaseStep>,
}

#[derive(Serialize)]
struct PlaywrightRunnerInput {
    base_url: String,
    video_dir: String,
    api_timeout_ms: u64,
    cases: Vec<PlaywrightRunnerCaseInput>,
}

#[derive(Serialize)]
struct PlaywrightRunnerCaseInput {
    id: String,
    name: String,
    screenshot_path: String,
    steps: Vec<ToonCaseStep>,
}

#[derive(Deserialize)]
struct PlaywrightRunnerOutput {
    cases: Vec<PlaywrightRunnerCaseOutput>,
}

#[derive(Deserialize)]
struct PlaywrightRunnerCaseOutput {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    error: Option<String>,
}

fn parse_toon_context_block(content: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let Some(context_start) = lines.iter().position(|line| line.trim() == "context:") else {
        return out;
    };

    let mut idx = context_start + 1;
    while idx < lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() {
            idx += 1;
            continue;
        }
        if !line.starts_with("  ") {
            break;
        }

        if let Some((k, v)) = line.trim().split_once(':') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
        idx += 1;
    }

    out
}

fn parse_toon_collection_rows(
    content: &str,
    collection_name: &str,
) -> Result<Vec<Vec<String>>, String> {
    let lines: Vec<&str> = content.lines().collect();
    let header_re = Regex::new(&format!(
        r"^{}\[(\d+)\]\{{([^}}]*)\}}:\s*(.*)$",
        regex::escape(collection_name)
    ))
    .map_err(|err| err.to_string())?;

    let Some((header_idx, caps)) = lines
        .iter()
        .enumerate()
        .find_map(|(idx, line)| header_re.captures(line.trim()).map(|caps| (idx, caps)))
    else {
        return Ok(Vec::new());
    };

    let inline_tail = caps.get(3).map(|v| v.as_str().trim()).unwrap_or_default();

    let mut rows = Vec::new();
    if !inline_tail.is_empty() {
        rows.push(
            inline_tail
                .split(',')
                .map(|part| part.trim().to_string())
                .collect(),
        );
        return Ok(rows);
    }

    let mut row_idx = header_idx + 1;
    while row_idx < lines.len() {
        let row = lines[row_idx];
        if row.trim().is_empty() {
            row_idx += 1;
            continue;
        }
        if !row.starts_with("  ") {
            break;
        }

        let mut cols: Vec<String> = row
            .trim()
            .split(',')
            .map(|part| part.trim().to_string())
            .collect();

        if cols.first().and_then(|v| v.parse::<usize>().ok()).is_some() {
            cols.remove(0);
        }

        rows.push(cols);
        row_idx += 1;
    }

    Ok(rows)
}

fn parse_toon_case(case_ref: &str, content: &str) -> Result<ParsedToonCase, String> {
    let context = parse_toon_context_block(content);
    let kind = context.get("kind").map(|v| v.trim()).unwrap_or_default();
    if kind != "case" {
        return Err("context.kind must be case".to_string());
    }

    let case_id = context
        .get("id")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| case_ref.to_string());
    let case_name = context
        .get("task")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| case_ref.to_string());

    let step_rows = parse_toon_collection_rows(content, "steps")?;
    if step_rows.is_empty() {
        return Err("steps collection is required and cannot be empty".to_string());
    }

    let mut steps = Vec::new();
    for row in step_rows {
        let (action, target, value) = if row.len() >= 4 {
            (
                row.get(1).map(|v| v.trim()).unwrap_or_default().to_string(),
                row.get(2).map(|v| v.trim()).unwrap_or_default().to_string(),
                row.get(3).map(|v| v.trim()).unwrap_or_default().to_string(),
            )
        } else if row.len() >= 3 {
            (
                row.get(0).map(|v| v.trim()).unwrap_or_default().to_string(),
                row.get(1).map(|v| v.trim()).unwrap_or_default().to_string(),
                row.get(2).map(|v| v.trim()).unwrap_or_default().to_string(),
            )
        } else {
            return Err("step row must include action,target,value columns".to_string());
        };

        if action.is_empty() {
            return Err("step action is required".to_string());
        }
        let normalized = action.to_ascii_lowercase();
        let supported = matches!(
            normalized.as_str(),
            "visit"
                | "fill"
                | "click"
                | "assert-url-contains"
                | "api-request"
                | "assert-status"
                | "assert-body-contains"
        );
        if !supported {
            return Err(format!("unsupported step action '{}'", action));
        }
        if normalized == "api-request" && row.len() > 4 {
            return Err(
                "api-request value contains unsupported extra comma-separated columns".to_string(),
            );
        }

        steps.push(ToonCaseStep {
            action,
            target,
            value,
        });
    }

    Ok(ParsedToonCase {
        id: case_id,
        name: case_name,
        steps,
    })
}

fn resolve_toon_suite_base_url(content: &str) -> String {
    let context = parse_toon_context_block(content);
    context
        .get("base_url")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_default()
}

async fn default_workspace_base_url(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
) -> Option<String> {
    let mut tunnels = ctx.state.list_tunnels(&ws.id).await;
    tunnels.sort_by_key(|t| t.local_port);
    if let Some(tunnel) = tunnels.first() {
        return Some(format!("http://127.0.0.1:{}", tunnel.local_port));
    }

    let ports = declared_frontend_ports(&ws.path).await;
    ports
        .first()
        .copied()
        .map(|port| format!("http://127.0.0.1:{}", port))
}

fn normalize_playwright_case_status(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "pass" => "pass",
        "blocked" => "blocked",
        _ => "fail",
    }
}

fn playwright_runner_script() -> &'static str {
    r#"import fs from 'node:fs/promises';
import { chromium } from 'playwright';

async function main() {
  const inputPath = process.argv[2];
  if (!inputPath) {
    throw new Error('missing input path');
  }

  const raw = await fs.readFile(inputPath, 'utf8');
  const input = JSON.parse(raw);
  const baseUrl = String(input.base_url || 'http://127.0.0.1:3000').replace(/\/$/, '');
  const apiTimeoutMs = Number.parseInt(String(input.api_timeout_ms || 30000), 10);
  let baseOrigin = '';
  try {
    baseOrigin = new URL(baseUrl).origin;
  } catch {
    throw new Error(`invalid base_url '${baseUrl}'`);
  }
  const cases = Array.isArray(input.cases) ? input.cases : [];

  const browser = await chromium.launch({ headless: true });

  const results = [];
  for (const c of cases) {
    const context = await browser.newContext({
      recordVideo: { dir: input.video_dir || '.' },
      viewport: { width: 1366, height: 900 },
    });
    const page = await context.newPage();
    let status = 'pass';
    let error = null;
    let state = {
      responseStatus: null,
      responseBody: '',
    };

    try {
      const steps = Array.isArray(c.steps) ? c.steps : [];
      for (let i = 0; i < steps.length; i += 1) {
        const step = steps[i] || {};
        const action = String(step.action || '').trim().toLowerCase();
        const target = String(step.target || '').trim();
        const value = String(step.value || '');

        if (!action) {
          status = 'fail';
          error = `step ${i + 1} action is empty`;
          break;
        }

        if (action === 'visit') {
          if (!target) {
            status = 'fail';
            error = `step ${i + 1} visit target is empty`;
            break;
          }
          const url = /^https?:\/\//i.test(target) ? target : `${baseUrl}${target.startsWith('/') ? '' : '/'}${target}`;
          try {
            const targetOrigin = new URL(url).origin;
            if (targetOrigin !== baseOrigin) {
              status = 'fail';
              error = `step ${i + 1} visit origin '${targetOrigin}' differs from base origin '${baseOrigin}'`;
              break;
            }
          } catch {
            status = 'fail';
            error = `step ${i + 1} visit target resolves to invalid URL '${url}'`;
            break;
          }
          await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
          const navigated = page.url();
          try {
            const actualOrigin = new URL(navigated).origin;
            if (actualOrigin !== baseOrigin) {
              status = 'fail';
              error = `step ${i + 1} navigation redirected to origin '${actualOrigin}', expected '${baseOrigin}'`;
              break;
            }
          } catch {
            status = 'fail';
            error = `step ${i + 1} navigation ended with invalid URL '${navigated}'`;
            break;
          }
          continue;
        }

        if (action === 'fill') {
          if (!target) {
            status = 'fail';
            error = `step ${i + 1} fill target is empty`;
            break;
          }
          await page.fill(target, value, { timeout: 15000 });
          continue;
        }

        if (action === 'click') {
          if (!target) {
            status = 'fail';
            error = `step ${i + 1} click target is empty`;
            break;
          }
          await page.click(target, { timeout: 15000 });
          continue;
        }

        if (action === 'assert-url-contains') {
          if (!target) {
            status = 'fail';
            error = `step ${i + 1} assert target is empty`;
            break;
          }
          const current = page.url();
          if (!current.includes(target)) {
            status = 'fail';
            error = `step ${i + 1} expected url to contain '${target}' but current url is '${current}'`;
          }
          continue;
        }

        if (action === 'api-request') {
          if (!target) {
            status = 'fail';
            error = `step ${i + 1} api-request target is empty`;
            break;
          }
          const [methodRaw, ...rest] = value.split('|');
          const method = String(methodRaw || 'GET').trim().toUpperCase() || 'GET';
          const body = rest.join('|').trim();
          const url = /^https?:\/\//i.test(target) ? target : `${baseUrl}${target.startsWith('/') ? '' : '/'}${target}`;
          try {
            const targetOrigin = new URL(url).origin;
            if (targetOrigin !== baseOrigin) {
              status = 'fail';
              error = `step ${i + 1} api-request origin '${targetOrigin}' differs from base origin '${baseOrigin}'`;
              break;
            }
          } catch {
            status = 'fail';
            error = `step ${i + 1} api-request target resolves to invalid URL '${url}'`;
            break;
          }
          const init = { method, headers: {} };
          if (body) {
            init.body = body;
            init.headers['content-type'] = 'application/json';
          }
          let response;
          try {
            response = await page.request.fetch(url, {
              ...init,
              timeout: apiTimeoutMs,
            });
          } catch (requestErr) {
            status = 'fail';
            error = `step ${i + 1} api-request failed: ${requestErr instanceof Error ? requestErr.message : String(requestErr)}`;
            break;
          }
          try {
            const responseUrl = typeof response.url === 'function' ? response.url() : String(response.url || '');
            const responseOrigin = new URL(responseUrl, baseUrl).origin;
            if (responseOrigin !== baseOrigin) {
              status = 'fail';
              error = `step ${i + 1} api-request redirected to origin '${responseOrigin}', expected '${baseOrigin}'`;
              break;
            }
          } catch {
            status = 'fail';
            const rawUrl = typeof response.url === 'function' ? response.url() : String(response.url || '');
            error = `step ${i + 1} api-request returned invalid response URL '${rawUrl}'`;
            break;
          }
          state.responseStatus = typeof response.status === 'function' ? response.status() : response.status;
          const bodyText = await (typeof response.text === 'function' ? response.text() : '');
          state.responseBody = bodyText.length > 100_000 ? bodyText.slice(0, 100_000) : bodyText;
          continue;
        }

        if (action === 'assert-status') {
          const expectedRaw = target || value;
          const expected = Number.parseInt(String(expectedRaw), 10);
          if (!Number.isInteger(expected)) {
            status = 'fail';
            error = `step ${i + 1} assert-status requires numeric expected status`;
            break;
          }
          if (state.responseStatus === null) {
            status = 'fail';
            error = `step ${i + 1} assert-status requires a prior api-request`;
            break;
          }
          if (state.responseStatus !== expected) {
            status = 'fail';
            error = `step ${i + 1} expected status ${expected} but got ${state.responseStatus}`;
          }
          continue;
        }

        if (action === 'assert-body-contains') {
          const expected = target || value;
          if (!expected) {
            status = 'fail';
            error = `step ${i + 1} assert-body-contains requires expected text`;
            break;
          }
          if (!state.responseBody.includes(expected)) {
            status = 'fail';
            error = `step ${i + 1} expected response body to contain '${expected}'`;
          }
          continue;
        }

        status = 'blocked';
        error = `unsupported action '${action}' in step ${i + 1}`;
        break;
      }
    } catch (err) {
      status = 'fail';
      error = err instanceof Error ? err.message : String(err);
    } finally {
      try {
        if (c.screenshot_path) {
          await page.screenshot({ path: c.screenshot_path, fullPage: true });
        }
      } catch (_) {
      }
      await page.close();
      await context.close();
    }

    results.push({
      id: String(c.id || ''),
      name: String(c.name || c.id || ''),
      status,
      error,
    });
  }

  await browser.close();
  process.stdout.write(JSON.stringify({ cases: results }));
}

main().catch((err) => {
  const msg = err instanceof Error ? err.message : String(err);
  process.stderr.write(msg + '\n');
  process.exit(1);
});
"#
}

async fn execute_toon_cases_with_playwright(
    ctx: &AppContext,
    workspace_id: &str,
    run_dir: &StdPath,
    video_dir: &StdPath,
    suite_base_url: &str,
    parsed_cases: &[ParsedToonCase],
    timeout_seconds: u64,
) -> Result<Vec<PlaywrightRunnerCaseOutput>, AppError> {
    let runner_script_path = run_dir.join("toon-runner.mjs");
    let runner_input_path = run_dir.join("toon-runner-input.json");

    let input = PlaywrightRunnerInput {
        base_url: suite_base_url.to_string(),
        video_dir: video_dir.to_string_lossy().to_string(),
        api_timeout_ms: timeout_seconds.saturating_mul(1000).clamp(1_000, 120_000),
        cases: parsed_cases
            .iter()
            .enumerate()
            .map(|(idx, case)| PlaywrightRunnerCaseInput {
                id: case.id.clone(),
                name: case.name.clone(),
                screenshot_path: run_dir
                    .join("screenshots")
                    .join(format!("case-{:03}.png", idx + 1))
                    .to_string_lossy()
                    .to_string(),
                steps: case.steps.clone(),
            })
            .collect(),
    };

    tokio::fs::write(&runner_script_path, playwright_runner_script()).await?;
    tokio::fs::write(&runner_input_path, serde_json::to_vec_pretty(&input)?).await?;

    let runner_cwd = ctx.config.opencode_repo.join("packages").join("opencode");
    let mut cmd = Command::new("bun");
    cmd.kill_on_drop(true)
        .arg(runner_script_path.to_string_lossy().to_string())
        .arg(runner_input_path.to_string_lossy().to_string())
        .current_dir(&runner_cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn()?;
    if let Some(pid) = child.id() {
        if let Ok(mut run_processes) = ctx.run_processes.lock() {
            run_processes.insert(workspace_id.to_string(), pid);
        }
    }
    let _process_guard = WorkspaceRunProcessGuard {
        workspace_id: workspace_id.to_string(),
        run_processes: Arc::clone(&ctx.run_processes),
    };

    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds),
        child.wait_with_output(),
    )
    .await
    {
        Ok(output_result) => output_result?,
        Err(_) => {
            let _ = terminate_workspace_runner_process(ctx, workspace_id).await;
            return Err(AppError::CommandFailed(format!(
                "toon Playwright runner timed out after {} seconds",
                timeout_seconds
            )));
        }
    };

    if !output.status.success() {
        if output.status.code().is_none() {
            return Err(AppError::CommandFailed(
                "toon Playwright runner cancelled".to_string(),
            ));
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::CommandFailed(format!(
            "toon Playwright runner failed: {}",
            if stderr.is_empty() {
                "unknown stderr".to_string()
            } else {
                stderr
            }
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parsed = serde_json::from_str::<PlaywrightRunnerOutput>(&stdout).map_err(|err| {
        AppError::CommandFailed(format!(
            "failed to parse Playwright runner JSON output: {}",
            err
        ))
    })?;

    Ok(parsed.cases)
}

#[cfg(test)]
fn execute_toon_case(case: &ParsedToonCase) -> (String, Option<String>) {
    let mut current_url = String::new();
    let mut filled = BTreeMap::<String, String>::new();
    let mut response_status: Option<u16> = None;
    let mut response_body = String::new();

    for (idx, step) in case.steps.iter().enumerate() {
        let action = step.action.trim().to_ascii_lowercase();
        match action.as_str() {
            "visit" => {
                if step.target.is_empty() {
                    return (
                        "fail".to_string(),
                        Some(format!("step {} visit target is empty", idx + 1)),
                    );
                }
                current_url = step.target.clone();
            }
            "fill" => {
                if step.target.is_empty() || step.value.is_empty() {
                    return (
                        "fail".to_string(),
                        Some(format!("step {} fill requires target and value", idx + 1)),
                    );
                }
                filled.insert(step.target.clone(), step.value.clone());
            }
            "click" => {
                if step.target.is_empty() {
                    return (
                        "fail".to_string(),
                        Some(format!("step {} click target is empty", idx + 1)),
                    );
                }
            }
            "assert-url-contains" => {
                if step.target.is_empty() {
                    return (
                        "fail".to_string(),
                        Some(format!("step {} assert target is empty", idx + 1)),
                    );
                }
                if current_url.is_empty() || !current_url.contains(&step.target) {
                    return (
                        "fail".to_string(),
                        Some(format!(
                            "step {} expected url to contain '{}' but current url is '{}'",
                            idx + 1,
                            step.target,
                            current_url
                        )),
                    );
                }
            }
            "api-request" => {
                if step.target.is_empty() {
                    return (
                        "fail".to_string(),
                        Some(format!("step {} api-request target is empty", idx + 1)),
                    );
                }
                if step.target.contains("/health") {
                    response_status = Some(200);
                    response_body = r#"{"status":"ok"}"#.to_string();
                } else {
                    response_status = Some(404);
                    response_body = r#"{"error":"not found"}"#.to_string();
                }
            }
            "assert-status" => {
                let expected_raw = if step.target.is_empty() {
                    step.value.trim()
                } else {
                    step.target.trim()
                };
                let Ok(expected) = expected_raw.parse::<u16>() else {
                    return (
                        "fail".to_string(),
                        Some(format!(
                            "step {} assert-status requires numeric expected status",
                            idx + 1
                        )),
                    );
                };
                let Some(actual) = response_status else {
                    return (
                        "fail".to_string(),
                        Some(format!(
                            "step {} assert-status requires a prior api-request",
                            idx + 1
                        )),
                    );
                };
                if actual != expected {
                    return (
                        "fail".to_string(),
                        Some(format!(
                            "step {} expected status {} but got {}",
                            idx + 1,
                            expected,
                            actual
                        )),
                    );
                }
            }
            "assert-body-contains" => {
                let expected = if step.target.is_empty() {
                    step.value.trim()
                } else {
                    step.target.trim()
                };
                if expected.is_empty() {
                    return (
                        "fail".to_string(),
                        Some(format!(
                            "step {} assert-body-contains requires expected text",
                            idx + 1
                        )),
                    );
                }
                if !response_body.contains(expected) {
                    return (
                        "fail".to_string(),
                        Some(format!(
                            "step {} expected response body to contain '{}'",
                            idx + 1,
                            expected
                        )),
                    );
                }
            }
            _ => {
                return (
                    "blocked".to_string(),
                    Some(format!(
                        "unsupported action '{}' in step {}",
                        step.action,
                        idx + 1
                    )),
                );
            }
        }
    }

    if filled.is_empty()
        && case
            .steps
            .iter()
            .any(|s| s.action.eq_ignore_ascii_case("fill"))
    {
        return (
            "fail".to_string(),
            Some("fill steps were declared but no field values were captured".to_string()),
        );
    }

    ("pass".to_string(), None)
}

async fn run_toon_suite(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    Json(req): Json<RunToonSuiteRequest>,
) -> Result<(StatusCode, Json<RunToonSuiteResponse>), AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let mut ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    if req.ensure_workspace_started && ws.status != WorkspaceStatus::Running {
        let manager = WorkspaceManager::new(ctx.config.clone(), ctx.supervisor.clone());
        manager.setup_workspace_opencode_files(&ws.path).await?;
        let port = ws.opencode_port.unwrap_or(4100);
        let ttyd_port = ws.ttyd_port.unwrap_or(7681);
        let _ = manager
            .start_workspace(&ws.name, &ws.path, port, ttyd_port)
            .await?;
        ws = ctx
            .state
            .update_workspace_status(&ws.id, WorkspaceStatus::Running)
            .await?;
        let _ = publish_declared_frontends(&ctx, &ws).await;
    }

    let _run_lock = acquire_workspace_run_lock(&ctx, &ws.id)?;

    if !safe_relative_toon_path(&req.suite_file) {
        return Err(AppError::BadRequest(
            "suite_file must be a safe relative .toon path".to_string(),
        ));
    }

    let (suite_root, suite_path) =
        resolve_suite_path_for_workspace(&ws, req.suite_file.trim()).await?;

    let suite_content = tokio::fs::read_to_string(&suite_path).await?;
    let suite_kind_re = Regex::new(r"(?m)^\s*kind:\s*suite\s*$")
        .map_err(|err| AppError::Internal(anyhow::anyhow!(err.to_string())))?;
    if !suite_kind_re.is_match(&suite_content) {
        return Err(AppError::BadRequest(
            "suite_file content must declare kind: suite in the context block".to_string(),
        ));
    }

    let case_refs = extract_case_refs_from_toon(&suite_content)?;
    if case_refs.is_empty() {
        return Err(AppError::BadRequest(
            "suite must reference at least one test_cases/*.toon file".to_string(),
        ));
    }

    let (_, Json(suite_validation)) = validate_toon_payload(
        State(ctx.clone()),
        Path(id.clone()),
        Json(ToonValidationRequest {
            file: req.suite_file.clone(),
            content: suite_content.clone(),
        }),
    )
    .await?;
    if !suite_validation.valid {
        return Err(AppError::BadRequest(format!(
            "suite validation failed: {}",
            compact_validation_errors(&suite_validation.errors, 6)
        )));
    }

    let mut case_validation_failures = Vec::new();
    for case_ref in &case_refs {
        if case_ref.contains("..") || case_ref.starts_with('/') || case_ref.starts_with("./") {
            case_validation_failures.push(format!("{}: invalid case reference path", case_ref));
            continue;
        }
        let case_path =
            match resolve_existing_path_within_root(&suite_root, case_ref, "case file").await {
                Ok(path) => path,
                Err(_) => {
                    case_validation_failures
                        .push(format!("{}: referenced test case file not found", case_ref));
                    continue;
                }
            };
        let case_content = tokio::fs::read_to_string(&case_path).await?;
        let (_, Json(case_validation)) = validate_toon_payload(
            State(ctx.clone()),
            Path(id.clone()),
            Json(ToonValidationRequest {
                file: case_ref.clone(),
                content: case_content,
            }),
        )
        .await?;
        if !case_validation.valid {
            case_validation_failures.push(format!(
                "{}: {}",
                case_ref,
                compact_validation_errors(&case_validation.errors, 4)
            ));
        }
    }

    if !case_validation_failures.is_empty() {
        return Err(AppError::BadRequest(format!(
            "toon contract validation failed: {}",
            case_validation_failures
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }

    let run_id = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f").to_string();
    info!(
        workspace_id = %ws.id,
        workspace = %ws.name,
        run_id = %run_id,
        suite_file = %req.suite_file,
        "toon run started"
    );
    let run_dir = ws.path.join("logs").join(&run_id);
    let screenshots_dir = run_dir.join("screenshots");
    let video_dir = run_dir.join("video");
    tokio::fs::create_dir_all(&screenshots_dir).await?;
    tokio::fs::create_dir_all(&video_dir).await?;

    let mut tests = Vec::new();
    let mut screenshots = Vec::new();
    let mut passed_case_ids = HashSet::<String>::new();
    let mut passed: u64 = 0;
    let mut failed: u64 = 0;
    let mut blocked: u64 = 0;
    let mut parsed_cases = Vec::<ParsedToonCase>::new();
    let fallback_base_url = default_workspace_base_url(&ctx, &ws)
        .await
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());
    let suite_declared_base = resolve_toon_suite_base_url(&suite_content);
    let requested_base = req
        .base_url
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let configured_base = requested_base
        .clone()
        .or_else(|| (!suite_declared_base.is_empty()).then_some(suite_declared_base.clone()));

    let suite_base_url = if let Some(base) = configured_base {
        if is_allowed_base_url_for_workspace(&ctx, &ws, &base).await {
            base
        } else {
            return Err(AppError::BadRequest(
                "base_url must target localhost/127.0.0.1 with an allowed workspace port"
                    .to_string(),
            ));
        }
    } else {
        fallback_base_url
    };
    let timeout_seconds = req.timeout_seconds.unwrap_or(300).clamp(30, 900);

    for case_ref in &case_refs {
        if case_ref.contains("..") || case_ref.starts_with('/') || case_ref.starts_with("./") {
            failed += 1;
            tests.push(serde_json::json!({
                "id": case_ref,
                "name": case_ref,
                "status": "fail",
                "error": "invalid case reference path"
            }));
            continue;
        }

        let case_path =
            match resolve_existing_path_within_root(&suite_root, case_ref, "case file").await {
                Ok(path) => path,
                Err(_) => {
                    failed += 1;
                    tests.push(serde_json::json!({
                        "id": case_ref,
                        "name": case_ref,
                        "status": "fail",
                        "error": "referenced test case file not found"
                    }));
                    continue;
                }
            };

        let case_content = tokio::fs::read_to_string(&case_path).await?;
        let parsed = parse_toon_case(case_ref, &case_content);
        match parsed {
            Ok(parsed_case) => {
                parsed_cases.push(parsed_case);
            }
            Err(parse_err) => {
                failed += 1;
                tests.push(serde_json::json!({
                    "id": case_ref,
                    "name": case_ref,
                    "status": "fail",
                    "error": format!("invalid case file: {}", parse_err)
                }));
            }
        }
    }

    let mut deduped_cases = Vec::new();
    let mut seen_case_ids = HashSet::new();
    for case in parsed_cases {
        if seen_case_ids.insert(case.id.clone()) {
            deduped_cases.push(case);
        } else {
            failed += 1;
            tests.push(serde_json::json!({
                "id": case.id,
                "name": case.name,
                "status": "fail",
                "error": "duplicate case id in suite"
            }));
        }
    }
    let parsed_cases = deduped_cases;

    if !parsed_cases.is_empty() {
        if ctx.config.execute_commands {
            match execute_toon_cases_with_playwright(
                &ctx,
                &ws.id,
                &run_dir,
                &video_dir,
                &suite_base_url,
                &parsed_cases,
                timeout_seconds,
            )
            .await
            {
                Ok(results) => {
                    for (idx, case) in parsed_cases.iter().enumerate() {
                        let screenshot_rel = format!("screenshots/case-{:03}.png", idx + 1);
                        let resolved = results
                            .iter()
                            .find(|r| r.id == case.id)
                            .map(|r| {
                                (
                                    normalize_playwright_case_status(&r.status),
                                    r.error.clone(),
                                    r.name.clone(),
                                )
                            })
                            .unwrap_or((
                                "fail",
                                Some("runner did not return this case result".to_string()),
                                case.name.clone(),
                            ));

                        match resolved.0 {
                            "pass" => {
                                passed += 1;
                                passed_case_ids.insert(case.id.clone());
                            }
                            "blocked" => blocked += 1,
                            _ => failed += 1,
                        }

                        tests.push(serde_json::json!({
                            "id": case.id,
                            "name": resolved.2,
                            "status": resolved.0,
                            "error": resolved.1
                        }));

                        if tokio::fs::try_exists(run_dir.join(&screenshot_rel))
                            .await
                            .unwrap_or(false)
                        {
                            screenshots.push(serde_json::json!({
                                "test_id": case.id,
                                "path": screenshot_rel,
                                "description": "playwright capture"
                            }));
                        }
                    }
                }
                Err(err) => {
                    let is_cancelled = err.to_string().to_ascii_lowercase().contains("cancelled");
                    for case in &parsed_cases {
                        if is_cancelled {
                            blocked += 1;
                        } else {
                            failed += 1;
                        }
                        tests.push(serde_json::json!({
                            "id": case.id,
                            "name": case.name,
                            "status": if is_cancelled { "blocked" } else { "fail" },
                            "error": if is_cancelled {
                                "playwright runner cancelled via API".to_string()
                            } else {
                                format!("playwright runner error: {}", err)
                            }
                        }));
                    }
                }
            }
        } else {
            for case in &parsed_cases {
                blocked += 1;
                tests.push(serde_json::json!({
                    "id": case.id,
                    "name": case.name,
                    "status": "blocked",
                    "error": "execute_commands is disabled; playwright execution skipped"
                }));
            }
        }
    }

    let total = case_refs.len() as u64;
    let status = if failed > 0 {
        "fail"
    } else if blocked > 0 && passed == 0 {
        "blocked"
    } else if blocked > 0 {
        "partial"
    } else {
        "pass"
    };

    let mut route_total = 0u64;
    let mut route_covered = 0u64;
    let mut button_total = 0u64;
    let mut button_covered = 0u64;
    let mut form_total = 0u64;
    let mut form_covered = 0u64;

    for case in &parsed_cases {
        let case_passed = passed_case_ids.contains(&case.id);
        for step in &case.steps {
            let action = step.action.trim().to_ascii_lowercase();
            match action.as_str() {
                "visit" | "assert-url-contains" | "api-request" => {
                    route_total += 1;
                    if case_passed {
                        route_covered += 1;
                    }
                }
                "click" => {
                    button_total += 1;
                    if case_passed {
                        button_covered += 1;
                    }
                }
                "fill" => {
                    form_total += 1;
                    if case_passed {
                        form_covered += 1;
                    }
                }
                _ => {}
            }
        }
    }

    let mut video_rel: Option<String> = None;
    let mut video_entries = tokio::fs::read_dir(&video_dir).await?;
    while let Some(entry) = video_entries.next_entry().await? {
        let path = entry.path();
        if !entry
            .file_type()
            .await
            .map(|ft| ft.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if ext == "webm" || ext == "mp4" {
            if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
                video_rel = Some(format!("video/{}", name));
                break;
            }
        }
    }

    let manifest = serde_json::json!({
        "created_at": chrono::Utc::now().to_rfc3339(),
        "status": status,
        "summary": {
            "total": total,
            "passed": passed,
            "failed": failed,
            "skipped": 0,
            "blocked": blocked
        },
        "tests": tests,
        "screenshots": screenshots,
        "video": {
            "path": video_rel
        }
    });

    let coverage = serde_json::json!({
        "route_total": route_total,
        "route_covered": route_covered,
        "button_total": button_total,
        "button_covered": button_covered,
        "form_total": form_total,
        "form_covered": form_covered,
        "functional_total": total,
        "functional_covered": passed
    });

    let safe_base_url = escape_html(&suite_base_url);
    let index_html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>toon run {run_id}</title></head><body><h1>TOON Run {run_id}</h1><p>Status: {status}</p><p>Passed: {passed}/{total}</p><p>Blocked: {blocked}</p><p>Failed: {failed}</p><p>Execution mode: Playwright case runner</p><p>Base URL: {safe_base_url}</p></body></html>"
    );

    tokio::fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .await?;
    tokio::fs::write(
        run_dir.join("coverage.json"),
        serde_json::to_vec_pretty(&coverage)?,
    )
    .await?;
    tokio::fs::write(run_dir.join("index.html"), index_html).await?;

    let screenshot_stats = artifact_stats(&screenshots_dir, &["png", "jpg", "jpeg", "webp"]).await;
    let video_stats = artifact_stats(&video_dir, &["mp4", "webm"]).await;
    let manifest_typed = serde_json::from_value::<TestReportManifest>(manifest.clone()).ok();
    let coverage_typed = serde_json::from_value::<TestReportCoverage>(coverage.clone()).ok();
    let missing_manifest_screenshot_files =
        manifest_missing_screenshot_files(&run_dir, manifest_typed.as_ref()).await;
    let (manifest_video_missing, manifest_video_zero_bytes) =
        manifest_video_path_flags(&run_dir, manifest_typed.as_ref()).await;
    let pass_rate = if total > 0 {
        ((passed as f64) * 100.0) / (total as f64)
    } else {
        0.0
    };
    let (tested_count, tested_scope, quality_warning) = build_tested_scope_and_quality(
        manifest_typed.as_ref(),
        coverage_typed.as_ref(),
        None,
        total,
        passed,
        failed,
        0,
        blocked,
        status,
        &screenshot_stats,
        &video_stats,
        missing_manifest_screenshot_files,
        manifest_video_missing,
        manifest_video_zero_bytes,
    );
    let issue = if failed > 0 {
        Some(format!("{} test(s) failed", failed))
    } else if blocked > 0 {
        Some(format!("{} test(s) blocked", blocked))
    } else {
        None
    };

    persist_report_row_to_db(
        &ctx,
        &ws,
        PersistReportRow {
            run_id: &run_id,
            status,
            pass_rate,
            total,
            passed,
            failed,
            skipped: 0,
            blocked,
            issue,
            html_url: Some(format!("/test-reports/{}/{}/index.html", ws.name, run_id)),
            tested_count,
            tested_scope,
            quality_warning,
            screenshot_files: screenshot_stats.files,
            video_files: video_stats.files,
            manifest_json: manifest.clone(),
            coverage_json: coverage.clone(),
        },
    )
    .await;

    info!(
        workspace_id = %ws.id,
        workspace = %ws.name,
        run_id = %run_id,
        status = status,
        passed = passed,
        failed = failed,
        blocked = blocked,
        "toon run completed"
    );

    Ok((
        StatusCode::CREATED,
        Json(RunToonSuiteResponse {
            run_id: run_id.clone(),
            status: status.to_string(),
            report_url: format!("/test-reports/{}/{}/index.html", ws.name, run_id),
            artifacts_dir: run_dir.to_string_lossy().to_string(),
            total_cases: total,
            passed_cases: passed,
            failed_cases: failed,
            blocked_cases: blocked,
        }),
    ))
}

async fn cancel_toon_run(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
) -> Result<Json<CancelToonRunResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let ws = resolve_workspace_by_id_or_name(&ctx, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("workspace '{id}' not found")))?;

    if let Some(pid) = terminate_workspace_runner_process(&ctx, &ws.id).await {
        return Ok(Json(CancelToonRunResponse {
            workspace_id: ws.id,
            cancelled: true,
            message: format!("cancel signal sent to active toon runner pid {}", pid),
        }));
    }

    let active_run = ctx
        .run_locks
        .lock()
        .map(|locks| locks.contains(&ws.id))
        .unwrap_or(false);
    if active_run {
        return Ok(Json(CancelToonRunResponse {
            workspace_id: ws.id,
            cancelled: false,
            message: "toon run is active but no cancellable runner process is currently attached"
                .to_string(),
        }));
    }

    Err(AppError::Conflict(
        "no active toon run for this workspace".to_string(),
    ))
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
        .update_workspace_status(&ws.id, WorkspaceStatus::Running)
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
        .update_workspace_status(&ws.id, WorkspaceStatus::Stopped)
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
    let mut stopped = Vec::new();

    for tunnel in existing_tunnels {
        if tunnel_mgr.stop_tunnel(&ws.id, tunnel.local_port).await {
            let _ = ctx.state.remove_tunnel(&ws.id, tunnel.local_port).await;
            stopped.push(tunnel.local_port);
        }
    }

    let workspace_path = ws.path.to_string_lossy();
    let _ = Command::new("pkill")
        .args(["-f", &workspace_path])
        .output()
        .await;

    stopped
}

async fn connect_report_db(ctx: &AppContext) -> Result<Option<tokio_postgres::Client>, AppError> {
    let Some(db_url) = ctx.config.report_db_url.as_deref() else {
        return Ok(None);
    };

    let (client, connection) = tokio_postgres::connect(db_url, NoTls)
        .await
        .map_err(|err| {
            AppError::Internal(anyhow::anyhow!(format!("db connect failed: {}", err)))
        })?;

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            warn!(error = %err, "report db connection closed");
        }
    });

    client
        .batch_execute(
            "
            CREATE TABLE IF NOT EXISTS toon_run_reports (
              run_id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              workspace_name TEXT NOT NULL,
              created_at TIMESTAMPTZ NOT NULL,
              status TEXT NOT NULL,
              pass_rate DOUBLE PRECISION NOT NULL,
              total BIGINT NOT NULL,
              passed BIGINT NOT NULL,
              failed BIGINT NOT NULL,
              skipped BIGINT NOT NULL,
              blocked BIGINT NOT NULL,
              issue TEXT,
              html_url TEXT,
              tested_count BIGINT,
              tested_scope TEXT,
              quality_warning TEXT,
              screenshot_files BIGINT NOT NULL DEFAULT 0,
              video_files BIGINT NOT NULL DEFAULT 0,
              manifest_json JSONB,
              coverage_json JSONB,
              updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_toon_run_reports_workspace_created
              ON toon_run_reports(workspace_id, created_at DESC);
            ",
        )
        .await
        .map_err(|err| {
            AppError::Internal(anyhow::anyhow!(format!("db schema init failed: {}", err)))
        })?;

    Ok(Some(client))
}

struct PersistReportRow<'a> {
    run_id: &'a str,
    status: &'a str,
    pass_rate: f64,
    total: u64,
    passed: u64,
    failed: u64,
    skipped: u64,
    blocked: u64,
    issue: Option<String>,
    html_url: Option<String>,
    tested_count: u64,
    tested_scope: Option<String>,
    quality_warning: Option<String>,
    screenshot_files: u64,
    video_files: u64,
    manifest_json: serde_json::Value,
    coverage_json: serde_json::Value,
}

async fn persist_report_row_to_db(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
    row: PersistReportRow<'_>,
) {
    let client = match connect_report_db(ctx).await {
        Ok(Some(client)) => client,
        Ok(None) => return,
        Err(err) => {
            warn!(workspace_id = %ws.id, error = %err, "report db disabled due to connection error");
            return;
        }
    };

    let query = "
      INSERT INTO toon_run_reports (
        run_id, workspace_id, workspace_name, created_at, status, pass_rate,
        total, passed, failed, skipped, blocked,
        issue, html_url, tested_count, tested_scope, quality_warning,
        screenshot_files, video_files, manifest_json, coverage_json, updated_at
      ) VALUES (
        $1,$2,$3,NOW(),$4,$5,$6,$7,$8,$9,$10,
        $11,$12,$13,$14,$15,$16,$17,$18,$19,NOW()
      )
      ON CONFLICT (run_id) DO UPDATE SET
        status=EXCLUDED.status,
        pass_rate=EXCLUDED.pass_rate,
        total=EXCLUDED.total,
        passed=EXCLUDED.passed,
        failed=EXCLUDED.failed,
        skipped=EXCLUDED.skipped,
        blocked=EXCLUDED.blocked,
        issue=EXCLUDED.issue,
        html_url=EXCLUDED.html_url,
        tested_count=EXCLUDED.tested_count,
        tested_scope=EXCLUDED.tested_scope,
        quality_warning=EXCLUDED.quality_warning,
        screenshot_files=EXCLUDED.screenshot_files,
        video_files=EXCLUDED.video_files,
        manifest_json=EXCLUDED.manifest_json,
        coverage_json=EXCLUDED.coverage_json,
        updated_at=NOW();
    ";

    if let Err(err) = client
        .execute(
            query,
            &[
                &row.run_id,
                &ws.id,
                &ws.name,
                &row.status,
                &row.pass_rate,
                &(row.total as i64),
                &(row.passed as i64),
                &(row.failed as i64),
                &(row.skipped as i64),
                &(row.blocked as i64),
                &row.issue,
                &row.html_url,
                &(row.tested_count as i64),
                &row.tested_scope,
                &row.quality_warning,
                &(row.screenshot_files as i64),
                &(row.video_files as i64),
                &row.manifest_json,
                &row.coverage_json,
            ],
        )
        .await
    {
        warn!(workspace_id = %ws.id, run_id = row.run_id, error = %err, "failed to persist report row to db");
    }
}

async fn workspace_test_runs_from_db(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
) -> Result<Option<Vec<TestReportRun>>, AppError> {
    let Some(client) = connect_report_db(ctx).await? else {
        return Ok(None);
    };

    let rows = client
        .query(
            "
            SELECT
              run_id,
              created_at,
              status,
              pass_rate,
              total,
              passed,
              failed,
              skipped,
              blocked,
              issue,
              html_url,
              tested_count,
              tested_scope,
              quality_warning,
              screenshot_files,
              video_files
            FROM toon_run_reports
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            LIMIT 500
            ",
            &[&ws.id],
        )
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!(format!("db query failed: {}", err))))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(TestReportRun {
            run_id: row.get::<_, String>(0),
            created_at: Some(row.get::<_, chrono::DateTime<chrono::Utc>>(1).to_rfc3339()),
            status: row.get::<_, String>(2),
            pass_rate: row.get::<_, f64>(3),
            total: row.get::<_, i64>(4).max(0) as u64,
            passed: row.get::<_, i64>(5).max(0) as u64,
            failed: row.get::<_, i64>(6).max(0) as u64,
            skipped: row.get::<_, i64>(7).max(0) as u64,
            blocked: row.get::<_, i64>(8).max(0) as u64,
            issue: row.get::<_, Option<String>>(9),
            html_url: row.get::<_, Option<String>>(10),
            tested_count: row.get::<_, i64>(11).max(0) as u64,
            tested_scope: row.get::<_, Option<String>>(12),
            quality_warning: row.get::<_, Option<String>>(13),
            screenshot_files: row.get::<_, i64>(14).max(0) as u64,
            video_files: row.get::<_, i64>(15).max(0) as u64,
        });
    }

    Ok(Some(out))
}

async fn list_test_reports(
    State(ctx): State<AppContext>,
) -> Result<Json<TestReportsResponse>, AppError> {
    sync_state_with_workspace_dirs(&ctx).await?;
    let list = ctx.state.list_workspaces().await;
    let mut out = Vec::with_capacity(list.len());

    for ws in &list {
        let runs = match workspace_test_runs_from_db(&ctx, ws).await {
            Ok(Some(db_runs)) if !db_runs.is_empty() => db_runs,
            Ok(Some(_)) | Ok(None) => workspace_test_runs(ws).await,
            Err(err) => {
                warn!(workspace_id = %ws.id, error = %err, "db report query failed; falling back to filesystem logs");
                workspace_test_runs(ws).await
            }
        };
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
    Regex::new(r"^(\d{8}-\d{6}(?:-\d{3})?|\d{4}-\d{2}-\d{2}\d{2}-\d{2}-\d{2})$")
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

    let run_dir_real = tokio::fs::canonicalize(&run_dir).await?;
    let target_real = tokio::fs::canonicalize(&target).await?;
    if !target_real.starts_with(&run_dir_real) {
        return Err(AppError::BadRequest("invalid report path".to_string()));
    }

    Ok(target_real)
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

async fn path_has_git_marker(path: &StdPath) -> bool {
    tokio::fs::try_exists(path.join(".git"))
        .await
        .unwrap_or(false)
}

async fn discover_workspace_repos(workspace_path: &StdPath) -> Vec<WorkspaceRepo> {
    let mut repos = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((workspace_path.to_path_buf(), 0usize));

    while let Some((dir, depth)) = queue.pop_front() {
        let dir_key = dir.to_string_lossy().to_string();
        if !seen.insert(dir_key.clone()) {
            continue;
        }

        if path_has_git_marker(&dir).await {
            let name = dir
                .file_name()
                .and_then(|v| v.to_str())
                .map(|v| v.to_string())
                .unwrap_or_else(|| "root".to_string());
            repos.push(WorkspaceRepo {
                name,
                path: dir_key,
            });
        }

        if depth >= 3 {
            continue;
        }

        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let Ok(ft) = entry.file_type().await else {
                continue;
            };
            if !ft.is_dir() {
                continue;
            }

            let path = entry.path();
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };

            if [
                ".git",
                "node_modules",
                "target",
                "build",
                ".venv",
                ".idea",
                ".vscode",
            ]
            .contains(&name)
            {
                continue;
            }

            queue.push_back((path, depth + 1));
        }
    }

    repos.sort_by(|a, b| a.path.cmp(&b.path));
    repos
}

async fn workspace_structure_status(workspace_path: &StdPath) -> WorkspaceStructureStatus {
    let required_paths = [
        "SETUP.md",
        "TEST.md",
        ".toon/schema.v1.toon",
        "setup/default.toon",
        "tests/smoke.toon",
        "test_cases/login-flow.toon",
    ];

    let mut missing = Vec::new();
    for required in required_paths {
        if !tokio::fs::try_exists(workspace_path.join(required))
            .await
            .unwrap_or(false)
        {
            missing.push(required.to_string());
        }
    }

    let mut legacy_template = false;
    let template_path = workspace_path.join("TEMPLATE.md");
    if tokio::fs::try_exists(&template_path).await.unwrap_or(false) {
        if let Ok(content) = tokio::fs::read_to_string(&template_path).await {
            if !content.to_lowercase().contains("deprecated") {
                legacy_template = true;
            }
        }
    }

    let stale = legacy_template;

    WorkspaceStructureStatus {
        ready: missing.is_empty() && !stale,
        stale,
        missing,
    }
}

async fn workspace_response_with_links(
    ctx: &AppContext,
    ws: &crate::state::Workspace,
) -> WorkspaceResponse {
    let tunnels = ctx.state.list_tunnels(&ws.id).await;
    let declared_ports = declared_frontend_ports(&ws.path).await;
    let repos = discover_workspace_repos(&ws.path).await;
    let structure_root = match resolve_workspace_target_root(ws).await {
        Ok(Some(path)) => path,
        Ok(None) => ws.path.clone(),
        Err(err) => {
            warn!(workspace_id = %ws.id, error = %err, "failed to resolve target root for structure status; falling back to workspace root");
            ws.path.clone()
        }
    };
    let structure = workspace_structure_status(&structure_root).await;
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
    response.repos = repos;
    response.structure = structure;
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

#[cfg(test)]
mod tests {
    use super::{
        execute_toon_case, normalize_playwright_case_status, parse_toon_case,
        resolve_toon_suite_base_url,
    };

    #[test]
    fn parse_toon_case_reads_context_and_steps() {
        let content = r#"context:
  kind: case
  version: 1.0
  id: FUNC-LOGIN-001
  task: login happy path
steps[3]{id,action,target,value}:
  1,visit,/login,
  2,fill,input[name=email],demo@example.com
  3,click,button[type=submit],
"#;

        let parsed = parse_toon_case("test_cases/login-flow.toon", content)
            .expect("case should parse successfully");
        assert_eq!(parsed.id, "FUNC-LOGIN-001");
        assert_eq!(parsed.name, "login happy path");
        assert_eq!(parsed.steps.len(), 3);
        assert_eq!(parsed.steps[0].action, "visit");
    }

    #[test]
    fn execute_toon_case_returns_pass_for_supported_steps() {
        let content = r#"context:
  kind: case
  version: 1.0
  id: FUNC-NAV-001
  task: dashboard navigation
steps[2]{id,action,target,value}:
  1,visit,/dashboard,
  2,assert-url-contains,/dashboard,
"#;

        let parsed = parse_toon_case("test_cases/nav.toon", content)
            .expect("case should parse successfully");
        let (status, error) = execute_toon_case(&parsed);
        assert_eq!(status, "pass");
        assert!(error.is_none());
    }

    #[test]
    fn execute_toon_case_returns_fail_when_assertion_misses() {
        let content = r#"context:
  kind: case
  version: 1.0
  id: FUNC-NAV-002
  task: route assertion
steps[2]{id,action,target,value}:
  1,visit,/dashboard,
  2,assert-url-contains,/settings,
"#;

        let parsed = parse_toon_case("test_cases/nav-miss.toon", content)
            .expect("case should parse successfully");
        let (status, error) = execute_toon_case(&parsed);
        assert_eq!(status, "fail");
        assert!(error
            .expect("expected failure error")
            .contains("expected url to contain"));
    }

    #[test]
    fn execute_toon_case_supports_api_assertions() {
        let content = r#"context:
  kind: case
  version: 1.0
  id: API-HEALTH-001
  task: health endpoint check
steps[3]{id,action,target,value}:
  1,api-request,/health,GET
  2,assert-status,200,
  3,assert-body-contains,ok,
"#;

        let parsed = parse_toon_case("test_cases/api-health.toon", content)
            .expect("case should parse successfully");
        let (status, error) = execute_toon_case(&parsed);
        assert_eq!(status, "pass");
        assert!(error.is_none());
    }

    #[test]
    fn parse_toon_case_rejects_unsupported_action() {
        let content = r#"context:
  kind: case
  version: 1.0
  id: API-BAD-001
  task: unsupported action
steps[1]{id,action,target,value}:
  1,do-random-thing,/x,
"#;

        let parsed = parse_toon_case("test_cases/bad.toon", content);
        match parsed {
            Ok(_) => panic!("parser should reject unsupported step action"),
            Err(err) => assert!(err.contains("unsupported step action")),
        }
    }

    #[test]
    fn parse_toon_case_rejects_api_request_payload_with_commas() {
        let content = r#"context:
  kind: case
  version: 1.0
  id: API-BAD-002
  task: comma payload rejection
steps[1]{id,action,target,value}:
  1,api-request,/api/test,POST|{"a":1,"b":2}
"#;

        let parsed = parse_toon_case("test_cases/bad-api.toon", content);
        match parsed {
            Ok(_) => panic!("parser should reject comma-separated API payload cells"),
            Err(err) => assert!(
                err.contains("extra comma-separated columns")
                    || err.contains("unsupported step action")
            ),
        }
    }

    #[test]
    fn resolve_base_url_from_suite_context() {
        let suite = r#"context:
  kind: suite
  version: 1.0
  base_url: https://code.example.com
cases[1]{id,file,required}:
  1,test_cases/login-flow.toon,true
"#;
        assert_eq!(
            resolve_toon_suite_base_url(suite),
            "https://code.example.com"
        );
    }

    #[test]
    fn normalize_playwright_status_maps_unknown_to_fail() {
        assert_eq!(normalize_playwright_case_status("pass"), "pass");
        assert_eq!(normalize_playwright_case_status("blocked"), "blocked");
        assert_eq!(normalize_playwright_case_status("timedOut"), "fail");
    }
}
