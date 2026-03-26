use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug)]
pub struct TrackedProcess {
    pub pid: u32,
    pub label: String,
}

pub struct ProcessSupervisor {
    children: Arc<Mutex<HashMap<String, u32>>>,
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

    pub async fn track_pid(&self, key: String, pid: u32) {
        info!(key = %key, pid = pid, "tracking process by PID");
        self.children.lock().await.insert(key, pid);
    }

    pub async fn get_pid(&self, key: &str) -> Option<u32> {
        self.children.lock().await.get(key).copied()
    }

    pub async fn kill(&self, key: &str) -> bool {
        let pid = self.children.lock().await.remove(key);
        match pid {
            Some(pid) => {
                info!(key = %key, pid = pid, "killing process by PID");
                let out = Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output();
                match out.await {
                    Ok(o) if o.status.success() => true,
                    _ => {
                        let _ = Command::new("kill")
                            .arg("-9")
                            .arg(pid.to_string())
                            .output()
                            .await;
                        true
                    }
                }
            }
            None => false,
        }
    }

    pub async fn kill_all(&self) {
        let pids: Vec<u32> = self
            .children
            .lock()
            .await
            .drain()
            .map(|(_, pid)| pid)
            .collect();
        for pid in pids {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .output()
                .await;
        }
    }

    pub async fn list(&self) -> Vec<TrackedProcess> {
        self.children
            .lock()
            .await
            .iter()
            .map(|(key, &pid)| TrackedProcess {
                pid,
                label: key.clone(),
            })
            .collect()
    }

    pub async fn is_tracked(&self, key: &str) -> bool {
        self.children.lock().await.contains_key(key)
    }
}
