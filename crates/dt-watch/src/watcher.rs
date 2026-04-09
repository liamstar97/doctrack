use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use dt_index::Index;

/// Events emitted by the file watcher.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A vault note was created or modified.
    VaultNoteChanged(PathBuf),
    /// A vault note was removed.
    VaultNoteRemoved(PathBuf),
    /// A code file was created or modified.
    CodeFileChanged(PathBuf),
    /// A code file was removed.
    CodeFileRemoved(PathBuf),
}

/// Watches both the vault and project directories for changes.
pub struct FileWatcher {
    index: Arc<Index>,
    vault_root: PathBuf,
    project_root: PathBuf,
}

impl FileWatcher {
    pub fn new(index: Arc<Index>, vault_root: PathBuf, project_root: PathBuf) -> Self {
        Self {
            index,
            vault_root,
            project_root,
        }
    }

    /// Start watching for file changes. Returns a channel of watch events.
    pub async fn start(&self) -> Result<mpsc::UnboundedReceiver<WatchEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let vault_root = self.vault_root.clone();
        let project_root = self.project_root.clone();
        let index = self.index.clone();

        // Spawn a blocking thread for the file watcher
        tokio::task::spawn_blocking(move || {
            let (notify_tx, notify_rx) = std::sync::mpsc::channel();

            let mut debouncer = new_debouncer(Duration::from_millis(500), notify_tx)
                .expect("failed to create debouncer");

            // Watch the vault directory
            debouncer
                .watcher()
                .watch(&vault_root, notify::RecursiveMode::Recursive)
                .expect("failed to watch vault directory");

            // Watch the project root (non-recursive — we'll filter by known files)
            debouncer
                .watcher()
                .watch(&project_root, notify::RecursiveMode::Recursive)
                .expect("failed to watch project directory");

            info!(
                "watching vault={} project={}",
                vault_root.display(),
                project_root.display()
            );

            for result in notify_rx {
                match result {
                    Ok(events) => {
                        for event in events {
                            if event.kind != DebouncedEventKind::Any {
                                continue;
                            }

                            let path = &event.path;

                            if path.starts_with(&vault_root) {
                                if is_markdown(path) {
                                    if path.exists() {
                                        debug!("vault note changed: {}", path.display());
                                        let _ = index.reindex_note(path);
                                        let _ = tx.send(WatchEvent::VaultNoteChanged(path.clone()));
                                    } else {
                                        debug!("vault note removed: {}", path.display());
                                        let _ = tx.send(WatchEvent::VaultNoteRemoved(path.clone()));
                                    }
                                }
                            } else if is_code_file(path) {
                                if path.exists() {
                                    debug!("code file changed: {}", path.display());
                                    let _ = index.reindex_code_file(path);
                                    let _ = tx.send(WatchEvent::CodeFileChanged(path.clone()));
                                } else {
                                    debug!("code file removed: {}", path.display());
                                    let _ = tx.send(WatchEvent::CodeFileRemoved(path.clone()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("file watcher error: {:?}", e);
                    }
                }
            }
        });

        Ok(rx)
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
}

fn is_code_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "cpp" | "cc" | "h" | "hpp"
    )
}
