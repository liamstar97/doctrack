use std::path::{Path, PathBuf};

use anyhow::Result;
use dashmap::DashMap;
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::symbols::CodeSymbol;
use crate::vault::VaultNote;

/// Unique identifier for a code symbol.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SymbolId {
    pub file: PathBuf,
    pub name: String,
}

/// A link from a vault note to a code symbol, with surrounding context.
#[derive(Debug, Clone)]
pub struct DocLink {
    pub note_path: PathBuf,
    pub note_title: String,
    pub note_type: String,
    pub context: String,
}

/// A reference from a vault note to a code location.
#[derive(Debug, Clone)]
pub struct SymbolRef {
    pub symbol_id: SymbolId,
    pub confidence: MatchConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchConfidence {
    /// Explicit file path in frontmatter file-registry
    Exact,
    /// Symbol name found in backticks matching a parsed symbol
    Strong,
    /// Fuzzy title/filename match
    Fuzzy,
}

/// The bidirectional code↔documentation index.
pub struct Index {
    /// Code file path → symbols extracted from it
    pub code_symbols: DashMap<PathBuf, Vec<CodeSymbol>>,
    /// Vault note path → parsed note metadata
    pub vault_notes: DashMap<PathBuf, VaultNote>,
    /// Symbol → notes that reference it
    pub sym_to_docs: DashMap<SymbolId, Vec<DocLink>>,
    /// Note → symbols it references
    pub doc_to_syms: DashMap<PathBuf, Vec<SymbolRef>>,
    /// Filename → list of absolute paths (for bare filename resolution)
    pub file_lookup: DashMap<String, Vec<PathBuf>>,
    /// Project root (for resolving relative paths)
    pub root: PathBuf,
    /// Vault root (.doctrack/ directory)
    pub vault_root: PathBuf,
}

impl Index {
    pub fn new(root: PathBuf, vault_root: PathBuf) -> Self {
        Self {
            code_symbols: DashMap::new(),
            vault_notes: DashMap::new(),
            sym_to_docs: DashMap::new(),
            doc_to_syms: DashMap::new(),
            file_lookup: DashMap::new(),
            root,
            vault_root,
        }
    }

    /// Full index build — parse all vault notes + code symbols, then link them.
    pub fn build(&self) -> Result<()> {
        info!("building index from vault: {:?}", self.vault_root);

        // Phase 1: Build the project file lookup table
        self.build_file_lookup();
        info!("file lookup: {} unique filenames", self.file_lookup.len());

        // Phase 2: Parse all vault notes
        let notes = crate::vault::parse_vault(&self.vault_root)?;
        for note in &notes {
            self.vault_notes.insert(note.path.clone(), note.clone());
        }
        info!("indexed {} vault notes", self.vault_notes.len());

        // Phase 3: Resolve file refs and extract code symbols
        let code_files = self.collect_referenced_code_files();
        for file in &code_files {
            match crate::symbols::extract_symbols(file) {
                Ok(symbols) => {
                    self.code_symbols.insert(file.clone(), symbols);
                }
                Err(e) => {
                    debug!("skipping {}: {}", file.display(), e);
                }
            }
        }
        info!("indexed symbols from {} code files", self.code_symbols.len());

        // Phase 4: Build bidirectional links
        crate::matching::link_all(self);
        info!(
            "linked {} symbol→doc mappings, {} doc→symbol mappings",
            self.sym_to_docs.len(),
            self.doc_to_syms.len()
        );

        Ok(())
    }

    /// Walk the project tree and build a filename → [absolute paths] lookup.
    fn build_file_lookup(&self) {
        let skip_dirs = [
            "target", "node_modules", ".git", ".doctrack", ".idea",
            ".vscode", "build", "dist", "out", "__pycache__", ".gradle",
            "vendor", ".next",
        ];

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Skip hidden dirs (except the ones we explicitly handle) and known junk
                if e.file_type().is_dir() {
                    return !skip_dirs.contains(&name.as_ref())
                        && !name.starts_with('.');
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(filename) = path.file_name() {
                    let name = filename.to_string_lossy().to_string();
                    self.file_lookup
                        .entry(name)
                        .or_default()
                        .push(path.to_path_buf());
                }
            }
        }
    }

    /// Resolve a file reference to an absolute path.
    /// Handles relative paths, bare filenames, and abbreviated `...` paths.
    pub fn resolve_file_ref(&self, file_ref: &crate::vault::FileRef) -> Vec<PathBuf> {
        let path_str = file_ref.path.to_string_lossy();

        // Handle paths with ... abbreviation (e.g. "ci-reporting/src/main/java/.../config/Foo.java")
        if path_str.contains("/...") || path_str.contains(".../" ) {
            return self.resolve_abbreviated_path(&path_str);
        }

        if file_ref.is_bare_filename {
            // Bare filename like "CertificateInfo.java" — search the lookup table
            if let Some(paths) = self.file_lookup.get(&*path_str) {
                return paths.value().clone();
            }
            // Try with just the filename component in case path has a shallow prefix
            if let Some(filename) = file_ref.path.file_name() {
                let name = filename.to_string_lossy().to_string();
                if let Some(paths) = self.file_lookup.get(&name) {
                    return paths.value().clone();
                }
            }
            vec![]
        } else {
            // Relative or absolute path — resolve against project root
            let abs = self.root.join(&file_ref.path);
            if abs.exists() {
                vec![abs]
            } else {
                // Maybe the path is partial (e.g. "dto/ListenerInfoDto.java") — try suffix matching
                let suffix = file_ref.path.to_string_lossy();
                let mut matches = Vec::new();
                for entry in self.file_lookup.iter() {
                    for full_path in entry.value() {
                        if full_path.to_string_lossy().ends_with(suffix.as_ref()) {
                            matches.push(full_path.clone());
                        }
                    }
                }
                matches
            }
        }
    }

    /// Resolve an abbreviated path containing `...` as a wildcard.
    /// e.g. "ci-reporting/src/main/java/.../config/ReportingProperties.java"
    /// matches any file where the prefix and suffix segments align.
    fn resolve_abbreviated_path(&self, path_str: &str) -> Vec<PathBuf> {
        // Split on ... to get prefix and suffix segments
        let parts: Vec<&str> = path_str.split("...").collect();
        if parts.len() != 2 {
            return vec![];
        }

        let prefix = parts[0].trim_end_matches('/');
        let suffix = parts[1].trim_start_matches('/');

        let mut matches = Vec::new();

        for entry in self.file_lookup.iter() {
            for full_path in entry.value() {
                let rel = full_path.strip_prefix(&self.root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Both prefix and suffix must match
                if rel.starts_with(prefix) && rel.ends_with(suffix) {
                    matches.push(full_path.clone());
                }
            }
        }

        matches
    }

    /// Collect all code file paths referenced across vault notes.
    fn collect_referenced_code_files(&self) -> Vec<PathBuf> {
        let mut files = std::collections::HashSet::new();
        for entry in self.vault_notes.iter() {
            let note = entry.value();
            for file_ref in &note.file_refs {
                for resolved in self.resolve_file_ref(file_ref) {
                    files.insert(resolved);
                }
            }
        }
        info!("resolved {} unique code files from vault references", files.len());
        files.into_iter().collect()
    }

    /// Re-index a single vault note (on file change).
    pub fn reindex_note(&self, path: &Path) -> Result<()> {
        if let Ok(note) = crate::vault::parse_note(path) {
            self.vault_notes.insert(path.to_path_buf(), note);
            // TODO: rebuild links for this note
        }
        Ok(())
    }

    /// Re-index a single code file (on file change).
    pub fn reindex_code_file(&self, path: &Path) -> Result<()> {
        if let Ok(symbols) = crate::symbols::extract_symbols(path) {
            self.code_symbols.insert(path.to_path_buf(), symbols);
            // TODO: rebuild links for symbols in this file
        }
        Ok(())
    }

    /// Look up all documentation links for a given symbol.
    pub fn docs_for_symbol(&self, file: &Path, name: &str) -> Vec<DocLink> {
        let id = SymbolId {
            file: file.to_path_buf(),
            name: name.to_string(),
        };
        self.sym_to_docs
            .get(&id)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }

    /// Look up all symbol references for a given vault note.
    pub fn symbols_for_note(&self, note_path: &Path) -> Vec<SymbolRef> {
        self.doc_to_syms
            .get(note_path)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }
}
