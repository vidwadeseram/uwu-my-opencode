use crate::config::AppConfig;
use crate::error::AppError;
use crate::supervisor::ProcessSupervisor;
use regex::Regex;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn};

pub struct TunnelManager {
    config: AppConfig,
    supervisor: ProcessSupervisor,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TunnelCommandResult {
    pub command: String,
    pub executed: bool,
    pub tunnel_url: Option<String>,
    pub pid: Option<u32>,
}

impl TunnelManager {
    pub fn new(config: AppConfig, supervisor: ProcessSupervisor) -> Self {
        Self { config, supervisor }
    }

    fn tunnel_key(workspace_id: &str, port: u16) -> String {
        format!("tunnel:{}:{}", workspace_id, port)
    }

    pub async fn start_tunnel(
        &self,
        workspace_id: &str,
        local_port: u16,
    ) -> Result<TunnelCommandResult, AppError> {
        let cmd_str = format!(
            "cloudflared tunnel --url http://127.0.0.1:{} --no-autoupdate",
            local_port
        );

        if !self.config.execute_commands {
            info!(command = %cmd_str, "dry-run mode, skipping tunnel execution");
            return Ok(TunnelCommandResult {
                command: cmd_str,
                executed: false,
                tunnel_url: Some(format!(
                    "https://<generated-subdomain>.trycloudflare.com (dry-run, port {})",
                    local_port
                )),
                pid: None,
            });
        }

        let has_cloudflared = tokio::process::Command::new("which")
            .arg("cloudflared")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        if !has_cloudflared {
            info!(
                port = local_port,
                "cloudflared not found, using localhost URL"
            );
            return Ok(TunnelCommandResult {
                command: cmd_str,
                executed: false,
                tunnel_url: Some(format!("http://127.0.0.1:{}", local_port)),
                pid: None,
            });
        }

        let pidfile_path = format!("/tmp/cloudflared-{}.pid", local_port);

        info!(command = %cmd_str, port = local_port, pidfile = %pidfile_path, "starting cloudflared tunnel");

        let mut child = Command::new("cloudflared")
            .args([
                "tunnel",
                "--url",
                &format!("http://127.0.0.1:{}", local_port),
                "--pidfile",
                &pidfile_path,
                "--no-autoupdate",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::CommandFailed(format!("failed to spawn cloudflared: {}", e)))?;

        let tunnel_url = parse_tunnel_url_from_child(&mut child).await;

        let daemon_pid = read_pidfile(&pidfile_path).await;

        let key = Self::tunnel_key(workspace_id, local_port);
        if let Some(pid) = daemon_pid {
            self.supervisor.track_pid(key, pid).await;
        }

        Ok(TunnelCommandResult {
            command: cmd_str,
            executed: true,
            tunnel_url,
            pid: daemon_pid,
        })
    }

    pub async fn stop_tunnel(&self, workspace_id: &str, local_port: u16) -> bool {
        let key = Self::tunnel_key(workspace_id, local_port);
        let killed = self.supervisor.kill(&key).await;
        let pidfile_path = format!("/tmp/cloudflared-{}.pid", local_port);
        let _ = fs::remove_file(&pidfile_path).await;
        if !killed {
            let out = Command::new("sh")
                .args([
                    "-c",
                    &format!(
                        "pkill -f 'cloudflared.*--url http://127.0.0.1:{}'",
                        local_port
                    ),
                ])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);
            return out;
        }
        killed
    }
}

async fn read_pidfile(path: &str) -> Option<u32> {
    let content = fs::read_to_string(path).await.ok()?;
    content.trim().parse::<u32>().ok()
}

async fn parse_tunnel_url_from_child(child: &mut tokio::process::Child) -> Option<String> {
    let stderr = child.stderr.take()?;
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    let url_re = Regex::new(r"https://[a-zA-Z0-9\-]+\.trycloudflare\.com").ok()?;

    let timeout = tokio::time::Duration::from_secs(15);
    let result = tokio::time::timeout(timeout, async {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(m) = url_re.find(&line) {
                return Some(m.as_str().to_string());
            }
        }
        None
    })
    .await;

    match result {
        Ok(Some(url)) => {
            info!(url = %url, "parsed cloudflared tunnel URL");
            Some(url)
        }
        Ok(None) => {
            warn!("cloudflared exited without producing a tunnel URL");
            None
        }
        Err(_) => {
            warn!("timed out waiting for cloudflared tunnel URL");
            None
        }
    }
}
