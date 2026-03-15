use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub target_pane: String,
    pub messages: Vec<Message>,
    pub last_capture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub index: String,
    pub name: String,
    pub path: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureResponse {
    pub target: String,
    pub capture: String,
}

#[derive(Debug)]
struct CommanderInner {
    tmux_bin: String,
    sessions: HashMap<String, Session>,
    active_session: String,
    next_message_id: u64,
}

#[derive(Clone, Debug)]
pub struct CommanderState {
    inner: Arc<RwLock<CommanderInner>>,
}

pub fn strip_ansi(input: &str) -> String {
    let ansi_re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07|\x1b\(B")
        .expect("valid ANSI stripping regex");
    let without_ansi = ansi_re.replace_all(input, "");
    let without_cr = without_ansi.replace('\r', "");

    let mut lines = Vec::new();
    let mut saw_blank = false;
    for line in without_cr.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !saw_blank {
                lines.push(String::new());
                saw_blank = true;
            }
        } else {
            lines.push(trimmed.to_string());
            saw_blank = false;
        }
    }

    lines.join("\n").trim().to_string()
}

pub async fn capture_pane(tmux_bin: &str, target: &str) -> Result<String> {
    let output = Command::new(tmux_bin)
        .args(["capture-pane", "-t", target, "-p", "-S", "-200"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!(
            "tmux capture-pane failed for target '{}' with status {}: {}",
            target,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(strip_ansi(&String::from_utf8_lossy(&output.stdout)))
}

pub fn diff_capture(old: &str, new: &str) -> Option<String> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    if old_lines == new_lines {
        return None;
    }

    if old_lines.is_empty() {
        return trim_diff_lines(new_lines);
    }

    if new_lines.len() >= old_lines.len() && new_lines.starts_with(&old_lines) {
        return trim_diff_lines(new_lines[old_lines.len()..].to_vec());
    }

    let overlap_limit = old_lines.len().min(new_lines.len());
    for overlap in (1..=overlap_limit).rev() {
        if old_lines[old_lines.len() - overlap..] == new_lines[..overlap] {
            return trim_diff_lines(new_lines[overlap..].to_vec());
        }
    }

    trim_diff_lines(new_lines)
}

pub async fn send_message(tmux_bin: &str, target: &str, message: &str) -> Result<()> {
    let escaped_message = escape_message(message);
    if escaped_message.is_empty() {
        return Err(anyhow!("message must not be empty"));
    }

    let output = Command::new(tmux_bin)
        .args(["send-keys", "-t", target, &escaped_message, "Enter"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!(
            "tmux send-keys failed for target '{}' with status {}: {}",
            target,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(())
}

pub async fn list_sessions(tmux_bin: &str) -> Result<Vec<SessionInfo>> {
    let output = Command::new(tmux_bin)
        .args([
            "list-windows",
            "-t",
            "uwu-main",
            "-F",
            "#{window_index}:#{window_name}:#{pane_current_path}",
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!(
            "tmux list-windows failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let mut sessions = Vec::new();
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let mut parts = line.splitn(3, ':');
        let index = parts.next().unwrap_or_default().trim().to_string();
        let name = parts.next().unwrap_or_default().trim().to_string();
        let path = parts.next().unwrap_or_default().trim().to_string();

        if !index.is_empty() {
            sessions.push(SessionInfo {
                target: format!("uwu-main:{}", index),
                index,
                name,
                path,
            });
        }
    }

    Ok(sessions)
}

impl CommanderState {
    pub fn new(tmux_bin: String) -> Self {
        let active_session = "uwu-main:0".to_string();
        let mut sessions = HashMap::new();
        sessions.insert(
            active_session.clone(),
            Session {
                name: "main".to_string(),
                target_pane: active_session.clone(),
                messages: Vec::new(),
                last_capture: String::new(),
            },
        );

        Self {
            inner: Arc::new(RwLock::new(CommanderInner {
                tmux_bin,
                sessions,
                active_session,
                next_message_id: 1,
            })),
        }
    }

    pub async fn switch_session(&self, target: String) -> Result<String> {
        let target = target.trim().to_string();
        if target.is_empty() {
            return Err(anyhow!("target is required"));
        }

        let mut inner = self.inner.write().await;
        inner.active_session = target.clone();
        inner
            .sessions
            .entry(target.clone())
            .or_insert_with(|| Session {
                name: target.clone(),
                target_pane: target.clone(),
                messages: Vec::new(),
                last_capture: String::new(),
            });

        Ok(target)
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let tmux_bin = self.inner.read().await.tmux_bin.clone();
        list_sessions(&tmux_bin).await
    }

    pub async fn send(&self, message: String) -> Result<Message> {
        let content = message.trim().to_string();
        if content.is_empty() {
            return Err(anyhow!("message must not be empty"));
        }

        let (tmux_bin, target, user_message) = {
            let mut inner = self.inner.write().await;
            let target = inner.active_session.clone();
            let id = inner.next_message_id;
            inner.next_message_id = inner.next_message_id.saturating_add(1);

            let user_message = Message {
                id,
                role: "user".to_string(),
                content: content.clone(),
                timestamp: Utc::now(),
            };

            inner
                .sessions
                .entry(target.clone())
                .or_insert_with(|| Session {
                    name: target.clone(),
                    target_pane: target.clone(),
                    messages: Vec::new(),
                    last_capture: String::new(),
                })
                .messages
                .push(user_message.clone());

            (inner.tmux_bin.clone(), target, user_message)
        };

        send_message(&tmux_bin, &target, &content).await?;
        let capture = capture_pane(&tmux_bin, &target).await?;

        {
            let mut inner = self.inner.write().await;
            if let Some(session) = inner.sessions.get_mut(&target) {
                session.last_capture = capture;
            }
        }

        Ok(user_message)
    }

    pub async fn poll_updates(&self) -> Result<Vec<Message>> {
        let (tmux_bin, target, old_capture) = {
            let inner = self.inner.read().await;
            let target = inner.active_session.clone();
            let old_capture = inner
                .sessions
                .get(&target)
                .map(|session| session.last_capture.clone())
                .unwrap_or_default();
            (inner.tmux_bin.clone(), target, old_capture)
        };

        let new_capture = capture_pane(&tmux_bin, &target).await?;
        let diff = diff_capture(&old_capture, &new_capture);

        let mut created = Vec::new();
        let mut inner = self.inner.write().await;
        let next_id = inner.next_message_id;

        let session = inner
            .sessions
            .entry(target.clone())
            .or_insert_with(|| Session {
                name: target.clone(),
                target_pane: target.clone(),
                messages: Vec::new(),
                last_capture: String::new(),
            });

        session.last_capture = new_capture;

        if let Some(content) = diff {
            let message = Message {
                id: next_id,
                role: "assistant".to_string(),
                content,
                timestamp: Utc::now(),
            };
            session.messages.push(message.clone());
            inner.next_message_id = inner.next_message_id.saturating_add(1);
            created.push(message);
        }

        Ok(created)
    }

    pub async fn messages_since(&self, since: u64) -> Vec<Message> {
        let inner = self.inner.read().await;
        let target = inner.active_session.clone();
        inner
            .sessions
            .get(&target)
            .map(|session| {
                session
                    .messages
                    .iter()
                    .filter(|message| message.id > since)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn capture(&self) -> Result<CaptureResponse> {
        let (tmux_bin, target) = {
            let inner = self.inner.read().await;
            (inner.tmux_bin.clone(), inner.active_session.clone())
        };

        let capture = capture_pane(&tmux_bin, &target).await?;
        Ok(CaptureResponse { target, capture })
    }
}

fn trim_diff_lines(lines: Vec<&str>) -> Option<String> {
    let mut start = 0;
    let mut end = lines.len();

    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    if start >= end {
        return None;
    }

    let content = lines[start..end].join("\n").trim().to_string();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}

fn escape_message(input: &str) -> String {
    input
        .trim()
        .replace('\r', " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
