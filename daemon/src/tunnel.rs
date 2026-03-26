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
    pub backend: &'static str,
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
        if !self.config.execute_commands {
            let cmd_str = format!("cloudflared (dry-run, port {})", local_port);
            info!(command = %cmd_str, "dry-run mode, skipping tunnel execution");
            return Ok(TunnelCommandResult {
                command: cmd_str,
                executed: false,
                tunnel_url: Some(format!(
                    "https://<random>.trycloudflare.com (dry-run, port {})",
                    local_port
                )),
                pid: None,
                backend: "cloudflared",
            });
        }

        if let Some(result) = self
            .start_cloudflared_tunnel(workspace_id, local_port)
            .await
        {
            if result.tunnel_url.is_some() {
                return Ok(result);
            }
            warn!(port = local_port, "cloudflared failed, trying localtunnel");
        } else {
            warn!(
                port = local_port,
                "cloudflared failed to start, trying localtunnel"
            );
        }

        if let Some(result) = self
            .start_localtunnel_tunnel(workspace_id, local_port)
            .await
        {
            if result.tunnel_url.is_some() {
                return Ok(result);
            }
            warn!(port = local_port, "localtunnel failed, trying serveo");
        } else {
            warn!(
                port = local_port,
                "localtunnel failed to start, trying serveo"
            );
        }

        return Ok(self.start_serveo_tunnel(workspace_id, local_port).await);
    }

    async fn start_localtunnel_tunnel(
        &self,
        workspace_id: &str,
        local_port: u16,
    ) -> Option<TunnelCommandResult> {
        let cmd_str = format!("npx --yes localtunnel --port {}", local_port);

        info!(command = %cmd_str, port = local_port, "starting localtunnel");

        let mut child = Command::new("npx")
            .args(["--yes", "localtunnel", "--port", &local_port.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;

        let tunnel_url = parse_localtunnel_url_from_child(&mut child).await;
        let pid = child.id();

        let key = Self::tunnel_key(workspace_id, local_port);
        if let Some(p) = pid {
            self.supervisor.track_pid(key, p).await;
        }

        Some(TunnelCommandResult {
            command: cmd_str,
            executed: true,
            tunnel_url,
            pid,
            backend: "localtunnel",
        })
    }

    async fn start_cloudflared_tunnel(
        &self,
        workspace_id: &str,
        local_port: u16,
    ) -> Option<TunnelCommandResult> {
        let cmd_str = format!(
            "cloudflared tunnel --url http://127.0.0.1:{} --no-autoupdate",
            local_port
        );

        let has_cloudflared = Command::new("which")
            .arg("cloudflared")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        if !has_cloudflared {
            warn!(port = local_port, "cloudflared not found");
            return None;
        }

        let pidfile_path = format!("/tmp/cloudflared-{}.pid", local_port);

        info!(command = %cmd_str, port = local_port, "starting cloudflared tunnel");

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
            .ok()?;

        let tunnel_url = parse_cloudflared_url_from_child(&mut child).await;
        let daemon_pid = read_pidfile(&pidfile_path).await;

        let key = Self::tunnel_key(workspace_id, local_port);
        if let Some(pid) = daemon_pid {
            self.supervisor.track_pid(key, pid).await;
        }

        Some(TunnelCommandResult {
            command: cmd_str,
            executed: true,
            tunnel_url,
            pid: daemon_pid,
            backend: "cloudflared",
        })
    }

    async fn start_serveo_tunnel(
        &self,
        workspace_id: &str,
        local_port: u16,
    ) -> TunnelCommandResult {
        let cmd_str = format!(
            "ssh -o StrictHostKeyChecking=no -o ServerAliveInterval=60 -R 80:localhost:{} serveo.net",
            local_port
        );

        info!(command = %cmd_str, port = local_port, "starting serveo tunnel");

        let mut child = Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ServerAliveInterval=60",
                "-R",
                &format!("80:localhost:{}", local_port),
                "serveo.net",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let tunnel_url = parse_serveo_url_from_child(&mut child).await;
        let pid = child.id();

        let key = Self::tunnel_key(workspace_id, local_port);
        if let Some(p) = pid {
            self.supervisor.track_pid(key, p).await;
        }

        TunnelCommandResult {
            command: cmd_str,
            executed: true,
            tunnel_url,
            pid,
            backend: "serveo",
        }
    }

    pub async fn stop_tunnel(&self, workspace_id: &str, local_port: u16) -> bool {
        let key = Self::tunnel_key(workspace_id, local_port);
        let killed = self.supervisor.kill(&key).await;
        let _ = fs::remove_file(format!("/tmp/cloudflared-{}.pid", local_port)).await;
        if !killed {
            let out = Command::new("sh")
                .args([
                    "-c",
                    &format!(
                        "pkill -f 'cloudflared.*--url http://127.0.0.1:{}' || pkill -f 'localtunnel' || pkill -f 'serveo.net' || true",
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

async fn parse_localtunnel_url_from_child(child: &mut tokio::process::Child) -> Option<String> {
    let stdout = child.stdout.take()?;
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let url_re = Regex::new(r"https://[a-zA-Z0-9\-]+\.loca\.lt").ok()?;

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
            info!(url = %url, "parsed localtunnel URL");
            Some(url)
        }
        Ok(None) => {
            warn!("localtunnel exited without producing a URL");
            None
        }
        Err(_) => {
            warn!("timed out waiting for localtunnel URL");
            None
        }
    }
}

async fn parse_cloudflared_url_from_child(child: &mut tokio::process::Child) -> Option<String> {
    let stderr = child.stderr.take()?;
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    let url_re = Regex::new(r"https://[a-zA-Z0-9\-]+\.trycloudflare\.com").ok()?;
    let err_re = Regex::new(r"(?i)(429|rate.limit|too.many.requests)").ok()?;

    let timeout = tokio::time::Duration::from_secs(15);
    let result = tokio::time::timeout(timeout, async {
        while let Ok(Some(line)) = lines.next_line().await {
            if err_re.is_match(&line) {
                warn!(line = %line, "cloudflared rate limited");
                return Some(String::new());
            }
            if let Some(m) = url_re.find(&line) {
                return Some(m.as_str().to_string());
            }
        }
        None
    })
    .await;

    match result {
        Ok(Some(url)) if !url.is_empty() => {
            info!(url = %url, "parsed cloudflared tunnel URL");
            Some(url)
        }
        Ok(_) => {
            warn!("cloudflared exited without producing a tunnel URL");
            None
        }
        Err(_) => {
            warn!("timed out waiting for cloudflared tunnel URL");
            None
        }
    }
}

async fn parse_serveo_url_from_child(child: &mut tokio::process::Child) -> Option<String> {
    let stderr = child.stderr.take()?;
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    let url_re = Regex::new(r"https://[a-zA-Z0-9\-]+\.serveusercontent\.com").ok()?;

    let timeout = tokio::time::Duration::from_secs(10);
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
            info!(url = %url, "parsed serveo tunnel URL");
            Some(url)
        }
        Ok(None) => {
            warn!("serveo exited without producing a tunnel URL");
            None
        }
        Err(_) => {
            warn!("timed out waiting for serveo tunnel URL");
            None
        }
    }
}
