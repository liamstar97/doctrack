use std::path::PathBuf;

use tracing::debug;

use crate::index::{DocLink, Index, MatchConfidence, SymbolId, SymbolRef};

/// Build all bidirectional links between vault notes and code symbols.
pub fn link_all(index: &Index) {
    for entry in index.vault_notes.iter() {
        let note = entry.value();
        let mut refs = Vec::new();

        // Tier 1: Exact path matches from file references (resolved via lookup)
        for file_ref in &note.file_refs {
            let resolved_paths = index.resolve_file_ref(file_ref);
            for abs_path in &resolved_paths {
                if let Some(symbols) = index.code_symbols.get(abs_path) {
                    for sym in symbols.value() {
                        let id = SymbolId {
                            file: abs_path.clone(),
                            name: sym.name.clone(),
                        };

                        // If a specific line is referenced, only link symbols at that line
                        let matches_line = file_ref.line.is_none_or(|line| {
                            line >= sym.start_line && line <= sym.end_line
                        });

                        if matches_line {
                            refs.push(SymbolRef {
                                symbol_id: id.clone(),
                                confidence: MatchConfidence::Exact,
                            });

                            index
                                .sym_to_docs
                                .entry(id)
                                .or_default()
                                .push(DocLink {
                                    note_path: note.path.clone(),
                                    note_title: note.title.clone(),
                                    note_type: note.note_type.clone(),
                                    context: note.summary.clone(),
                                });
                        }
                    }
                }
            }
        }

        // Tier 2: Symbol name matches from backtick references in body
        let backtick_names = extract_backtick_identifiers(&note.body);
        for name in &backtick_names {
            for code_entry in index.code_symbols.iter() {
                let file = code_entry.key();
                for sym in code_entry.value() {
                    if sym.name == *name {
                        let id = SymbolId {
                            file: file.clone(),
                            name: sym.name.clone(),
                        };

                        // Avoid duplicating if already linked by exact match
                        let already_linked = refs.iter().any(|r| r.symbol_id == id);
                        if !already_linked {
                            refs.push(SymbolRef {
                                symbol_id: id.clone(),
                                confidence: MatchConfidence::Strong,
                            });

                            index
                                .sym_to_docs
                                .entry(id)
                                .or_default()
                                .push(DocLink {
                                    note_path: note.path.clone(),
                                    note_title: note.title.clone(),
                                    note_type: note.note_type.clone(),
                                    context: format!("references `{name}`"),
                                });
                        }
                    }
                }
            }
        }

        // Tier 3: Fuzzy title/filename matching
        let fuzzy_matches = fuzzy_match_title(&note.title, index);
        for (file, sym_name, score) in fuzzy_matches {
            let id = SymbolId {
                file: file.clone(),
                name: sym_name.clone(),
            };

            let already_linked = refs.iter().any(|r| r.symbol_id == id);
            if !already_linked {
                debug!(
                    "fuzzy match: note '{}' -> {}::{} (score: {})",
                    note.title,
                    file.display(),
                    sym_name,
                    score
                );

                refs.push(SymbolRef {
                    symbol_id: id.clone(),
                    confidence: MatchConfidence::Fuzzy,
                });

                index
                    .sym_to_docs
                    .entry(id)
                    .or_default()
                    .push(DocLink {
                        note_path: note.path.clone(),
                        note_title: note.title.clone(),
                        note_type: note.note_type.clone(),
                        context: format!("fuzzy match (score: {score})"),
                    });
            }
        }

        if !refs.is_empty() {
            index.doc_to_syms.insert(note.path.clone(), refs);
        }
    }
}

/// Extract identifiers from backtick-enclosed text that look like symbol names.
fn extract_backtick_identifiers(body: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut rest = body;

    while let Some(start) = rest.find('`') {
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('`') {
            let content = &rest[..end];
            // Only include things that look like identifiers (PascalCase, snake_case, etc.)
            if looks_like_identifier(content) {
                identifiers.push(content.to_string());
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }

    identifiers
}

/// Heuristic: does this look like a code identifier rather than a file path or command?
fn looks_like_identifier(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.contains(' ') || s.contains('/') {
        return false;
    }
    // Must start with a letter or underscore
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Fuzzy match a note title against code file names and symbol names.
/// Returns (file_path, symbol_name, score) for matches above threshold.
fn fuzzy_match_title(title: &str, index: &Index) -> Vec<(PathBuf, String, u32)> {
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Config, Matcher};

    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::parse(title, CaseMatching::Ignore, Normalization::Smart);

    let mut results = Vec::new();

    for entry in index.code_symbols.iter() {
        let file = entry.key();

        // Match against filename
        if let Some(file_name) = file.file_stem().and_then(|s| s.to_str()) {
            let mut buf = Vec::new();
            if let Some(score) = pattern.score(nucleo_matcher::Utf32Str::new(file_name, &mut buf), &mut matcher) {
                if score > 50 {
                    // Link to all top-level symbols in the file
                    for sym in entry.value() {
                        results.push((file.clone(), sym.name.clone(), score));
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_identifier() {
        assert!(looks_like_identifier("SessionManager"));
        assert!(looks_like_identifier("parse_note"));
        assert!(looks_like_identifier("_private"));
        assert!(!looks_like_identifier("src/main.rs"));
        assert!(!looks_like_identifier("some text"));
        assert!(!looks_like_identifier(""));
    }

    #[test]
    fn test_extract_backtick_identifiers() {
        let body = "Uses `SessionManager` to handle `parse_note` from `src/vault.rs`";
        let ids = extract_backtick_identifiers(body);
        assert_eq!(ids, vec!["SessionManager", "parse_note"]);
    }
}
