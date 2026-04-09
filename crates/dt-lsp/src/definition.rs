use std::path::PathBuf;

use tower_lsp::lsp_types::*;

use dt_index::Index;

/// Handle textDocument/definition — jump between code and docs.
pub fn handle_goto_definition(
    index: &Index,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;
    let path = PathBuf::from(uri.path());

    // From code: jump to the vault note documenting this symbol
    if let Some(symbols) = index.code_symbols.get(&path) {
        let line = position.line;

        for sym in symbols.value() {
            if line >= sym.start_line && line <= sym.end_line {
                let docs = index.docs_for_symbol(&path, &sym.name);
                if !docs.is_empty() {
                    let locations: Vec<Location> = docs
                        .iter()
                        .filter_map(|doc| {
                            let uri = Url::from_file_path(&doc.note_path).ok()?;
                            Some(Location {
                                uri,
                                range: Range::default(),
                            })
                        })
                        .collect();

                    if locations.len() == 1 {
                        return Some(GotoDefinitionResponse::Scalar(locations.into_iter().next().unwrap()));
                    } else if !locations.is_empty() {
                        return Some(GotoDefinitionResponse::Array(locations));
                    }
                }
            }
        }
    }

    // From vault note: jump to the code file/symbol referenced
    if path.starts_with(&index.vault_root) {
        if let Some(note) = index.vault_notes.get(&path) {
            // Find the file ref closest to the cursor line
            for file_ref in &note.file_refs {
                let abs_path = index.root.join(&file_ref.path);
                if abs_path.exists() {
                    let uri = Url::from_file_path(&abs_path).ok()?;
                    let line = file_ref.line.unwrap_or(0);
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri,
                        range: Range {
                            start: Position::new(line, 0),
                            end: Position::new(line, 0),
                        },
                    }));
                }
            }
        }
    }

    None
}
