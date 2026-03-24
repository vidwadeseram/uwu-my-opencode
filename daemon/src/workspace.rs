use crate::config::AppConfig;
use crate::error::AppError;
use crate::supervisor::ProcessSupervisor;
use chrono::Utc;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tracing::{info, warn};

const TEMPLATE_CONTENT: &str = include_str!("../assets/TEMPLATE.md");
const SETUP_CONTENT: &str = include_str!("../assets/SETUP.md");
const SETUP_GUIDE_CONTENT: &str = include_str!("../assets/SETUP_GUIDE.md");
const TEST_CASES_CONTENT: &str = include_str!("../assets/TEST_CASES.md");

pub struct WorkspaceManager {
    config: AppConfig,
    supervisor: ProcessSupervisor,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandResult {
    pub command: String,
    pub executed: bool,
    pub success: Option<bool>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StartResult {
    pub commands: Vec<CommandResult>,
    pub browser_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TmuxTestLogResult {
    pub workspace: String,
    pub sessions: Vec<String>,
    pub log_file: String,
}

impl WorkspaceManager {
    pub fn new(config: AppConfig, supervisor: ProcessSupervisor) -> Self {
        Self { config, supervisor }
    }

    fn tmux(&self) -> &str {
        &self.config.tmux_bin
    }

    fn ttyd_spawn_cwd(&self, workspace_path: Option<&Path>) -> PathBuf {
        if let Some(path) = workspace_path {
            return path.to_path_buf();
        }

        if !self.config.workspace_root.as_os_str().is_empty() {
            return self.config.workspace_root.clone();
        }

        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"))
    }

    pub async fn create_directory(&self, workspace_path: &Path) -> Result<(), AppError> {
        tokio::fs::create_dir_all(workspace_path).await?;
        info!(path = %workspace_path.display(), "created workspace directory");
        Ok(())
    }

    pub fn tmux_session_name(workspace_name: &str) -> String {
        workspace_name
            .trim()
            .to_lowercase()
            .replace(' ', "-")
            .replace('_', "-")
    }

    fn legacy_tmux_session_name(workspace_name: &str) -> String {
        format!("uwu-{}", workspace_name)
    }

    fn ttyd_key(workspace_name: &str) -> String {
        format!("ttyd:{}", workspace_name)
    }

    fn setup_tmux_script(workspace_path: &Path) -> PathBuf {
        workspace_path.join("scripts").join("dev-tmux-session.sh")
    }

    async fn tmux_sessions_for_workspace(
        &self,
        workspace_path: &Path,
    ) -> Result<Vec<String>, AppError> {
        let tmux = self.tmux().to_string();
        let output = tokio::process::Command::new(&tmux)
            .args(["list-sessions", "-F", "#{session_name}\t#{session_path}"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| AppError::CommandFailed(format!("failed to list tmux sessions: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let workspace = workspace_path.to_string_lossy();
        let mut sessions = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(path) = parts.next() else {
                continue;
            };
            if path == workspace || path.starts_with(&format!("{}/", workspace)) {
                sessions.push(name.to_string());
            }
        }
        sessions.sort();
        sessions.dedup();
        Ok(sessions)
    }

    async fn run_cmd(&self, args: &[&str]) -> Result<CommandResult, AppError> {
        let cmd_str = args.join(" ");

        if !self.config.execute_commands {
            info!(command = %cmd_str, "dry-run");
            return Ok(CommandResult {
                command: cmd_str,
                executed: false,
                success: None,
                stdout: None,
                stderr: None,
            });
        }

        info!(command = %cmd_str, "executing");

        let output = tokio::process::Command::new(args[0])
            .args(&args[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::CommandFailed(format!("failed to run '{}': {}", cmd_str, e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            warn!(command = %cmd_str, stderr = %stderr, "command failed");
        }

        Ok(CommandResult {
            command: cmd_str,
            executed: true,
            success: Some(output.status.success()),
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }

    async fn list_workspace_dirs(&self) -> Result<Vec<PathBuf>, AppError> {
        tokio::fs::create_dir_all(&self.config.workspace_root).await?;

        let mut entries = tokio::fs::read_dir(&self.config.workspace_root).await?;
        let mut dirs = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }

        dirs.sort();
        if dirs.is_empty() {
            let default_dir = self.config.workspace_root.join("workspace-1");
            tokio::fs::create_dir_all(&default_dir).await?;
            dirs.push(default_dir);
        }

        Ok(dirs)
    }

    async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
        tokio::fs::create_dir_all(dst).await?;
        let mut entries = tokio::fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                Box::pin(Self::copy_dir_recursive(&src_path, &dst_path)).await?;
            } else {
                if let Some(parent) = dst_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::copy(&src_path, &dst_path).await?;
            }
        }
        Ok(())
    }

    pub async fn bootstrap_linux_editor_configs(&self) -> Result<Vec<CommandResult>, AppError> {
        if std::env::consts::OS != "linux" {
            return Ok(Vec::new());
        }

        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| AppError::BadRequest("HOME is not set".to_string()))?;

        let tmux_target = home.join(".tmux.conf");
        let nvim_target = home.join(".config").join("nvim");
        let oh_my_zsh_target = home.join(".oh-my-zsh");
        let zshrc_target = home.join(".zshrc");
        let zsh_custom_target = oh_my_zsh_target.join("custom");
        let autosuggest_target = zsh_custom_target
            .join("plugins")
            .join("zsh-autosuggestions");
        let syntax_highlight_target = zsh_custom_target
            .join("plugins")
            .join("zsh-syntax-highlighting");
        let completions_target = zsh_custom_target.join("plugins").join("zsh-completions");

        let tmux_missing = !tmux_target.exists();
        let nvim_missing = !nvim_target.exists();
        let oh_my_zsh_missing = !oh_my_zsh_target.exists();
        let zshrc_missing = !zshrc_target.exists();
        let autosuggest_missing = !autosuggest_target.exists();
        let syntax_highlight_missing = !syntax_highlight_target.exists();
        let completions_missing = !completions_target.exists();

        if !tmux_missing
            && !nvim_missing
            && !oh_my_zsh_missing
            && !zshrc_missing
            && !autosuggest_missing
            && !syntax_highlight_missing
            && !completions_missing
        {
            return Ok(Vec::new());
        }

        let cache_root = home.join(".cache").join("uwu-dotfiles");
        let dotfiles_repo = "https://github.com/vidwadeseram/dotfiles.git";
        let cache_root_str = cache_root.to_string_lossy().to_string();

        let mut commands = Vec::new();
        if cache_root.exists() {
            commands.push(
                self.run_cmd(&["git", "-C", &cache_root_str, "pull", "--ff-only"])
                    .await?,
            );
        } else {
            commands.push(
                self.run_cmd(&[
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    dotfiles_repo,
                    &cache_root_str,
                ])
                .await?,
            );
        }

        let tmux_src = cache_root
            .join("tmux")
            .join(".config")
            .join("tmux")
            .join("tmux.conf");
        let nvim_src = cache_root.join("nvim").join(".config").join("nvim");

        if tmux_missing && tmux_src.exists() {
            tokio::fs::copy(&tmux_src, &tmux_target).await?;
            commands.push(CommandResult {
                command: format!(
                    "copy {} {}",
                    tmux_src.to_string_lossy(),
                    tmux_target.to_string_lossy()
                ),
                executed: true,
                success: Some(true),
                stdout: None,
                stderr: None,
            });
        }

        if nvim_missing && nvim_src.exists() {
            if let Some(parent) = nvim_target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            Self::copy_dir_recursive(&nvim_src, &nvim_target).await?;
            commands.push(CommandResult {
                command: format!(
                    "copy {} {}",
                    nvim_src.to_string_lossy(),
                    nvim_target.to_string_lossy()
                ),
                executed: true,
                success: Some(true),
                stdout: None,
                stderr: None,
            });
        }

        if oh_my_zsh_missing {
            commands.push(
                self.run_cmd(&[
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/ohmyzsh/ohmyzsh.git",
                    &oh_my_zsh_target.to_string_lossy(),
                ])
                .await?,
            );
        }

        if autosuggest_missing {
            if let Some(parent) = autosuggest_target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            commands.push(
                self.run_cmd(&[
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/zsh-users/zsh-autosuggestions",
                    &autosuggest_target.to_string_lossy(),
                ])
                .await?,
            );
        }

        if syntax_highlight_missing {
            if let Some(parent) = syntax_highlight_target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            commands.push(
                self.run_cmd(&[
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/zsh-users/zsh-syntax-highlighting.git",
                    &syntax_highlight_target.to_string_lossy(),
                ])
                .await?,
            );
        }

        if completions_missing {
            if let Some(parent) = completions_target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            commands.push(
                self.run_cmd(&[
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/zsh-users/zsh-completions",
                    &completions_target.to_string_lossy(),
                ])
                .await?,
            );
        }

        if zshrc_missing {
            let zshrc = "export ZSH=\"$HOME/.oh-my-zsh\"\nZSH_THEME=\"robbyrussell\"\nplugins=(git zsh-autosuggestions zsh-syntax-highlighting zsh-completions)\nif [ -f \"$ZSH/oh-my-zsh.sh\" ]; then\n  source \"$ZSH/oh-my-zsh.sh\"\nfi\nif command -v direnv >/dev/null 2>&1; then\n  eval \"$(direnv hook zsh)\"\nfi\n\n# Nested tmux helper - allows attaching to sessions from within tmux\nta() {\n    if [ -n \"$TMUX\" ]; then\n        TMUX= tmux attach -t \"$1\"\n    else\n        tmux attach -t \"$1\"\n    fi\n}\n";
            tokio::fs::write(&zshrc_target, zshrc).await?;
            commands.push(CommandResult {
                command: format!("write {}", zshrc_target.to_string_lossy()),
                executed: true,
                success: Some(true),
                stdout: None,
                stderr: None,
            });
        }

        Ok(commands)
    }

    fn shell_quote(value: &str) -> String {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }

    pub async fn setup_workspace_opencode_files(&self, dir: &Path) -> Result<(), AppError> {
        let opencode_dir = dir.join(".opencode");
        let plugins_dir = opencode_dir.join("plugins");
        let commands_dir = opencode_dir.join("command");
        let scripts_dir = dir.join("scripts");
        let workspace_docs_dir = dir.join("workspace-docs");

        let template_file = dir.join("TEMPLATE.md");
        let setup_file = dir.join("SETUP.md");

        let docs_template_file = workspace_docs_dir.join("TEMPLATE.md");
        let docs_setup_file = workspace_docs_dir.join("SETUP.md");
        let docs_test_cases_file = workspace_docs_dir.join("TEST_CASES.md");

        let frontends_manifest_file = opencode_dir.join("frontends.json");

        tokio::fs::create_dir_all(&plugins_dir).await?;
        tokio::fs::create_dir_all(&commands_dir).await?;
        tokio::fs::create_dir_all(&scripts_dir).await?;
        tokio::fs::create_dir_all(&workspace_docs_dir).await?;

        if !tokio::fs::try_exists(&docs_template_file).await? {
            tokio::fs::write(&docs_template_file, TEMPLATE_CONTENT).await?;
        } else {
            let existing = tokio::fs::read_to_string(&docs_template_file)
                .await
                .unwrap_or_default();
            if !existing.contains("# Workspace Test Template (Compact)")
                || !existing.contains("Coverage is only considered complete when route/button/form/functional totals are explicitly recorded")
                || !existing.contains("11. **Stable capture rule (required before screenshot/pass)**")
                || !existing.contains("logs/{run_id}/coverage.json")
                || !existing.contains("--repo` filter for multi-repo workspaces")
                || !existing.contains("Route-visit-only coverage is NOT sufficient")
                || !existing.contains("FUNC-*")
            {
                tokio::fs::write(&docs_template_file, TEMPLATE_CONTENT).await?;
            }
        }

        if !tokio::fs::try_exists(&docs_setup_file).await? {
            tokio::fs::write(&docs_setup_file, SETUP_GUIDE_CONTENT).await?;
        } else {
            let existing = tokio::fs::read_to_string(&docs_setup_file)
                .await
                .unwrap_or_default();
            if !existing.contains("## PostgreSQL bootstrap (required before API start)")
                || !existing.contains("## API env normalization (required)")
                || !existing.contains("## Regression report artifact validation")
                || !existing.contains("coverage.json")
                || !existing.contains("spinner/skeleton/blank placeholder")
                || !existing.contains("Video recording placeholder")
                || !existing.contains("wrong page name like `junk-qr-payments`")
                || !existing.contains("/start-test --repo <repo-path-or-name>")
                || !existing.contains("workspace root is not a git repo")
                || !existing
                    .contains("## Test data seeding (required before deep functional tests)")
                || !existing.contains("ensure-superadmin.sh")
                || !existing.contains("functional_total")
                || !existing.contains(
                    "coverage functional_covered must equal functional_total for exhaustive run",
                )
            {
                tokio::fs::write(&docs_setup_file, SETUP_GUIDE_CONTENT).await?;
            }
        }

        if !tokio::fs::try_exists(&docs_test_cases_file).await? {
            tokio::fs::write(&docs_test_cases_file, TEST_CASES_CONTENT).await?;
        } else {
            let existing = tokio::fs::read_to_string(&docs_test_cases_file)
                .await
                .unwrap_or_default();
            if !existing.contains("# allinonepos - Exhaustive Test Cases")
                || !existing.contains("## 2) Route Inventory (Source of Truth)")
                || !existing.contains("ROUTE-<route_key>")
                || !existing.contains("## 8.1) Required `coverage.json`")
                || !existing.contains("index.html still contains video placeholder text")
                || !existing.contains("LOG-004")
                || !existing.contains("Every `FAIL` and `BLOCKED` test must have at least one screenshot evidence entry.")
                || !existing.contains("## 12) Deep Functional Test Scenarios")
                || !existing.contains("FUNC-KYC-006")
                || !existing.contains("## 13) Functional Test Execution Contract")
                || !existing.contains("functional_total")
                || !existing.contains("functional_covered == functional_total")
            {
                tokio::fs::write(&docs_test_cases_file, TEST_CASES_CONTENT).await?;
            }
        }

        if !tokio::fs::try_exists(&template_file).await? {
            tokio::fs::write(&template_file, TEMPLATE_CONTENT).await?;
        } else {
            let existing = tokio::fs::read_to_string(&template_file)
                .await
                .unwrap_or_default();
            if existing.contains("## SECTION 1: MERCHANT PORTAL - ALL SECTIONS")
                || existing.contains("## RESULTS OUTPUT FORMAT")
                || existing.contains("logs/{YYYY-MM-DD}{HH-MM-SS}.md")
            {
                tokio::fs::write(&template_file, TEMPLATE_CONTENT).await?;
            }
        }

        if !tokio::fs::try_exists(&setup_file).await? {
            tokio::fs::write(&setup_file, SETUP_CONTENT).await?;
        } else {
            let existing = tokio::fs::read_to_string(&setup_file)
                .await
                .unwrap_or_default();
            if existing.contains("This guide explains how to create a tmux session script")
                || existing.contains("## PostgreSQL bootstrap (required before API start)")
                || existing.contains("## Start required backend APIs (tmux session contract)")
                || existing.contains("## Regression report artifact validation")
            {
                tokio::fs::write(&setup_file, SETUP_CONTENT).await?;
            }
        }

        if !tokio::fs::try_exists(&frontends_manifest_file).await? {
            let frontends_manifest = r#"{
  "frontends": [
    {
      "name": "web",
      "port": 3000,
      "description": "main frontend"
    }
  ]
}
"#;
            tokio::fs::write(&frontends_manifest_file, frontends_manifest).await?;
        }

        let oh_my_repo = self.config.oh_my_opencode_repo.clone();
        let mut oh_my_src_path = oh_my_repo.join("src").join("index.ts");
        if !oh_my_src_path.exists() {
            if let Some(parent) = oh_my_repo.parent() {
                for candidate_name in ["oh-my-openagent", "oh-my-opencode"] {
                    let candidate_repo = parent.join(candidate_name);
                    let candidate_src = candidate_repo.join("src").join("index.ts");
                    if candidate_src.exists() {
                        oh_my_src_path = candidate_src;
                        break;
                    }
                }
            }
        }
        let oh_my_src = oh_my_src_path.to_string_lossy().to_string();

        let plugin_file = plugins_dir.join("oh-my-opencode.ts");
        let plugin_content = format!(
            "import OhMyOpenCodePlugin from \"{}\";\nexport default OhMyOpenCodePlugin;\n",
            oh_my_src.replace('\\', "\\\\")
        );
        tokio::fs::write(plugin_file, plugin_content).await?;

        let oac_opencode_dir = self.config.openagentscontrol_repo.join(".opencode");
        if oac_opencode_dir.exists() {
            let dirs_to_copy = [
                "agent", "command", "skill", "context", "tool", "prompts", "profiles",
            ];
            for dir_name in &dirs_to_copy {
                let src_dir = oac_opencode_dir.join(dir_name);
                if src_dir.exists() {
                    let dest_dir = opencode_dir.join(dir_name);
                    if let Err(e) = Self::copy_dir_recursive(&src_dir, &dest_dir).await {
                        warn!(
                            src = %src_dir.display(),
                            dest = %dest_dir.display(),
                            error = %e,
                            "failed to copy OpenAgentsControl directory"
                        );
                    }
                }
            }
        }

        let host_project_file = commands_dir.join("host-project.md");
        let host_project_content = r#"---
description: host current project and provide a URL reachable from my PC
subtask: false
---

Host this project for preview.

Context:
- opencode is running on a remote Linux server via ttyd.
- The final preview URL must be reachable from my PC browser, not only from the server itself.
- Use any available model in this environment (do not depend on one specific model).

Milestone-first workflow:
- When the user runs `/milestones`, first create a thoughtful implementation plan with milestones and linked issues/tasks.
- Milestones and issues must map directly to the user request and include explicit technology choices (framework, runtime, package manager, deployment approach).
- Keep each issue scoped, testable, and ordered by dependency.

Build execution workflow:
- When the user runs `/start-building`, execute the milestone issues in order and start implementation immediately.
- Track progress issue-by-issue, and do not stop until all milestone issues are completed.
- As each issue and milestone completes, update project documentation (README/changelog or relevant docs) with what changed and why.
- When a milestone completes for a web app, provide the currently hosted preview URL. If missing, run `/host-project` to produce one and report it.
- If model/token quota is exhausted, preserve progress context and continue from the first incomplete issue after quota refresh.

Required flow (do exactly):

1) Build milestones/issues from request
- Parse the user request into 3-8 milestone buckets.
- For each milestone, define concrete issues with acceptance criteria and tech decisions.
- Announce the milestone plan before writing code.

2) Find PROJECT_DIR (any stack, not only JS)
- If current dir has one of: package.json, pyproject.toml, requirements.txt, go.mod, Cargo.toml, Gemfile, composer.json, index.html -> use current dir.
- Otherwise search child dirs up to depth 3 for those files and pick the nearest match.
- Print PROJECT_DIR before running anything.

3) Pick port and bind address
- Use port 3000 first, if busy use 3001.
- Start server on 0.0.0.0 when possible so it can be reached externally.

4) Start dev server in a named tmux window
- Always create a dedicated tmux window before launching the server:
  tmux new-window -t uwu-main -n "host-preview"
- Run dependency install + server command inside that "host-preview" window so both user and AI can see logs.
- After hosting, report how to attach:
  tmux select-window -t uwu-main:host-preview

5) Install deps + start command by stack
- Node/JS/TS (package.json): detect package manager from lockfile, install, then run script in order dev -> start -> preview.
- Python (pyproject.toml/requirements.txt): install deps (pip/poetry/uv) and run framework dev server on chosen port.
- Go (go.mod): go run/build and serve on chosen port if app supports PORT.
- Rust (Cargo.toml): cargo run with chosen port env if supported.
- Static site (index.html only): python3 -m http.server <PORT> --bind 0.0.0.0.

6) Error recovery
- If output contains dependency errors such as 'Cannot find module', 'Module not found', 'ERR_MODULE_NOT_FOUND', missing package/import, or command not found:
  a) stop process
  b) run dependency install/fix for that stack
  c) retry once

7) Verify local service
- Wait until curl http://127.0.0.1:<PORT> succeeds (HTTP 200-399) or timeout.

8) Make it reachable from my PC
- If cloudflared exists, run a quick tunnel to http://127.0.0.1:<PORT> and print the public https://*.trycloudflare.com URL.
- If cloudflared is unavailable, use another available tunnel/proxy approach and print the reachable URL.
- As a fallback, print the server public IP/domain with port and clear firewall/security-group steps.

9) Return concise result
- Always return: stack detected, PROJECT_DIR, local URL, public URL, command used to run app, tmux window name (host-preview), and current status.
- If unresolved, return exact failing command + last 40 lines of stderr and the next fix you will attempt.
"#;
        tokio::fs::write(host_project_file, host_project_content).await?;

        let milestones_file = commands_dir.join("milestones.md");
        let milestones_content = r#"---
description: create milestone and issue plan from the active request
subtask: false
---

Create a milestone-first implementation plan for the active request.

Requirements:
- Produce 3-8 milestones that map directly to the request.
- For each milestone, define concrete issues/tasks with acceptance criteria.
- Explicitly choose technologies (framework, runtime, package manager, deployment approach).
- Order issues by dependency and call out critical path items.
- Keep issues scoped, testable, and implementation-ready.

Output format:
1) Milestone summary (name + outcome)
2) Issue list per milestone
3) Technology decisions and rationale
4) Execution order
5) Definition of done per milestone
"#;
        tokio::fs::write(milestones_file, milestones_content).await?;

        let start_building_file = commands_dir.join("start-building.md");
        let start_building_content = r#"---
description: execute milestone issues until completion with docs and hosted URL updates
subtask: false
---

Execute the planned milestone issues and keep building until all milestones are completed.

Execution rules:
- If no plan exists, run `/milestones` first, then continue.
- Execute issues in dependency order and track issue-by-issue progress.
- Do not stop until all milestone issues are done.
- After each completed issue and milestone, update project documentation (README/changelog or relevant docs).
- For web apps, report the current hosted preview URL when a milestone completes.
- If no hosted URL exists yet, run `/host-project`, then report the URL.
- If token/subscription quota is exhausted, preserve progress context and resume from the first incomplete issue after quota refresh.

Required output while running:
1) Current milestone and active issue
2) Completed issues since last update
3) Documentation updates made
4) Hosted URL status (for web apps)
5) Remaining milestones and ETA
"#;
        tokio::fs::write(start_building_file, start_building_content).await?;

        let run_project_file = commands_dir.join("run-project.md");
        let run_project_content = r#"---
description: run a project in its own dedicated terminal and return view URL
subtask: false
---

Run the requested project in a dedicated runtime terminal separate from the workspace terminal.

Rules:
- Do not run the app in the workspace's main tmux pane.
- Create a dedicated tmux session for runtime logs:
  tmux new-session -d -s run-<project-slug> -c <PROJECT_DIR>
- Start the app inside that session and keep it attached to runtime output.
- Start a dedicated ttyd instance for that runtime session on an available port (prefer 8800+, increment if used).
- Return terminal URL as `/terminal/<PORT>/` so users can watch logs from browser.

Project URL requirements:
- If the app exposes a web server, return a direct view URL.
- If a tunnel is needed, create one and return the public URL.
- Always return both:
  1) View URL (the app)
  2) Runtime terminal URL (`/terminal/<PORT>/`)

Output format:
1) Project detected and start command used
2) Runtime tmux session name
3) Runtime terminal URL
4) Project view URL
5) Health check result
"#;
        tokio::fs::write(run_project_file, run_project_content).await?;

        let start_test_file = commands_dir.join("start-test.md");
        let start_test_content = r#"---
description: run exhaustive tests on main, a branch, or PR URL branches and return report links
subtask: false
---

Start an exhaustive regression test workflow for this workspace.

Argument contract:
- `/start-test` -> default target branch: `main`
- `/start-test <branch-name>` -> test that branch
- `/start-test --branch <branch-name>` -> test that branch
- `/start-test <pr-url> [<pr-url> ...]` -> resolve each PR head branch and test all
- `/start-test --repo <repo-path-or-name> [targets...]` -> limit execution to one repo inside a multi-repo workspace

Accepted PR URL format:
- `https://github.com/<owner>/<repo>/pull/<number>`

Required execution rules:
1) Parse targets from arguments in order.
   - If no targets are provided, use `main`.
   - If branch names and PR URLs are mixed, run in the same order provided.

2) Discover git repositories for this workspace.
   - Do NOT assume workspace root is a git repo.
   - If `git rev-parse --is-inside-work-tree` succeeds at workspace root, include workspace root as a repo target.
   - Also discover nested repos with `.git` directories (microservice layout).
   - If no git repos are found, stop with a clear error that includes scanned paths.
   - If `--repo` was provided, filter discovered repos to the matching path/name only.

3) Validate git preconditions per discovered repo before switching targets:
   - Run `git -C <repo> status --porcelain`; stop on dirty repo and report exact repo path.
   - Record pre-test state per repo:
     - `git -C <repo> rev-parse --abbrev-ref HEAD`
     - `git -C <repo> rev-parse HEAD`
   - Always restore every repo to its original state at the end.

4) Resolve and switch target with repo-aware mapping:
   - Branch target:
     - Apply branch switch per selected repo.
     - `git -C <repo> fetch origin <branch>`
     - `git -C <repo> checkout <branch>` if it exists locally, otherwise `git -C <repo> checkout -b <branch> --track origin/<branch>`
     - If branch does not exist for a repo, report that repo as skipped with reason.
   - PR URL target:
     - Resolve metadata with `gh pr view <url> --json number,headRefName,headRepositoryOwner,headRepository`
     - Map PR repo (`owner/name`) to a discovered local repo by inspecting `git -C <repo> remote get-url origin`
     - Run `gh pr checkout <url>` from the mapped repo directory only.
     - If no repo match exists, mark that PR target as blocked with explicit reason.

5) For each switched target, run full test contract from workspace docs:
    - Follow `workspace-docs/SETUP.md` preflight and infra checks.
    - Execute exhaustive coverage from `workspace-docs/TEST_CASES.md`.
    - Route-visit checks are NOT enough; execute deep functional workflows from Section 12.
    - Required functional examples (must run and be recorded):
      - add user/employee and verify it appears in list/detail
      - add item and verify it appears in item list
      - full KYC lifecycle: merchant submit -> super admin approve/reject -> merchant verify status
      - CRUD flows for items/categories/customers/employees with valid + invalid form paths
    - Include `FUNC-*` test IDs in `manifest.json` and functional totals in `coverage.json`.
    - Enforce quality gates:
       - no PASS on 404/error/loading evidence
       - no placeholder video text in `index.html`
       - full-process video file must exist and be non-zero bytes
       - dashboard after login must be stable before PASS (visible heading + loaded content + no auth redirect)

6) Collect and report run outputs per target:
    - run id
    - final status (pass/fail/partial/blocked)
    - report URL: `/test-reports/{workspace}/{run_id}/index.html`
    - repo scope used for the target (single repo or list)

7) Restore original branch/ref for all touched repos, even if one target fails.

Output format:
1) Parsed targets
2) Discovered repos and repo filter result
3) Per-target switch result (branch/PR metadata + mapped repo)
4) Per-target test result (run id, status, report URL)
5) Final restored branch/ref per repo
6) Blockers and exact failing command output (if any)
"#;
        tokio::fs::write(start_test_file, start_test_content).await?;

        let publish_frontends_file = commands_dir.join("publish-frontends.md");
        let publish_frontends_content = r#"---
description: publish configured frontend ports and return clickable hosted URLs
subtask: false
---

Publish hosted frontend links for this workspace.

Framework:
- Source of truth is `.opencode/frontends.json`.
- Each entry must include a `port` number.
- Use `./scripts/publish-frontends.sh` to publish all configured ports.

Required output:
1) Ports discovered from manifest
2) Hosted URLs created (if cloudflared is available)
3) Local fallback URLs per port
4) Any ports skipped and why
"#;
        tokio::fs::write(publish_frontends_file, publish_frontends_content).await?;

        let tmux_test_log_file = commands_dir.join("tmux-test-log.md");
        let tmux_test_log_content = r#"---
description: create tmux test logs for this workspace and list saved files
subtask: false
---

Create a reproducible tmux test log for this workspace.

Rules:
- Prefer using the workspace script: `./scripts/tmux-test-log.sh`.
- If missing, fall back to `tmux capture-pane` commands and write logs under `logs/tmux/`.
- Capture from tmux session named as the workspace.
- Capture at least the last 2000 lines per window.

Output format:
1) tmux sessions captured
2) log file path(s)
3) timestamp
4) next verification command to inspect log output
"#;
        tokio::fs::write(tmux_test_log_file, tmux_test_log_content).await?;

        let ensure_superadmin_file = commands_dir.join("ensure-superadmin.md");
        let ensure_superadmin_content = r#"---
description: ensure SuperAdmin account exists in DB and reset password if login fails
subtask: false
---

If SuperAdmin login fails, ensure the DB account exists and set a known password.

Flow:
1) Run `./scripts/ensure-superadmin.sh "SuperAdmin" "Alpha23@$"`.
2) Confirm script output says account was created or password was reset.
3) Re-test login with `SuperAdmin` / `Alpha23@$`.
4) Record exact command output in test logs.

If script fails:
- Return the exact failing DB command and the missing environment variable.
- Do not continue admin tests until fixed.
"#;
        tokio::fs::write(ensure_superadmin_file, ensure_superadmin_content).await?;

        let publish_frontends_script = scripts_dir.join("publish-frontends.sh");
        if !tokio::fs::try_exists(&publish_frontends_script).await? {
            let script = r###"#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WS_NAME="$(basename "${ROOT_DIR}")"
DAEMON_URL="${UWU_DAEMON_URL:-http://127.0.0.1:18080}"
DAEMON_USER="${UWU_DAEMON_USER:-admin}"
DAEMON_PASS="${UWU_DAEMON_PASS:-admin}"
MANIFEST="${ROOT_DIR}/.opencode/frontends.json"

if [[ ! -f "${MANIFEST}" ]]; then
  echo "missing manifest: ${MANIFEST}" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to parse ${MANIFEST}" >&2
  exit 1
fi

PORTS="$(python3 - "${MANIFEST}" <<'PY'
import json,sys
path=sys.argv[1]
with open(path, 'r', encoding='utf-8') as f:
    data=json.load(f)
ports=[]
for item in data.get('frontends', []):
    port=item.get('port')
    if isinstance(port, int) and port > 0:
        ports.append(str(port))
print(' '.join(sorted(set(ports), key=lambda x:int(x))))
PY
)"

if [[ -z "${PORTS}" ]]; then
  echo "no frontend ports found in ${MANIFEST}" >&2
  exit 1
fi

for port in ${PORTS}; do
  echo "publishing port ${port} for workspace ${WS_NAME}"
  curl -fsSL -u "${DAEMON_USER}:${DAEMON_PASS}" \
    -H "content-type: application/json" \
    -X POST "${DAEMON_URL}/api/workspaces/${WS_NAME}/previews" \
    -d "{\"port\":${port}}"
  echo
done

echo "done. check links in dashboard Running Projects." 
"###;
            tokio::fs::write(&publish_frontends_script, script).await?;
            let publish_frontends_script_str =
                publish_frontends_script.to_string_lossy().to_string();
            let _ = self
                .run_cmd(&["chmod", "+x", &publish_frontends_script_str])
                .await;
        }

        let ensure_superadmin_script = scripts_dir.join("ensure-superadmin.sh");
        let script = r###"#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
USERNAME="${1:-SuperAdmin}"
PASSWORD="${2:-Alpha23@$}"

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is not installed" >&2
  exit 1
fi

if [[ -z "${POSTGRESQL_DSL:-}" ]]; then
  if [[ -f "${ROOT_DIR}/pos-super-admin-api/.envrc" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "${ROOT_DIR}/pos-super-admin-api/.envrc"
    set +a
  fi
fi

if [[ -z "${POSTGRESQL_DSL:-}" ]]; then
  echo "POSTGRESQL_DSL is not set. Export it or define it in pos-super-admin-api/.envrc" >&2
  exit 1
fi

if ! psql "${POSTGRESQL_DSL}" -c 'SELECT 1;' >/dev/null 2>&1; then
  if command -v systemctl >/dev/null 2>&1 && systemctl list-unit-files | grep -q '^postgresql.service'; then
    systemctl start postgresql || true
  fi
fi

if ! psql "${POSTGRESQL_DSL}" -c 'SELECT 1;' >/dev/null 2>&1; then
  echo "Cannot connect to database using POSTGRESQL_DSL=${POSTGRESQL_DSL}" >&2
  echo "Ensure PostgreSQL is running and the URL/user/password are correct." >&2
  exit 1
fi

psql "${POSTGRESQL_DSL}" -v super_user="${USERNAME}" -v super_pass="${PASSWORD}" <<'SQL'
CREATE EXTENSION IF NOT EXISTS pgcrypto;

UPDATE users
SET
  password = crypt(:'super_pass', gen_salt('bf')),
  is_deleted = FALSE,
  updated_at = CURRENT_TIMESTAMP
WHERE user_name = :'super_user';

INSERT INTO users (
  id,
  name,
  user_name,
  mobile_number,
  email,
  password,
  is_deleted,
  created_at,
  updated_at
)
SELECT
  gen_random_uuid(),
  'Super Admin',
  :'super_user',
  '+94761112224',
  'superadmin@example.com',
  crypt(:'super_pass', gen_salt('bf')),
  FALSE,
  CURRENT_TIMESTAMP,
  CURRENT_TIMESTAMP
WHERE NOT EXISTS (
  SELECT 1 FROM users WHERE user_name = :'super_user'
);
SQL

echo "SuperAdmin account ensured for user '${USERNAME}'."
"###;
        tokio::fs::write(&ensure_superadmin_script, script).await?;
        let ensure_superadmin_script_str = ensure_superadmin_script.to_string_lossy().to_string();
        let _ = self
            .run_cmd(&["chmod", "+x", &ensure_superadmin_script_str])
            .await;

        let dev_tmux_script = scripts_dir.join("dev-tmux-session.sh");
        if !tokio::fs::try_exists(&dev_tmux_script).await? {
            let script = r###"#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SESSION_NAME="${MYAPP_TMUX_SESSION_NAME:-$(basename "${ROOT_DIR}")}"

if [[ "${SESSION_NAME}" == "uwu-main" ]]; then
  SESSION_NAME="$(basename "${ROOT_DIR}")"
fi

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is not installed" >&2
  exit 1
fi

if tmux has-session -t "${SESSION_NAME}" 2>/dev/null; then
  echo "tmux session ${SESSION_NAME} already exists"
  exit 0
fi

tmux new-session -d -s "${SESSION_NAME}" -n "app" -c "${ROOT_DIR}"
tmux send-keys -t "${SESSION_NAME}:app" "echo 'Update scripts/dev-tmux-session.sh with your real service commands from workspace-docs/SETUP.md'" C-m

echo "tmux session ${SESSION_NAME} created"
echo "attach with: tmux attach -t ${SESSION_NAME}"
"###;
            tokio::fs::write(&dev_tmux_script, script).await?;
            let dev_tmux_script_str = dev_tmux_script.to_string_lossy().to_string();
            let _ = self.run_cmd(&["chmod", "+x", &dev_tmux_script_str]).await;
        }

        let tmux_log_script = scripts_dir.join("tmux-test-log.sh");
        if !tokio::fs::try_exists(&tmux_log_script).await? {
            let script = r###"#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SESSION_NAME="${1:-${MYAPP_TMUX_SESSION_NAME:-$(basename "${ROOT_DIR}")}}"

if [[ "${SESSION_NAME}" == "uwu-main" ]]; then
  SESSION_NAME="$(basename "${ROOT_DIR}")"
fi
LOG_DIR="${ROOT_DIR}/logs/tmux"
mkdir -p "${LOG_DIR}"

STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${LOG_DIR}/tmux-test-${SESSION_NAME}-${STAMP}.log"

if ! tmux has-session -t "${SESSION_NAME}" 2>/dev/null; then
  echo "session not found: ${SESSION_NAME}" >&2
  exit 1
fi

{
  echo "# tmux test log"
  echo "session=${SESSION_NAME}"
  echo "timestamp=$(date -Iseconds)"
  echo
} >"${OUT}"

while IFS= read -r win; do
  idx="${win%%$'\t'*}"
  name="${win#*$'\t'}"
  {
    echo "----- ${SESSION_NAME}:${idx} (${name}) -----"
    tmux capture-pane -pt "${SESSION_NAME}:${idx}.0" -S -2000
    echo
  } >>"${OUT}"
done < <(tmux list-windows -t "${SESSION_NAME}" -F "#{window_index}\t#{window_name}")

echo "created ${OUT}"
"###;
            tokio::fs::write(&tmux_log_script, script).await?;
            let tmux_log_script_str = tmux_log_script.to_string_lossy().to_string();
            let _ = self.run_cmd(&["chmod", "+x", &tmux_log_script_str]).await;
        }

        Ok(())
    }

    fn opencode_launch_command(&self, dir: &Path) -> String {
        let opencode_pkg = self
            .config
            .opencode_repo
            .join("packages")
            .join("opencode")
            .to_string_lossy()
            .to_string();
        let cfg_dir = dir.join(".opencode").to_string_lossy().to_string();

        format!(
            "OPENCODE_PERMISSION='{{\"all\":\"allow\"}}' OPENCODE_CONFIG_DIR={} bun --cwd {} --conditions=browser src/index.ts {}",
            Self::shell_quote(&cfg_dir),
            Self::shell_quote(&opencode_pkg),
            Self::shell_quote(&dir.to_string_lossy()),
        )
    }

    pub async fn bootstrap_tmux_tabs(&self) -> Result<Vec<CommandResult>, AppError> {
        let tmux = self.tmux().to_string();
        let session = "uwu-main";
        let dirs = self.list_workspace_dirs().await?;
        let mut commands = Vec::new();

        commands.push(
            self.run_cmd(&[&tmux, "kill-session", "-t", session])
                .await
                .unwrap_or(CommandResult {
                    command: format!("{} kill-session -t {}", tmux, session),
                    executed: self.config.execute_commands,
                    success: Some(false),
                    stdout: None,
                    stderr: None,
                }),
        );

        let first_dir = dirs[0].to_string_lossy().to_string();
        self.setup_workspace_opencode_files(&dirs[0]).await?;
        let first_opencode_cmd = self.opencode_launch_command(&dirs[0]);
        commands.push(
            self.run_cmd(&[
                &tmux,
                "new-session",
                "-d",
                "-s",
                session,
                "-n",
                "workspace-1",
                "-c",
                &first_dir,
            ])
            .await?,
        );

        commands.push(
            self.run_cmd(&[&tmux, "set-option", "-t", session, "window-size", "latest"])
                .await?,
        );

        commands.push(
            self.run_cmd(&[
                &tmux,
                "set-option",
                "-t",
                session,
                "aggressive-resize",
                "on",
            ])
            .await?,
        );

        commands.push(
            self.run_cmd(&[
                &tmux,
                "set-option",
                "-t",
                &format!("{}:0.0", session),
                "-p",
                "protected-pane",
                "on",
            ])
            .await?,
        );

        commands.push(
            self.run_cmd(&[
                &tmux,
                "send-keys",
                "-t",
                &format!("{}:0.0", session),
                "gh auth status >/dev/null 2>&1 || echo \"⚠️  GitHub CLI not authenticated. Run: gh auth login\"",
                "Enter",
            ])
            .await?,
        );

        commands.push(
            self.run_cmd(&[
                &tmux,
                "send-keys",
                "-t",
                &format!("{}:0.0", session),
                &first_opencode_cmd,
                "Enter",
            ])
            .await?,
        );

        for (idx, dir) in dirs.iter().enumerate().skip(1) {
            let window_name = format!("workspace-{}", idx + 1);
            let target_pane = format!("{}:{}.0", session, idx);
            let dir_str = dir.to_string_lossy().to_string();
            self.setup_workspace_opencode_files(dir).await?;
            let opencode_cmd = self.opencode_launch_command(dir);

            commands.push(
                self.run_cmd(&[
                    &tmux,
                    "new-window",
                    "-t",
                    session,
                    "-n",
                    &window_name,
                    "-c",
                    &dir_str,
                ])
                .await?,
            );

            commands.push(
                self.run_cmd(&[
                    &tmux,
                    "set-option",
                    "-t",
                    &target_pane,
                    "-p",
                    "protected-pane",
                    "on",
                ])
                .await?,
            );

            commands.push(
                self.run_cmd(&[
                    &tmux,
                    "send-keys",
                    "-t",
                    &target_pane,
                    "gh auth status >/dev/null 2>&1 || echo \"⚠️  GitHub CLI not authenticated. Run: gh auth login\"",
                    "Enter",
                ])
                .await?,
            );

            commands.push(
                self.run_cmd(&[
                    &tmux,
                    "send-keys",
                    "-t",
                    &target_pane,
                    &opencode_cmd,
                    "Enter",
                ])
                .await?,
            );
        }

        Ok(commands)
    }

    pub async fn start_ttyd_main(&self, ttyd_port: u16) -> Result<StartResult, AppError> {
        let tmux = self.tmux().to_string();
        let session = "uwu-main";
        let ttyd_port_str = ttyd_port.to_string();
        let credential = format!("{}:{}", self.config.ttyd_user, self.config.ttyd_pass);
        let font_family = "JetBrains, SarasaMono, JetBrainsMono Nerd Font, monospace";
        tokio::fs::create_dir_all(&self.config.workspace_root).await?;
        let ttyd_cwd = self.ttyd_spawn_cwd(None);
        let ttyd_cmd_str = format!(
            "ttyd --port {} -W -t fontSize=13 -t lineHeight=1 -t 'fontFamily={}' -t titleFixed=uwu\\ workspace --credential {} {} attach -t {}",
            ttyd_port, font_family, credential, tmux, session
        );

        let browser_url = if self.config.execute_commands {
            let child = tokio::process::Command::new("ttyd")
                .args([
                    "--port",
                    &ttyd_port_str,
                    "-W",
                    "-t",
                    "fontSize=13",
                    "-t",
                    "lineHeight=1",
                    "-t",
                    &format!("fontFamily={}", font_family),
                    "-t",
                    "titleFixed=uwu workspace",
                    "--credential",
                    &credential,
                    &tmux,
                    "attach",
                    "-t",
                    session,
                ])
                .current_dir(&ttyd_cwd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| AppError::CommandFailed(format!("failed to spawn ttyd: {}", e)))?;

            self.supervisor.track("ttyd:main".to_string(), child).await;
            Some(format!("http://127.0.0.1:{}", ttyd_port))
        } else {
            Some(format!("http://127.0.0.1:{} (dry-run)", ttyd_port))
        };

        Ok(StartResult {
            commands: vec![CommandResult {
                command: ttyd_cmd_str,
                executed: self.config.execute_commands,
                success: if self.config.execute_commands {
                    Some(true)
                } else {
                    None
                },
                stdout: None,
                stderr: None,
            }],
            browser_url,
        })
    }

    pub async fn start_workspace(
        &self,
        workspace_name: &str,
        workspace_path: &Path,
        _opencode_port: u16,
        ttyd_port: u16,
    ) -> Result<StartResult, AppError> {
        self.create_directory(workspace_path).await?;

        let tmux = self.tmux().to_string();
        let default_session = Self::tmux_session_name(workspace_name);
        let session = default_session.clone();
        let path_str = workspace_path.to_string_lossy().to_string();
        let mut commands = Vec::new();

        let setup_script = Self::setup_tmux_script(workspace_path);
        if setup_script.exists() {
            let session_env = format!("MYAPP_TMUX_SESSION_NAME={}", default_session);
            let setup_script_str = setup_script.to_string_lossy().to_string();
            commands.push(
                self.run_cmd(&["env", &session_env, "bash", &setup_script_str])
                    .await?,
            );
        }

        if session == default_session {
            let has_session = self.run_cmd(&[&tmux, "has-session", "-t", &session]).await;
            let already_exists = has_session
                .as_ref()
                .ok()
                .and_then(|r| r.success)
                .unwrap_or(false);

            if already_exists {
                info!(session = %session, "tmux session already exists, reusing");
                commands.push(CommandResult {
                    command: format!("{} has-session -t {}", tmux, session),
                    executed: true,
                    success: Some(true),
                    stdout: Some("session already exists".into()),
                    stderr: None,
                });
            } else {
                commands.push(
                    self.run_cmd(&[&tmux, "new-session", "-d", "-s", &session, "-c", &path_str])
                        .await?,
                );

                commands.push(
                    self.run_cmd(&[
                        &tmux,
                        "set-option",
                        "-t",
                        &session,
                        "-p",
                        "protected-pane",
                        "on",
                    ])
                    .await?,
                );
            }
        }

        let ttyd_port_str = ttyd_port.to_string();
        let credential = format!("{}:{}", self.config.ttyd_user, self.config.ttyd_pass);
        let font_family = "JetBrains, SarasaMono, JetBrainsMono Nerd Font, monospace";
        let ttyd_cwd = self.ttyd_spawn_cwd(Some(workspace_path));
        let ttyd_cmd_str = format!(
            "ttyd --port {} -W -t fontSize=13 -t lineHeight=1 -t 'fontFamily={}' --credential {} {} attach -t {}",
            ttyd_port, font_family, credential, tmux, session
        );

        let browser_url = if self.config.execute_commands {
            let child = tokio::process::Command::new("ttyd")
                .args([
                    "--port",
                    &ttyd_port_str,
                    "-W",
                    "-t",
                    "fontSize=13",
                    "-t",
                    "lineHeight=1",
                    "-t",
                    &format!("fontFamily={}", font_family),
                    "--credential",
                    &credential,
                    &tmux,
                    "attach",
                    "-t",
                    &session,
                ])
                .current_dir(&ttyd_cwd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| AppError::CommandFailed(format!("failed to spawn ttyd: {}", e)))?;

            let key = Self::ttyd_key(workspace_name);
            self.supervisor.track(key, child).await;

            commands.push(CommandResult {
                command: ttyd_cmd_str,
                executed: true,
                success: Some(true),
                stdout: None,
                stderr: None,
            });

            Some(format!("http://127.0.0.1:{}", ttyd_port))
        } else {
            commands.push(CommandResult {
                command: ttyd_cmd_str,
                executed: false,
                success: None,
                stdout: None,
                stderr: None,
            });

            Some(format!("http://127.0.0.1:{} (dry-run)", ttyd_port))
        };

        Ok(StartResult {
            commands,
            browser_url,
        })
    }

    pub async fn stop_workspace(
        &self,
        workspace_name: &str,
        workspace_path: &Path,
    ) -> Result<Vec<CommandResult>, AppError> {
        let tmux = self.tmux().to_string();
        let mut results = Vec::new();

        let key = Self::ttyd_key(workspace_name);
        if self.supervisor.kill(&key).await {
            info!(workspace = %workspace_name, "killed ttyd process");
        }

        let mut sessions: BTreeSet<String> = BTreeSet::new();
        sessions.insert(Self::tmux_session_name(workspace_name));
        sessions.insert(Self::legacy_tmux_session_name(workspace_name));

        let rooted_sessions = self.tmux_sessions_for_workspace(workspace_path).await?;
        for rooted in rooted_sessions {
            if rooted == Self::tmux_session_name(workspace_name)
                || rooted == Self::legacy_tmux_session_name(workspace_name)
            {
                sessions.insert(rooted);
            }
        }

        for session in sessions {
            results.push(
                self.run_cmd(&[&tmux, "kill-session", "-t", &session])
                    .await?,
            );
        }

        Ok(results)
    }

    pub async fn create_tmux_test_log(
        &self,
        workspace_name: &str,
        workspace_path: &Path,
    ) -> Result<TmuxTestLogResult, AppError> {
        let session = Self::tmux_session_name(workspace_name);

        let tmux = self.tmux().to_string();
        let has_session = tokio::process::Command::new(&tmux)
            .args(["has-session", "-t", &session])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| AppError::CommandFailed(format!("failed to check tmux session: {}", e)))?;

        if !has_session.success() {
            return Err(AppError::NotFound(format!(
                "tmux session '{}' not found; start project first",
                session
            )));
        }

        let sessions: Vec<String> = vec![session.clone()];

        let logs_dir = workspace_path.join("logs").join("tmux");
        tokio::fs::create_dir_all(&logs_dir).await?;
        let file_name = format!("tmux-test-{}.log", Utc::now().format("%Y%m%d-%H%M%S"));
        let output_path = logs_dir.join(file_name);

        let mut content = String::new();
        content.push_str("# tmux test log\n");
        content.push_str(&format!("workspace={}\n", workspace_name));
        content.push_str(&format!("created_at={}\n\n", Utc::now().to_rfc3339()));

        for session in &sessions {
            let windows = tokio::process::Command::new(&tmux)
                .args([
                    "list-windows",
                    "-t",
                    session,
                    "-F",
                    "#{window_index}\t#{window_name}",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await
                .map_err(|e| AppError::CommandFailed(format!("failed to list windows: {}", e)))?;

            if !windows.status.success() {
                continue;
            }

            for window in String::from_utf8_lossy(&windows.stdout).lines() {
                let mut parts = window.splitn(2, '\t');
                let Some(index) = parts.next() else {
                    continue;
                };
                let window_name = parts.next().unwrap_or("window");
                let target = format!("{}:{}.0", session, index);

                let pane = tokio::process::Command::new(&tmux)
                    .args(["capture-pane", "-pt", &target, "-S", "-2000"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()
                    .await
                    .map_err(|e| {
                        AppError::CommandFailed(format!("failed to capture pane: {}", e))
                    })?;

                if !pane.status.success() {
                    continue;
                }

                content.push_str(&format!("----- {} ({}) -----\n", target, window_name));
                content.push_str(&String::from_utf8_lossy(&pane.stdout));
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
            }
        }

        tokio::fs::write(&output_path, content).await?;

        Ok(TmuxTestLogResult {
            workspace: workspace_name.to_string(),
            sessions,
            log_file: output_path.to_string_lossy().to_string(),
        })
    }
}
