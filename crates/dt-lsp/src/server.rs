use std::path::PathBuf;
use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::info;

use dt_index::Index;
use dt_watch::FileWatcher;

use crate::capabilities;
use crate::definition;
use crate::diagnostics;
use crate::hover;

pub struct DoctrackServer {
    client: Client,
    index: Arc<Index>,
}

impl DoctrackServer {
    pub fn new(client: Client) -> Self {
        // These will be set properly on initialize based on workspace root
        let index = Arc::new(Index::new(PathBuf::new(), PathBuf::new()));
        Self { client, index }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for DoctrackServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("doctrack-lsp initializing");

        // Determine project root from workspace folders
        if let Some(folders) = &params.workspace_folders {
            if let Some(folder) = folders.first() {
                let root = PathBuf::from(folder.uri.path());
                let vault_root = root.join(".doctrack");

                if vault_root.exists() {
                    info!("found vault at {}", vault_root.display());

                    // Rebuild the index with correct paths
                    let index = Arc::new(Index::new(root.clone(), vault_root.clone()));

                    if let Err(e) = index.build() {
                        info!("index build error: {e}");
                    }

                    // Start file watcher
                    let watcher = FileWatcher::new(index.clone(), vault_root, root);
                    let _client = self.client.clone();
                    tokio::spawn(async move {
                        if let Ok(mut rx) = watcher.start().await {
                            while let Some(event) = rx.recv().await {
                                info!("watch event: {:?}", event);
                                // TODO: publish diagnostics on change
                            }
                        }
                    });
                } else {
                    info!("no .doctrack/ vault found in workspace");
                }
            }
        }

        Ok(InitializeResult {
            capabilities: capabilities::server_capabilities(),
            server_info: Some(ServerInfo {
                name: "doctrack-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("doctrack-lsp initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        info!("doctrack-lsp shutting down");
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(hover::handle_hover(&self.index, params))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        Ok(definition::handle_goto_definition(&self.index, params))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let path = PathBuf::from(uri.path());

        // Index the file on open if it's a code file we haven't seen
        if !self.index.code_symbols.contains_key(&path) {
            let _ = self.index.reindex_code_file(&path);
        }

        // Publish diagnostics for vault notes
        if path.starts_with(&self.index.vault_root) {
            let diags = diagnostics::check_note(&self.index, &path);
            self.client
                .publish_diagnostics(uri, diags, None)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let path = PathBuf::from(uri.path());

        // Re-index and republish diagnostics
        if path.starts_with(&self.index.vault_root) {
            let _ = self.index.reindex_note(&path);
            let diags = diagnostics::check_note(&self.index, &path);
            self.client
                .publish_diagnostics(uri, diags, None)
                .await;
        } else {
            let _ = self.index.reindex_code_file(&path);
        }
    }
}
