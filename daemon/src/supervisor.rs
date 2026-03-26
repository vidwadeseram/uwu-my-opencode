use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug)]
pub struct TrackedProcess {
    pub pid: u32,
    pub label: String,
}

pub struct ProcessSupervisor {
    children: Arc<Mutex<HashMap<String, Child>>>,
}

impl Clone for ProcessSupervisor {
    fn clone(&self) -> Self {
        Self {
            children: Arc::clone(&self.children),
        }
    }
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn track(&self, key: String, child: Child) -> Option<u32> {
        let pid = child.id();
        info!(key = %key, pid = ?pid, "tracking child process");
        self.children.lock().await.insert(key, child);
        pid
    }

    pub async fn kill(&self, key: &str) -> bool {
        let mut children = self.children.lock().await;
        if let Some(mut child) = children.remove(key) {
            let pid = child.id();
            info!(key = %key, pid = ?pid, "reaping child process");
            // Use wait() instead of kill(): wait() properly reaps zombies
            // (returns immediately with exit status if already zombie) and
            // also waits for a running process to exit. kill() fails with
            // ESRCH on zombies, leaving them zombie forever.
            match child.wait().await {
                Ok(status) => {
                    info!(key = %key, pid = ?pid, status = %status, "child reaped successfully");
                    true
                }
                Err(e) => {
                    warn!(key = %key, error = %e, "failed to reap child");
                    false
                }
            }
        } else {
            false
        }
    }

    pub async fn kill_all(&self) {
        let mut children = self.children.lock().await;
        for (key, mut child) in children.drain() {
            let pid = child.id();
            info!(key = %key, pid = ?pid, "reaping child process (shutdown)");
            let _ = child.wait().await;
        }
    }

    pub async fn list(&self) -> Vec<TrackedProcess> {
        let children = self.children.lock().await;
        children
            .iter()
            .map(|(key, child)| TrackedProcess {
                pid: child.id().unwrap_or(0),
                label: key.clone(),
            })
            .collect()
    }

    pub async fn is_tracked(&self, key: &str) -> bool {
        self.children.lock().await.contains_key(key)
    }
}
