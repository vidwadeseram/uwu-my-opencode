use crate::config::AppConfig;
use crate::error::AppError;
use crate::supervisor::ProcessSupervisor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tracing::{info, warn};

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

impl WorkspaceManager {
    pub fn new(config: AppConfig, supervisor: ProcessSupervisor) -> Self {
        Self { config, supervisor }
    }

    fn tmux(&self) -> &str {
        &self.config.tmux_bin
    }

    pub async fn create_directory(&self, workspace_path: &Path) -> Result<(), AppError> {
        tokio::fs::create_dir_all(workspace_path).await?;
        info!(path = %workspace_path.display(), "created workspace directory");
        Ok(())
    }

    pub fn tmux_session_name(workspace_name: &str) -> String {
        format!("uwu-{}", workspace_name)
    }

    fn ttyd_key(workspace_name: &str) -> String {
        format!("ttyd:{}", workspace_name)
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
        let tmux_missing = !tmux_target.exists();
        let nvim_missing = !nvim_target.exists();

        if !tmux_missing && !nvim_missing {
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

        Ok(commands)
    }

    fn shell_quote(value: &str) -> String {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }

    async fn setup_workspace_opencode_files(&self, dir: &Path) -> Result<(), AppError> {
        let opencode_dir = dir.join(".opencode");
        let plugins_dir = opencode_dir.join("plugins");
        let commands_dir = opencode_dir.join("command");

        tokio::fs::create_dir_all(&plugins_dir).await?;
        tokio::fs::create_dir_all(&commands_dir).await?;

        let oh_my_src = self
            .config
            .oh_my_opencode_repo
            .join("src")
            .join("index.ts")
            .to_string_lossy()
            .to_string();

        let plugin_file = plugins_dir.join("oh-my-opencode.ts");
        let plugin_content = format!(
            "import OhMyOpenCodePlugin from \"{}\";\nexport default OhMyOpenCodePlugin;\n",
            oh_my_src.replace('\\', "\\\\")
        );
        tokio::fs::write(plugin_file, plugin_content).await?;

        let host_project_file = commands_dir.join("host-project.md");
        let host_project_content = "---\ndescription: host project locally on port 3000\nmodel: opencode/kimi-k2.5\nsubtask: false\n---\n\nHost this project for preview.\n\n1) Detect project stack and install dependencies if needed\n2) Start the dev server on port 3000\n3) If 3000 is unavailable, use 3001 and report it\n4) Verify with curl that HTTP returns 200\n5) Print final local URL and what command is running\n\nPrefer non-blocking run methods (tmux pane / background process) so the terminal stays usable.\n";
        tokio::fs::write(host_project_file, host_project_content).await?;

        Ok(())
    }

    fn opencode_launch_command(&self, dir: &Path) -> String {
        let opencode_entry = self
            .config
            .opencode_repo
            .join("packages")
            .join("opencode")
            .join("src")
            .join("index.ts")
            .to_string_lossy()
            .to_string();
        let cfg_dir = dir.join(".opencode").to_string_lossy().to_string();

        format!(
            "OPENCODE_PERMISSION='{{\"all\":\"allow\"}}' OPENCODE_CONFIG_DIR={} bun run --conditions=browser {}",
            Self::shell_quote(&cfg_dir),
            Self::shell_quote(&opencode_entry),
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
        let credential = "admin:admin";
        let ttyd_cmd_str = format!(
            "ttyd --port {} -W -t fontSize=13 -t lineHeight=1 --credential {} {} attach -t {}",
            ttyd_port, credential, tmux, session
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
                    "--credential",
                    credential,
                    &tmux,
                    "attach",
                    "-t",
                    session,
                ])
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
        let session = Self::tmux_session_name(workspace_name);
        let path_str = workspace_path.to_string_lossy().to_string();
        let mut commands = Vec::new();

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

        let ttyd_port_str = ttyd_port.to_string();
        let credential = "admin:admin";
        let ttyd_cmd_str = format!(
            "ttyd --port {} --credential {} {} attach -t {}",
            ttyd_port, credential, tmux, session
        );

        let browser_url = if self.config.execute_commands {
            let child = tokio::process::Command::new("ttyd")
                .args([
                    "--port",
                    &ttyd_port_str,
                    "--credential",
                    credential,
                    &tmux,
                    "attach",
                    "-t",
                    &session,
                ])
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
    ) -> Result<Vec<CommandResult>, AppError> {
        let tmux = self.tmux().to_string();
        let session = Self::tmux_session_name(workspace_name);
        let mut results = Vec::new();

        let key = Self::ttyd_key(workspace_name);
        if self.supervisor.kill(&key).await {
            info!(workspace = %workspace_name, "killed ttyd process");
        }

        results.push(
            self.run_cmd(&[&tmux, "kill-session", "-t", &session])
                .await?,
        );

        Ok(results)
    }
}
