//! Map command handler: /map — structural codebase understanding.

use crate::commands_search::{is_ast_grep_available, is_binary_extension, list_project_files};
use crate::format::*;
use regex::Regex;
use std::path::Path;

// ── /map — structural codebase understanding ────────────────────────────

/// Kind of structural symbol extracted from source code.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
    Type,
    Const,
    Impl,
    Module,
}

/// A structural symbol extracted from a source file.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub is_public: bool,
    pub line: usize,
}

/// Symbols extracted from a single file.
#[derive(Debug, Clone)]
pub struct FileSymbols {
    pub path: String,
    pub lines: usize,
    pub symbols: Vec<Symbol>,
}

/// Detect programming language from file extension.
pub fn detect_language(path: &str) -> Option<&'static str> {
    match Path::new(path).extension()?.to_str()? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "jsx" | "mjs" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "go" => Some("go"),
        "java" => Some("java"),
        _ => None,
    }
}

/// Extract structural symbols from source code for the given language.
///
/// Uses regex-based line-by-line extraction. This is intentionally simple —
/// false positives in comments are acceptable for v1.
pub fn extract_symbols(code: &str, language: &str) -> Vec<Symbol> {
    match language {
        "rust" => extract_rust_symbols(code),
        "python" => extract_python_symbols(code),
        "javascript" => extract_js_symbols(code),
        "typescript" => extract_ts_symbols(code),
        "go" => extract_go_symbols(code),
        "java" => extract_java_symbols(code),
        _ => Vec::new(),
    }
}

/// Extract symbols from Rust source code.
/// Skips content inside `#[cfg(test)]` modules.
fn extract_rust_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut in_test_module = false;
    let mut test_brace_depth: i32 = 0;

    let re_fn = Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
    let re_struct = Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?struct\s+(\w+)").unwrap();
    let re_enum = Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?enum\s+(\w+)").unwrap();
    let re_trait = Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?trait\s+(\w+)").unwrap();
    let re_impl = Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(.+?)(?:\s*\{|$)").unwrap();
    let re_const = Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?(?:const|static)\s+(\w+)").unwrap();
    let re_mod = Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?mod\s+(\w+)").unwrap();
    let re_cfg_test = Regex::new(r"#\[cfg\(test\)\]").unwrap();

    let mut next_is_test_mod = false;

    for (line_num, line) in code.lines().enumerate() {
        // Track #[cfg(test)] — the next `mod` after this attribute starts a test module
        if re_cfg_test.is_match(line) {
            next_is_test_mod = true;
            continue;
        }

        if in_test_module {
            // Count braces to find the end of the test module
            for ch in line.chars() {
                if ch == '{' {
                    test_brace_depth += 1;
                } else if ch == '}' {
                    test_brace_depth -= 1;
                    if test_brace_depth <= 0 {
                        in_test_module = false;
                        break;
                    }
                }
            }
            continue;
        }

        // If the previous line was #[cfg(test)], check if this line starts a mod
        if next_is_test_mod {
            if re_mod.is_match(line) {
                in_test_module = true;
                test_brace_depth = 0;
                for ch in line.chars() {
                    if ch == '{' {
                        test_brace_depth += 1;
                    } else if ch == '}' {
                        test_brace_depth -= 1;
                    }
                }
                if test_brace_depth <= 0 && line.contains('{') {
                    in_test_module = false;
                }
                next_is_test_mod = false;
                continue;
            }
            // If not a mod line, the #[cfg(test)] applied to something else
            next_is_test_mod = false;
        }

        let is_pub = line.trim_start().starts_with("pub");

        // impl blocks (check before fn to avoid matching fn inside impl detection)
        if let Some(caps) = re_impl.captures(line) {
            // Skip if line also matches fn (impl is not a fn)
            if !re_fn.is_match(line) {
                let impl_target = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let name = format!("impl {impl_target}");
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Impl,
                    is_public: is_pub,
                    line: line_num + 1,
                });
                continue;
            }
        }

        if let Some(caps) = re_fn.captures(line) {
            let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_struct.captures(line) {
            let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_enum.captures(line) {
            let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Enum,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_trait.captures(line) {
            let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Trait,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_const.captures(line) {
            let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Const,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_mod.captures(line) {
            let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Module,
                is_public: is_pub,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// Extract symbols from Python source code.
/// Only extracts top-level definitions (indentation level 0).
fn extract_python_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class = Regex::new(r"^class\s+(\w+)").unwrap();
    let re_func = Regex::new(r"^(?:async\s+)?def\s+(\w+)").unwrap();
    let re_const = Regex::new(r"^([A-Z][A-Z0-9_]*)\s*=").unwrap();

    for (line_num, line) in code.lines().enumerate() {
        // Only consider top-level (no indentation)
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: !line.starts_with('_'),
                line: line_num + 1,
            });
        } else if let Some(caps) = re_func.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = !name.starts_with('_');
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_const.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Const,
                is_public: true,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// Extract symbols from JavaScript source code.
fn extract_js_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_export_func =
        Regex::new(r"^(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s+(\w+)").unwrap();
    let re_class = Regex::new(r"^(?:export\s+(?:default\s+)?)?class\s+(\w+)").unwrap();
    let re_const = Regex::new(r"^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=").unwrap();

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();

        if let Some(caps) = re_export_func.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_class.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_const.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Const,
                is_public,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// Extract symbols from TypeScript source code.
/// Includes all JS patterns plus interface and type.
fn extract_ts_symbols(code: &str) -> Vec<Symbol> {
    // Start with JS symbols
    let mut symbols = extract_js_symbols(code);

    let re_interface = Regex::new(r"^(?:export\s+)?interface\s+(\w+)").unwrap();
    let re_type = Regex::new(r"^(?:export\s+)?type\s+(\w+)\s*[=<]").unwrap();

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();

        if let Some(caps) = re_interface.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Interface,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_type.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Type,
                is_public,
                line: line_num + 1,
            });
        }
    }

    // Sort by line number since we appended TS-specific symbols after JS ones
    symbols.sort_by_key(|s| s.line);
    symbols
}

/// Extract symbols from Go source code.
fn extract_go_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_func = Regex::new(r"^func\s+(\w+)\s*\(").unwrap();
    let re_method = Regex::new(r"^func\s+\([^)]+\)\s+(\w+)\s*\(").unwrap();
    let re_type_struct = Regex::new(r"^type\s+(\w+)\s+struct\b").unwrap();
    let re_type_interface = Regex::new(r"^type\s+(\w+)\s+interface\b").unwrap();
    let re_const = Regex::new(r"^(?:const|var)\s+(\w+)").unwrap();

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();

        if let Some(caps) = re_method.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_func.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_type_struct.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_type_interface.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Interface,
                is_public,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_const.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Const,
                is_public,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// Extract symbols from Java source code.
fn extract_java_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class =
        Regex::new(r"^\s*(?:public\s+)?(?:abstract\s+)?(?:final\s+)?class\s+(\w+)").unwrap();
    let re_interface = Regex::new(r"^\s*(?:public\s+)?interface\s+(\w+)").unwrap();
    let re_enum = Regex::new(r"^\s*(?:public\s+)?enum\s+(\w+)").unwrap();
    let re_method = Regex::new(
        r"^\s*(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(?:[\w<>\[\],\s]+)\s+(\w+)\s*\(",
    )
    .unwrap();

    for (line_num, line) in code.lines().enumerate() {
        let is_pub = line.trim_start().starts_with("public");

        if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_interface.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Interface,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_enum.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Enum,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_method.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            // Skip common Java keywords that match the method regex
            if ![
                "if",
                "for",
                "while",
                "switch",
                "catch",
                "return",
                "new",
                "class",
                "interface",
            ]
            .contains(&name.as_str())
            {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Function,
                    is_public: is_pub,
                    line: line_num + 1,
                });
            }
        }
    }

    symbols
}

/// Build the ast-grep inline rule YAML for a given language.
///
/// Returns a YAML string targeting structural symbol kinds (functions, structs,
/// classes, etc.) appropriate for the language.
fn ast_grep_rule_for_language(language: &str) -> Option<String> {
    let rule = match language {
        "rust" => {
            "id: symbols\nlanguage: Rust\nrule:\n  any:\n    \
             - kind: function_item\n    \
             - kind: struct_item\n    \
             - kind: enum_item\n    \
             - kind: trait_item\n    \
             - kind: impl_item\n    \
             - kind: const_item\n    \
             - kind: mod_item"
        }
        "python" => {
            "id: symbols\nlanguage: Python\nrule:\n  any:\n    \
             - kind: function_definition\n    \
             - kind: class_definition"
        }
        "javascript" => {
            "id: symbols\nlanguage: JavaScript\nrule:\n  any:\n    \
             - kind: function_declaration\n    \
             - kind: class_declaration\n    \
             - kind: lexical_declaration\n    \
             - kind: export_statement"
        }
        "typescript" => {
            "id: symbols\nlanguage: TypeScript\nrule:\n  any:\n    \
             - kind: function_declaration\n    \
             - kind: class_declaration\n    \
             - kind: interface_declaration\n    \
             - kind: type_alias_declaration\n    \
             - kind: lexical_declaration\n    \
             - kind: export_statement"
        }
        "go" => {
            "id: symbols\nlanguage: Go\nrule:\n  any:\n    \
             - kind: function_declaration\n    \
             - kind: method_declaration\n    \
             - kind: type_declaration"
        }
        "java" => {
            "id: symbols\nlanguage: Java\nrule:\n  any:\n    \
             - kind: class_declaration\n    \
             - kind: interface_declaration\n    \
             - kind: enum_declaration\n    \
             - kind: method_declaration"
        }
        _ => return None,
    };
    Some(rule.to_string())
}

/// Parse ast-grep JSON output into Symbol entries.
///
/// Each match from `sg scan --json` has "text", "range.start.line", etc.
/// We parse the first line of text to extract the symbol kind and name.
pub fn parse_ast_grep_symbols(json_str: &str, language: &str) -> Vec<Symbol> {
    // ast-grep outputs a JSON array of match objects
    let arr: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut symbols = Vec::new();
    for item in &arr {
        let text = match item.get("text").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };
        let line = item
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(|l| l.as_u64())
            .unwrap_or(0) as usize;

        // Extract symbol info from the first line of matched text
        let first_line = text.lines().next().unwrap_or("");
        if let Some(sym) = parse_symbol_from_text(first_line, language, line) {
            symbols.push(sym);
        }
    }
    symbols
}

/// Parse a symbol kind and name from a source code line.
///
/// Handles patterns like:
///   - `pub fn name(...)` / `fn name(...)`
///   - `pub struct Name` / `struct Name`
///   - `impl Name` / `impl Trait for Name`
///   - `class Name` / `def name(...)` / `func name(...)` etc.
fn parse_symbol_from_text(line: &str, language: &str, line_num: usize) -> Option<Symbol> {
    let trimmed = line.trim();
    let is_public = trimmed.starts_with("pub ")
        || trimmed.starts_with("export ")
        || (language == "go" && first_ident_uppercase(trimmed));

    // Strip leading visibility/decorators
    let stripped = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .or_else(|| trimmed.strip_prefix("export default "))
        .or_else(|| trimmed.strip_prefix("export "))
        .or_else(|| trimmed.strip_prefix("async "))
        .unwrap_or(trimmed);

    // Also handle "async" after pub
    let stripped = stripped.strip_prefix("async ").unwrap_or(stripped);

    // Match keyword → (SymbolKind, what-follows)
    if let Some(rest) = stripped.strip_prefix("fn ") {
        let name = ident_before(rest, &['(', '<', ' ', '{']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("struct ") {
        let name = ident_before(rest, &['(', '<', ' ', '{', ';']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Struct,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("enum ") {
        let name = ident_before(rest, &['<', ' ', '{']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Enum,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("trait ") {
        let name = ident_before(rest, &['<', ' ', '{', ':']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Trait,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("impl ") {
        // "impl Foo" or "impl Trait for Foo"
        let name = rest.split([' ', '<', '{']).next().unwrap_or("").trim();
        if name.is_empty() {
            return None;
        }
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Impl,
            is_public: false,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("mod ") {
        let name = ident_before(rest, &[' ', '{', ';']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Module,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("const ") {
        let name = ident_before(rest, &[':', ' ', '=']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Const,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("class ") {
        let name = ident_before(rest, &['(', ' ', '{', ':']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Class,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("interface ") {
        let name = ident_before(rest, &['<', ' ', '{']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Interface,
            is_public,
            line: line_num,
        });
    }
    if let Some(rest) = stripped.strip_prefix("type ") {
        let name = ident_before(rest, &['<', ' ', '=']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Type,
            is_public,
            line: line_num,
        });
    }
    // Python: def/async def
    if let Some(rest) = stripped.strip_prefix("def ") {
        let name = ident_before(rest, &['(', ' ', ':']);
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            is_public: !name.starts_with('_'),
            line: line_num,
        });
    }
    // Go: func (receiver) Name(...) or func Name(...)
    if let Some(rest) = stripped.strip_prefix("func ") {
        let rest = if rest.starts_with('(') {
            // Method: skip receiver
            rest.find(')').map(|i| rest[i + 1..].trim()).unwrap_or(rest)
        } else {
            rest
        };
        let name = ident_before(rest, &['(', '<', ' ', '{']);
        let is_go_pub = name.chars().next().is_some_and(|c| c.is_uppercase());
        return Some(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            is_public: is_go_pub,
            line: line_num,
        });
    }

    None
}

/// Extract the identifier from the start of `s`, stopping at any of `stops`.
fn ident_before<'a>(s: &'a str, stops: &[char]) -> &'a str {
    let end = s.find(stops).unwrap_or(s.len());
    s[..end].trim()
}

/// Check if the first identifier in a Go declaration is uppercase (exported).
fn first_ident_uppercase(line: &str) -> bool {
    // Skip "func ", "type ", etc.
    let after_kw = line
        .strip_prefix("func ")
        .or_else(|| line.strip_prefix("type "))
        .or_else(|| line.strip_prefix("const "))
        .or_else(|| line.strip_prefix("var "))
        .unwrap_or(line);
    // For methods, skip receiver
    let after_kw = if after_kw.starts_with('(') {
        after_kw
            .find(')')
            .map(|i| after_kw[i + 1..].trim())
            .unwrap_or(after_kw)
    } else {
        after_kw
    };
    after_kw.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Try to extract symbols from a file using ast-grep.
///
/// Returns `Some(symbols)` if ast-grep succeeds, `None` if sg is not available
/// or the extraction fails (callers should fall back to regex).
pub fn extract_symbols_ast_grep(path: &str, language: &str) -> Option<Vec<Symbol>> {
    let rule = ast_grep_rule_for_language(language)?;

    let output = std::process::Command::new("sg")
        .arg("scan")
        .arg("--json")
        .arg("--inline-rules")
        .arg(&rule)
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Some(Vec::new());
    }

    let symbols = parse_ast_grep_symbols(&stdout, language);
    Some(symbols)
}

/// Which backend was used for symbol extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapBackend {
    AstGrep,
    Regex,
}

/// Build a repo map by scanning project files and extracting symbols.
///
/// If `root` is Some, only scan files under that path.
/// If `public_only` is true, filter to only public/exported symbols.
pub fn build_repo_map(root: Option<&str>, public_only: bool) -> Vec<FileSymbols> {
    build_repo_map_with_backend(root, public_only, false).0
}

/// Build a repo map with explicit backend control.
///
/// When `force_regex` is true, skip ast-grep even if available.
/// Returns the file symbols and which backend was actually used.
pub fn build_repo_map_with_backend(
    root: Option<&str>,
    public_only: bool,
    force_regex: bool,
) -> (Vec<FileSymbols>, MapBackend) {
    let files = list_project_files();
    let mut result = Vec::new();

    // Resolve git toplevel so file reads use absolute paths,
    // preventing CWD races when parallel tests call set_current_dir.
    let toplevel = crate::git::run_git(&["rev-parse", "--show-toplevel"]).ok();

    // Check ast-grep availability once upfront
    let use_ast_grep = !force_regex && is_ast_grep_available();
    let backend = if use_ast_grep {
        MapBackend::AstGrep
    } else {
        MapBackend::Regex
    };

    for path in &files {
        // If a root filter is given, only include matching files
        if let Some(root_path) = root {
            if !path.starts_with(root_path) {
                continue;
            }
        }

        if is_binary_extension(path) {
            continue;
        }
        let lang = match detect_language(path) {
            Some(l) => l,
            None => continue,
        };
        // Use absolute path for file I/O to avoid CWD dependency
        let abs_path = if let Some(ref tl) = toplevel {
            std::path::Path::new(tl)
                .join(path)
                .to_string_lossy()
                .to_string()
        } else {
            path.clone()
        };
        let content = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let line_count = content.lines().count();

        // Try ast-grep first, fall back to regex
        let mut symbols = if use_ast_grep {
            extract_symbols_ast_grep(path, lang).unwrap_or_else(|| extract_symbols(&content, lang))
        } else {
            extract_symbols(&content, lang)
        };

        if public_only {
            symbols.retain(|s| s.is_public);
        }
        if !symbols.is_empty() {
            result.push(FileSymbols {
                path: path.clone(),
                lines: line_count,
                symbols,
            });
        }
    }

    // Sort by line count descending (biggest/most important files first)
    result.sort_by_key(|b| std::cmp::Reverse(b.lines));
    (result, backend)
}

/// Format the repo map with ANSI colors for REPL display.
pub fn format_repo_map_colored(entries: &[FileSymbols]) -> String {
    if entries.is_empty() {
        return format!("{DIM}  (no structural symbols found){RESET}\n");
    }

    let mut output = String::new();

    for entry in entries {
        output.push_str(&format!(
            "\n{BOLD_CYAN}{}{RESET} {DIM}({} lines){RESET}\n",
            entry.path, entry.lines
        ));
        for sym in &entry.symbols {
            let kind_colored = match sym.kind {
                SymbolKind::Function => format!("{GREEN}fn{RESET}"),
                SymbolKind::Struct => format!("{YELLOW}struct{RESET}"),
                SymbolKind::Enum => format!("{YELLOW}enum{RESET}"),
                SymbolKind::Trait => format!("{YELLOW}trait{RESET}"),
                SymbolKind::Interface => format!("{YELLOW}interface{RESET}"),
                SymbolKind::Class => format!("{YELLOW}class{RESET}"),
                SymbolKind::Type => format!("{YELLOW}type{RESET}"),
                SymbolKind::Const => format!("{CYAN}const{RESET}"),
                SymbolKind::Impl => format!("{MAGENTA}impl{RESET}"),
                SymbolKind::Module => format!("{MAGENTA}mod{RESET}"),
            };
            let vis = if sym.is_public {
                format!("{GREEN}pub{RESET} ")
            } else {
                String::new()
            };
            output.push_str(&format!("  {vis}{kind_colored} {}\n", sym.name));
        }
    }

    output
}

/// Format the repo map as plain text for the system prompt.
///
/// Condensed format: no blank lines, public symbols only, capped at `max_chars`.
pub fn format_repo_map(entries: &[FileSymbols]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    for entry in entries {
        output.push_str(&format!("{} ({} lines)\n", entry.path, entry.lines));
        for sym in &entry.symbols {
            let kind_label = match sym.kind {
                SymbolKind::Function => "fn",
                SymbolKind::Struct => "struct",
                SymbolKind::Enum => "enum",
                SymbolKind::Trait => "trait",
                SymbolKind::Interface => "interface",
                SymbolKind::Class => "class",
                SymbolKind::Type => "type",
                SymbolKind::Const => "const",
                SymbolKind::Impl => "impl",
                SymbolKind::Module => "mod",
            };
            output.push_str(&format!("  {kind_label} {}\n", sym.name));
        }
    }

    output
}

/// Generate a repo map for the system prompt, capped at `max_chars` characters.
///
/// Returns `None` if no supported source files are found.
pub fn generate_repo_map_for_prompt_with_limit(max_chars: usize) -> Option<String> {
    let entries = build_repo_map(None, true);
    if entries.is_empty() {
        return None;
    }

    let full = format_repo_map(&entries);
    if full.len() <= max_chars {
        Some(full)
    } else {
        // Truncate: include files until we hit the limit
        let mut output = String::new();
        for entry in &entries {
            let mut file_block = format!("{} ({} lines)\n", entry.path, entry.lines);
            for sym in &entry.symbols {
                let kind_label = match sym.kind {
                    SymbolKind::Function => "fn",
                    SymbolKind::Struct => "struct",
                    SymbolKind::Enum => "enum",
                    SymbolKind::Trait => "trait",
                    SymbolKind::Interface => "interface",
                    SymbolKind::Class => "class",
                    SymbolKind::Type => "type",
                    SymbolKind::Const => "const",
                    SymbolKind::Impl => "impl",
                    SymbolKind::Module => "mod",
                };
                file_block.push_str(&format!("  {kind_label} {}\n", sym.name));
            }
            if output.len() + file_block.len() > max_chars {
                output.push_str("  ...\n");
                break;
            }
            output.push_str(&file_block);
        }
        Some(output)
    }
}

/// Default max characters for the system prompt repo map (~16K chars ≈ ~4K tokens).
const REPO_MAP_MAX_CHARS: usize = 16_000;

/// Generate a repo map for the system prompt with the default size cap.
pub fn generate_repo_map_for_prompt() -> Option<String> {
    generate_repo_map_for_prompt_with_limit(REPO_MAP_MAX_CHARS)
}

/// Handle the `/map` REPL command: show structural symbols from the codebase.
///
/// Usage: `/map [path]` — show all symbols
/// Usage: `/map --all [path]` — include private symbols
/// Usage: `/map --regex [path]` — force regex backend even if ast-grep is available
pub fn handle_map(input: &str) {
    let rest = input.strip_prefix("/map").unwrap_or("").trim();

    let mut show_all = false;
    let mut force_regex = false;
    let mut path_filter: Option<&str> = None;

    for part in rest.split_whitespace() {
        match part {
            "--all" => show_all = true,
            "--regex" => force_regex = true,
            _ => path_filter = Some(part),
        }
    }

    println!("{DIM}  Building repo map...{RESET}");
    let public_only = !show_all;
    let (entries, backend) = build_repo_map_with_backend(path_filter, public_only, force_regex);

    if entries.is_empty() {
        println!("{DIM}  (no supported source files with symbols found){RESET}\n");
        return;
    }

    let total_symbols: usize = entries.iter().map(|e| e.symbols.len()).sum();
    let total_files = entries.len();

    let formatted = format_repo_map_colored(&entries);
    print!("{formatted}");

    let backend_label = match backend {
        MapBackend::AstGrep => "using ast-grep",
        MapBackend::Regex => "using regex",
    };

    println!(
        "\n{DIM}  {} symbol{} across {} file{} ({backend_label}){RESET}\n",
        total_symbols,
        if total_symbols == 1 { "" } else { "s" },
        total_files,
        if total_files == 1 { "" } else { "s" },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::KNOWN_COMMANDS;

    // ── /map: SymbolKind, Symbol, extract_symbols ─────────────────────

    #[test]
    fn extract_rust_symbols_basic() {
        let code = r#"
pub fn hello(name: &str) -> String { todo!() }
fn private_fn() {}
pub struct MyStruct {
    field: i32,
}
pub enum Color { Red, Green, Blue }
pub trait Drawable { fn draw(&self); }
impl MyStruct {
    pub fn new() -> Self { todo!() }
}
const MAX: usize = 100;
"#;
        let symbols = extract_symbols(code, "rust");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "hello" && s.kind == SymbolKind::Function),
            "should find pub fn hello"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MyStruct" && s.kind == SymbolKind::Struct),
            "should find pub struct MyStruct"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Color" && s.kind == SymbolKind::Enum),
            "should find pub enum Color"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Drawable" && s.kind == SymbolKind::Trait),
            "should find pub trait Drawable"
        );
        assert!(
            symbols.iter().any(|s| s.name.contains("impl MyStruct")),
            "should find impl MyStruct"
        );
    }

    #[test]
    fn extract_rust_skips_test_module() {
        let code = r#"
pub fn real_fn() {}

#[cfg(test)]
mod tests {
    fn test_something() {}
}
"#;
        let symbols = extract_symbols(code, "rust");
        assert!(
            symbols.iter().any(|s| s.name == "real_fn"),
            "should find real_fn"
        );
        assert!(
            !symbols.iter().any(|s| s.name == "test_something"),
            "should skip test_something inside #[cfg(test)]"
        );
    }

    #[test]
    fn extract_rust_pub_visibility() {
        let code = "pub fn public_one() {}\nfn private_one() {}\n";
        let symbols = extract_symbols(code, "rust");
        let public = symbols.iter().find(|s| s.name == "public_one").unwrap();
        assert!(public.is_public);
        let private = symbols.iter().find(|s| s.name == "private_one").unwrap();
        assert!(!private.is_public);
    }

    #[test]
    fn extract_python_symbols() {
        let code = r#"
class MyClass:
    def method(self):
        pass

def top_level_func(x, y):
    return x + y

async def async_handler(req):
    pass

MAX_SIZE = 1024
"#;
        let symbols = extract_symbols(code, "python");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MyClass" && s.kind == SymbolKind::Class),
            "should find class MyClass"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "top_level_func" && s.kind == SymbolKind::Function),
            "should find def top_level_func"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "async_handler" && s.kind == SymbolKind::Function),
            "should find async def async_handler"
        );
    }

    #[test]
    fn extract_python_skips_indented() {
        let code = "class Foo:\n    def method(self):\n        pass\n";
        let symbols = extract_symbols(code, "python");
        // `method` is indented, so should NOT be extracted as top-level
        assert!(
            !symbols.iter().any(|s| s.name == "method"),
            "should skip indented def method"
        );
        assert!(symbols.iter().any(|s| s.name == "Foo"));
    }

    #[test]
    fn extract_js_symbols() {
        let code = r#"
export function fetchData(url) { }
function helper() { }
export class ApiClient { }
const BASE_URL = "https://api.example.com";
export default function main() { }
"#;
        let symbols = extract_symbols(code, "javascript");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "fetchData" && s.kind == SymbolKind::Function),
            "should find export function fetchData"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "ApiClient" && s.kind == SymbolKind::Class),
            "should find export class ApiClient"
        );
    }

    #[test]
    fn extract_typescript_symbols() {
        let code = r#"
interface Config { key: string; }
type Result<T> = { data: T; error?: string; }
export class Service { }
"#;
        let symbols = extract_symbols(code, "typescript");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Config" && s.kind == SymbolKind::Interface),
            "should find interface Config"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Result" && s.kind == SymbolKind::Type),
            "should find type Result"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Service" && s.kind == SymbolKind::Class),
            "should find export class Service"
        );
    }

    #[test]
    fn extract_go_symbols() {
        let code = r#"
func main() { }
func (s *Server) Handle(w http.ResponseWriter, r *http.Request) { }
type Server struct { port int }
type Handler interface { Handle() }
"#;
        let symbols = extract_symbols(code, "go");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "main" && s.kind == SymbolKind::Function),
            "should find func main"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Server" && s.kind == SymbolKind::Struct),
            "should find type Server struct"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Handler" && s.kind == SymbolKind::Interface),
            "should find type Handler interface"
        );
    }

    #[test]
    fn extract_go_method() {
        let code = "func (s *Server) Handle(w http.ResponseWriter) { }\n";
        let symbols = extract_symbols(code, "go");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Handle" && s.kind == SymbolKind::Function),
            "should find method Handle"
        );
    }

    #[test]
    fn extract_java_symbols() {
        let code = r#"
public class MyApp {
    public void run() { }
    private int count() { return 0; }
}
public interface Runnable {
    void run();
}
public enum Status { OK, ERROR }
"#;
        let symbols = extract_symbols(code, "java");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MyApp" && s.kind == SymbolKind::Class),
            "should find public class MyApp"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Runnable" && s.kind == SymbolKind::Interface),
            "should find public interface Runnable"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Status" && s.kind == SymbolKind::Enum),
            "should find public enum Status"
        );
    }

    // ── detect_language ──────────────────────────────────────────────

    #[test]
    fn detect_language_known_extensions() {
        assert_eq!(detect_language("main.rs"), Some("rust"));
        assert_eq!(detect_language("app.py"), Some("python"));
        assert_eq!(detect_language("index.js"), Some("javascript"));
        assert_eq!(detect_language("index.jsx"), Some("javascript"));
        assert_eq!(detect_language("lib.ts"), Some("typescript"));
        assert_eq!(detect_language("lib.tsx"), Some("typescript"));
        assert_eq!(detect_language("main.go"), Some("go"));
        assert_eq!(detect_language("App.java"), Some("java"));
    }

    #[test]
    fn detect_language_unknown_extension() {
        assert_eq!(detect_language("README.md"), None);
        assert_eq!(detect_language("Cargo.toml"), None);
        assert_eq!(detect_language("file.txt"), None);
    }

    // ── format_repo_map ─────────────────────────────────────────────

    #[test]
    fn format_repo_map_empty_project() {
        let entries: Vec<FileSymbols> = vec![];
        let result = format_repo_map(&entries);
        assert!(
            result.is_empty(),
            "empty entries should produce empty string"
        );
    }

    #[test]
    fn format_repo_map_basic() {
        let entries = vec![FileSymbols {
            path: "src/main.rs".to_string(),
            lines: 100,
            symbols: vec![
                Symbol {
                    name: "main".to_string(),
                    kind: SymbolKind::Function,
                    is_public: false,
                    line: 1,
                },
                Symbol {
                    name: "Config".to_string(),
                    kind: SymbolKind::Struct,
                    is_public: true,
                    line: 10,
                },
            ],
        }];
        let result = format_repo_map(&entries);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("100 lines"));
        assert!(result.contains("fn main"));
        assert!(result.contains("struct Config"));
    }

    // ── generate_repo_map_for_prompt_with_limit ─────────────────────

    #[test]
    fn generate_repo_map_respects_size_limit() {
        // We can't control what files are in the repo during tests,
        // but we can verify the function doesn't panic and respects limits
        let result = generate_repo_map_for_prompt_with_limit(1000);
        if let Some(map) = result {
            assert!(
                map.len() <= 1010, // small tolerance for "..." truncation
                "map should respect size limit, got {} chars",
                map.len()
            );
        }
    }

    #[test]
    fn generate_repo_map_for_prompt_does_not_panic() {
        // Should not panic even if no source files exist
        let _result = generate_repo_map_for_prompt();
    }

    // ── handle_map ──────────────────────────────────────────────────

    #[test]
    fn handle_map_no_panic_empty() {
        // Should not panic with default input
        handle_map("/map");
    }

    #[test]
    fn handle_map_no_panic_with_path() {
        // Should not panic with a path argument
        handle_map("/map src/");
    }

    #[test]
    fn handle_map_no_panic_with_all() {
        // Should not panic with --all flag
        handle_map("/map --all");
    }

    // ── /map in KNOWN_COMMANDS and help ─────────────────────────────

    #[test]
    fn map_in_known_commands() {
        assert!(
            KNOWN_COMMANDS.contains(&"/map"),
            "/map should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn map_in_help_text() {
        let help = crate::help::help_text();
        assert!(
            help.contains("/map"),
            "help_text should mention /map command"
        );
    }

    #[test]
    fn map_has_detailed_help() {
        use crate::help::command_help;
        let help = command_help("map");
        assert!(help.is_some(), "/map should have detailed help text");
        let text = help.unwrap();
        assert!(
            text.contains("structural"),
            "map help should describe structural mapping"
        );
    }

    // ── ast-grep backend ───────────────────────────────────────────

    #[test]
    fn ast_grep_rule_exists_for_supported_languages() {
        for lang in &["rust", "python", "javascript", "typescript", "go", "java"] {
            assert!(
                ast_grep_rule_for_language(lang).is_some(),
                "should have ast-grep rule for {lang}"
            );
        }
    }

    #[test]
    fn ast_grep_rule_none_for_unknown_language() {
        assert!(ast_grep_rule_for_language("haskell").is_none());
        assert!(ast_grep_rule_for_language("").is_none());
    }

    #[test]
    fn parse_ast_grep_symbols_empty_input() {
        let symbols = parse_ast_grep_symbols("[]", "rust");
        assert!(symbols.is_empty());
    }

    #[test]
    fn parse_ast_grep_symbols_invalid_json() {
        let symbols = parse_ast_grep_symbols("not json", "rust");
        assert!(symbols.is_empty());
    }

    #[test]
    fn parse_ast_grep_symbols_rust_function() {
        let json = r#"[{
            "text": "pub fn my_func(x: i32) -> bool {\n    true\n}",
            "range": {"start": {"line": 5, "column": 0}, "end": {"line": 7, "column": 1}},
            "file": "src/lib.rs",
            "ruleId": "symbols"
        }]"#;
        let symbols = parse_ast_grep_symbols(json, "rust");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "my_func");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert!(symbols[0].is_public);
        assert_eq!(symbols[0].line, 5);
    }

    #[test]
    fn parse_ast_grep_symbols_rust_struct() {
        let json = r#"[{
            "text": "pub struct Config {\n    name: String\n}",
            "range": {"start": {"line": 1, "column": 0}, "end": {"line": 3, "column": 1}},
            "file": "src/lib.rs",
            "ruleId": "symbols"
        }]"#;
        let symbols = parse_ast_grep_symbols(json, "rust");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Config");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert!(symbols[0].is_public);
    }

    #[test]
    fn parse_ast_grep_symbols_rust_impl() {
        let json = r#"[{
            "text": "impl Config {\n    fn new() -> Self { todo!() }\n}",
            "range": {"start": {"line": 10, "column": 0}, "end": {"line": 12, "column": 1}},
            "file": "src/lib.rs",
            "ruleId": "symbols"
        }]"#;
        let symbols = parse_ast_grep_symbols(json, "rust");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Config");
        assert_eq!(symbols[0].kind, SymbolKind::Impl);
    }

    #[test]
    fn parse_ast_grep_symbols_rust_enum_and_trait() {
        let json = r#"[
            {
                "text": "pub enum Color {\n    Red,\n    Blue\n}",
                "range": {"start": {"line": 1, "column": 0}, "end": {"line": 4, "column": 1}},
                "file": "src/lib.rs",
                "ruleId": "symbols"
            },
            {
                "text": "pub trait Drawable {\n    fn draw(&self);\n}",
                "range": {"start": {"line": 6, "column": 0}, "end": {"line": 8, "column": 1}},
                "file": "src/lib.rs",
                "ruleId": "symbols"
            }
        ]"#;
        let symbols = parse_ast_grep_symbols(json, "rust");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, SymbolKind::Enum);
        assert_eq!(symbols[1].name, "Drawable");
        assert_eq!(symbols[1].kind, SymbolKind::Trait);
    }

    #[test]
    fn parse_ast_grep_symbols_private_fn() {
        let json = r#"[{
            "text": "fn helper() {\n    // ...\n}",
            "range": {"start": {"line": 0, "column": 0}, "end": {"line": 2, "column": 1}},
            "file": "src/lib.rs",
            "ruleId": "symbols"
        }]"#;
        let symbols = parse_ast_grep_symbols(json, "rust");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "helper");
        assert!(!symbols[0].is_public);
    }

    #[test]
    fn parse_ast_grep_symbols_python() {
        let json = r#"[
            {
                "text": "def process(data):\n    pass",
                "range": {"start": {"line": 0, "column": 0}, "end": {"line": 1, "column": 8}},
                "file": "main.py",
                "ruleId": "symbols"
            },
            {
                "text": "class Handler:\n    pass",
                "range": {"start": {"line": 3, "column": 0}, "end": {"line": 4, "column": 8}},
                "file": "main.py",
                "ruleId": "symbols"
            }
        ]"#;
        let symbols = parse_ast_grep_symbols(json, "python");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "process");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[1].name, "Handler");
        assert_eq!(symbols[1].kind, SymbolKind::Class);
    }

    #[test]
    fn parse_ast_grep_symbols_go() {
        let json = r#"[{
            "text": "func (s *Server) HandleRequest(w http.ResponseWriter, r *http.Request) {",
            "range": {"start": {"line": 10, "column": 0}, "end": {"line": 20, "column": 1}},
            "file": "server.go",
            "ruleId": "symbols"
        }]"#;
        let symbols = parse_ast_grep_symbols(json, "go");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "HandleRequest");
        assert!(symbols[0].is_public, "Go exported func should be public");
    }

    #[test]
    fn parse_symbol_from_text_various_rust() {
        let sym = parse_symbol_from_text("pub const MAX_SIZE: usize = 100;", "rust", 1).unwrap();
        assert_eq!(sym.name, "MAX_SIZE");
        assert_eq!(sym.kind, SymbolKind::Const);
        assert!(sym.is_public);

        let sym = parse_symbol_from_text("mod utils {", "rust", 5).unwrap();
        assert_eq!(sym.name, "utils");
        assert_eq!(sym.kind, SymbolKind::Module);

        let sym = parse_symbol_from_text("pub async fn serve()", "rust", 3).unwrap();
        assert_eq!(sym.name, "serve");
        assert_eq!(sym.kind, SymbolKind::Function);
        assert!(sym.is_public);
    }

    #[test]
    fn parse_symbol_from_text_typescript() {
        let sym =
            parse_symbol_from_text("export interface ApiResponse {", "typescript", 1).unwrap();
        assert_eq!(sym.name, "ApiResponse");
        assert_eq!(sym.kind, SymbolKind::Interface);
        assert!(sym.is_public);

        let sym = parse_symbol_from_text("type Config = {", "typescript", 5).unwrap();
        assert_eq!(sym.name, "Config");
        assert_eq!(sym.kind, SymbolKind::Type);
    }

    #[test]
    fn extract_symbols_ast_grep_returns_none_when_sg_unavailable() {
        // If the system `sg` is NOT ast-grep (or not installed),
        // extract_symbols_ast_grep should return None (graceful fallback).
        // This test just verifies it doesn't panic.
        let result = extract_symbols_ast_grep("nonexistent_file.rs", "rust");
        // Result is None (file doesn't exist) or Some (if sg happened to work)
        // Either way, no panic.
        let _ = result;
    }

    #[test]
    fn build_repo_map_with_regex_backend() {
        let (entries, backend) = build_repo_map_with_backend(Some("src/"), true, true);
        assert_eq!(backend, MapBackend::Regex);
        // entries may be empty if another parallel test changed CWD via
        // set_current_dir (global process state race). Only assert non-empty
        // when we can confirm we're still in the project root.
        let in_project_root = std::path::Path::new("Cargo.toml").exists();
        if in_project_root {
            assert!(
                !entries.is_empty(),
                "should find symbols in src/ with regex backend"
            );
        }
    }

    #[test]
    fn handle_map_no_panic_with_regex_flag() {
        handle_map("/map --regex");
    }

    #[test]
    fn handle_map_no_panic_with_regex_and_all() {
        handle_map("/map --regex --all");
    }

    #[test]
    fn map_backend_display() {
        // Verify MapBackend values match expected variants
        assert_eq!(MapBackend::AstGrep, MapBackend::AstGrep);
        assert_eq!(MapBackend::Regex, MapBackend::Regex);
        assert_ne!(MapBackend::AstGrep, MapBackend::Regex);
    }
}
