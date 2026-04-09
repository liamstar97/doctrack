use std::path::Path;

use anyhow::{bail, Result};
use streaming_iterator::StreamingIterator;
use tracing::debug;

/// A code symbol extracted via tree-sitter.
#[derive(Debug, Clone)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    Constant,
    Method,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "function"),
            Self::Class => write!(f, "class"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Interface => write!(f, "interface"),
            Self::Module => write!(f, "module"),
            Self::Constant => write!(f, "constant"),
            Self::Method => write!(f, "method"),
        }
    }
}

/// Determine the language from file extension and extract symbols.
pub fn extract_symbols(path: &Path) -> Result<Vec<CodeSymbol>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let source = std::fs::read_to_string(path)?;

    match ext {
        "rs" => extract_with_language(tree_sitter_rust::LANGUAGE.into(), &source, rust_queries()),
        "py" => extract_with_language(tree_sitter_python::LANGUAGE.into(), &source, python_queries()),
        "js" | "jsx" => extract_with_language(tree_sitter_javascript::LANGUAGE.into(), &source, js_queries()),
        "ts" | "tsx" => extract_with_language(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), &source, ts_queries()),
        "go" => extract_with_language(tree_sitter_go::LANGUAGE.into(), &source, go_queries()),
        "java" => extract_with_language(tree_sitter_java::LANGUAGE.into(), &source, java_queries()),
        "c" | "h" => extract_with_language(tree_sitter_c::LANGUAGE.into(), &source, c_queries()),
        "cpp" | "cc" | "cxx" | "hpp" => extract_with_language(tree_sitter_cpp::LANGUAGE.into(), &source, cpp_queries()),
        _ => {
            debug!("unsupported file extension: {ext}");
            bail!("unsupported language: {ext}")
        }
    }
}

fn extract_with_language(
    language: tree_sitter::Language,
    source: &str,
    queries: &str,
) -> Result<Vec<CodeSymbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language)?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("failed to parse source"))?;

    let query = tree_sitter::Query::new(&language, queries)?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut symbols = Vec::new();
    let capture_names = query.capture_names();

    while let Some(m) = matches.next() {
        let mut name = None;
        let mut kind = None;
        let mut start_line = 0;
        let mut end_line = 0;

        for capture in m.captures {
            let capture_name = &capture_names[capture.index as usize];
            let text = &source[capture.node.byte_range()];

            match capture_name.as_ref() {
                "name" => {
                    name = Some(text.to_string());
                    start_line = capture.node.start_position().row as u32;
                    end_line = capture.node.end_position().row as u32;
                }
                "kind" => {
                    kind = Some(parse_kind(text));
                }
                _ => {}
            }
        }

        if let (Some(name), Some(kind)) = (name, kind) {
            symbols.push(CodeSymbol {
                name,
                kind,
                start_line,
                end_line,
            });
        }
    }

    Ok(symbols)
}

fn parse_kind(text: &str) -> SymbolKind {
    match text {
        "fn" | "func" | "def" | "function" => SymbolKind::Function,
        "class" => SymbolKind::Class,
        "struct" => SymbolKind::Struct,
        "enum" => SymbolKind::Enum,
        "interface" | "trait" => SymbolKind::Interface,
        "mod" | "module" | "package" => SymbolKind::Module,
        "const" | "static" => SymbolKind::Constant,
        _ => SymbolKind::Function,
    }
}

// --- Tree-sitter queries per language ---

fn rust_queries() -> &'static str {
    r#"
    (function_item name: (identifier) @name (#set! kind "fn")) @kind
    (struct_item name: (type_identifier) @name (#set! kind "struct")) @kind
    (enum_item name: (type_identifier) @name (#set! kind "enum")) @kind
    (trait_item name: (type_identifier) @name (#set! kind "trait")) @kind
    (impl_item trait: (type_identifier) @name (#set! kind "trait")) @kind
    (mod_item name: (identifier) @name (#set! kind "mod")) @kind
    (const_item name: (identifier) @name (#set! kind "const")) @kind
    "#
}

fn python_queries() -> &'static str {
    r#"
    (function_definition name: (identifier) @name (#set! kind "def")) @kind
    (class_definition name: (identifier) @name (#set! kind "class")) @kind
    "#
}

fn js_queries() -> &'static str {
    r#"
    (function_declaration name: (identifier) @name (#set! kind "function")) @kind
    (class_declaration name: (identifier) @name (#set! kind "class")) @kind
    (variable_declarator name: (identifier) @name value: (arrow_function)) @kind
    "#
}

fn ts_queries() -> &'static str {
    r#"
    (function_declaration name: (identifier) @name (#set! kind "function")) @kind
    (class_declaration name: (identifier) @name (#set! kind "class")) @kind
    (interface_declaration name: (type_identifier) @name (#set! kind "interface")) @kind
    (enum_declaration name: (identifier) @name (#set! kind "enum")) @kind
    (type_alias_declaration name: (type_identifier) @name (#set! kind "interface")) @kind
    "#
}

fn go_queries() -> &'static str {
    r#"
    (function_declaration name: (identifier) @name (#set! kind "func")) @kind
    (method_declaration name: (field_identifier) @name (#set! kind "func")) @kind
    (type_declaration (type_spec name: (type_identifier) @name (#set! kind "struct"))) @kind
    "#
}

fn java_queries() -> &'static str {
    r#"
    (method_declaration name: (identifier) @name (#set! kind "function")) @kind
    (class_declaration name: (identifier) @name (#set! kind "class")) @kind
    (interface_declaration name: (identifier) @name (#set! kind "interface")) @kind
    (enum_declaration name: (identifier) @name (#set! kind "enum")) @kind
    "#
}

fn c_queries() -> &'static str {
    r#"
    (function_definition declarator: (function_declarator declarator: (identifier) @name) (#set! kind "function")) @kind
    (struct_specifier name: (type_identifier) @name (#set! kind "struct")) @kind
    (enum_specifier name: (type_identifier) @name (#set! kind "enum")) @kind
    "#
}

fn cpp_queries() -> &'static str {
    r#"
    (function_definition declarator: (function_declarator declarator: (qualified_identifier) @name) (#set! kind "function")) @kind
    (function_definition declarator: (function_declarator declarator: (identifier) @name) (#set! kind "function")) @kind
    (class_specifier name: (type_identifier) @name (#set! kind "class")) @kind
    (struct_specifier name: (type_identifier) @name (#set! kind "struct")) @kind
    (enum_specifier name: (type_identifier) @name (#set! kind "enum")) @kind
    "#
}
