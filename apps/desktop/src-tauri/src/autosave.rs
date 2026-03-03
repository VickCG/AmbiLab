use crate::workspace_config::WorkspaceConfig;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const DEBOUNCE_DURATION_MS: u64 = 500;

pub struct AutoSaveManager {
    pending_saves: Arc<RwLock<Vec<(String, WorkspaceConfig)>>>,
}

impl AutoSaveManager {
    pub fn new() -> Self {
        Self {
            pending_saves: Arc::new(RwLock::new(Vec::with_capacity(10))),
        }
    }

    pub fn queue_save(&self, workspace_path: String, config: WorkspaceConfig) {
        let mut pending = self.pending_saves.write();

        pending.retain(|(path, _)| path != &workspace_path);

        pending.push((workspace_path, config));
    }

    pub fn start_background_task(self: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            loop {
                sleep(Duration::from_millis(DEBOUNCE_DURATION_MS)).await;

                let saves = {
                    let mut pending = self.pending_saves.write();
                    std::mem::take(&mut *pending)
                };

                if !saves.is_empty() {
                    let futures: Vec<_> = saves
                        .into_iter()
                        .map(|(path, config)| async move {
                            if let Err(e) = config.save(&path).await {
                                eprintln!("Failed to save workspace config: {:?}", e);
                            }
                        })
                        .collect();

                    futures_util::future::join_all(futures).await;
                }
            }
        });
    }
}

impl Default for AutoSaveManager {
    fn default() -> Self {
        Self::new()
    }
}
