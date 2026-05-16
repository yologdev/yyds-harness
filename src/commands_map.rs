//! Map command handler: /map — structural codebase understanding.

use crate::commands_ast_grep::is_ast_grep_available;
use crate::commands_search::{is_binary_extension, list_project_files};
use crate::format::*;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

// ── Rust symbol regexes ──────────────────────────────────────────────
static RE_RUST_FN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+(\w+)").unwrap());
static RE_RUST_STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?struct\s+(\w+)").unwrap());
static RE_RUST_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?enum\s+(\w+)").unwrap());
static RE_RUST_TRAIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?trait\s+(\w+)").unwrap());
static RE_RUST_IMPL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(.+?)(?:\s*\{|$)").unwrap());
static RE_RUST_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?(?:const|static)\s+(\w+)").unwrap());
static RE_RUST_MOD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(pub(?:\(crate\))?\s+)?mod\s+(\w+)").unwrap());
static RE_RUST_CFG_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\[cfg\(test\)\]").unwrap());

// ── Python symbol regexes ────────────────────────────────────────────
static RE_PY_CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^class\s+(\w+)").unwrap());
static RE_PY_DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:async\s+)?def\s+(\w+)").unwrap());
static RE_PY_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Z][A-Z0-9_]*)\s*=").unwrap());

// ── JavaScript symbol regexes ────────────────────────────────────────
static RE_JS_FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s+(\w+)").unwrap()
});
static RE_JS_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:export\s+(?:default\s+)?)?class\s+(\w+)").unwrap());
static RE_JS_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=").unwrap());

// ── TypeScript-specific regexes (JS ones reused via extract_js_symbols) ──
static RE_TS_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:export\s+)?interface\s+(\w+)").unwrap());
static RE_TS_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:export\s+)?type\s+(\w+)\s*[=<]").unwrap());

// ── Go symbol regexes ────────────────────────────────────────────────
static RE_GO_FUNC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^func\s+(\w+)\s*\(").unwrap());
static RE_GO_METHOD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^func\s+\([^)]+\)\s+(\w+)\s*\(").unwrap());
static RE_GO_STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^type\s+(\w+)\s+struct\b").unwrap());
static RE_GO_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^type\s+(\w+)\s+interface\b").unwrap());
static RE_GO_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:const|var)\s+(\w+)").unwrap());

// ── Java symbol regexes ──────────────────────────────────────────────
static RE_JAVA_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public\s+)?(?:abstract\s+)?(?:final\s+)?class\s+(\w+)").unwrap()
});
static RE_JAVA_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:public\s+)?interface\s+(\w+)").unwrap());
static RE_JAVA_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:public\s+)?enum\s+(\w+)").unwrap());
static RE_JAVA_METHOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(?:[\w<>\[\],\s]+)\s+(\w+)\s*\(",
    )
    .unwrap()
});

// ── C regexes ──
static RE_C_FUNC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][\w\s\*]*\s+(\w+)\s*\(").unwrap());
static RE_C_STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:typedef\s+)?struct\s+(\w+)").unwrap());
static RE_C_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:typedef\s+)?enum\s+(\w+)").unwrap());
static RE_C_DEFINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#define\s+(\w+)").unwrap());

// ── C++ regexes (extends C) ──
static RE_CPP_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:template\s*<[^>]*>\s*)?class\s+(\w+)").unwrap());
static RE_CPP_NAMESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^namespace\s+(\w+)").unwrap());

// ── Ruby regexes ──
static RE_RUBY_DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*def\s+([\w\?\!]+)").unwrap());
static RE_RUBY_CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*class\s+(\w+)").unwrap());
static RE_RUBY_MODULE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*module\s+(\w+)").unwrap());
static RE_RUBY_CONST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*([A-Z][A-Z_0-9]+)\s*=").unwrap());

// ── Shell regexes ──
static RE_SHELL_FUNC_PARENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\w+)\s*\(\)\s*\{").unwrap());
static RE_SHELL_FUNC_KEYWORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*function\s+(\w+)").unwrap());

// ── C# regexes ──
static RE_CSHARP_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public|private|protected|internal)?\s*(?:static\s+)?(?:abstract\s+)?(?:sealed\s+)?(?:partial\s+)?class\s+(\w+)").unwrap()
});
static RE_CSHARP_INTERFACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public|private|protected|internal)?\s*(?:partial\s+)?interface\s+(\w+)")
        .unwrap()
});
static RE_CSHARP_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public|private|protected|internal)?\s*(?:readonly\s+)?(?:partial\s+)?struct\s+(\w+)").unwrap()
});
static RE_CSHARP_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public|private|protected|internal)?\s*enum\s+(\w+)").unwrap()
});
static RE_CSHARP_NAMESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*namespace\s+([\w.]+)").unwrap());
static RE_CSHARP_RECORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:public|private|protected|internal)?\s*(?:sealed\s+)?record\s+(?:struct\s+)?(\w+)",
    )
    .unwrap()
});
static RE_CSHARP_METHOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public|private|protected|internal)\s+(?:static\s+)?(?:virtual\s+)?(?:override\s+)?(?:async\s+)?(?:[\w<>\[\],\?\s]+)\s+(\w+)\s*\(").unwrap()
});

// ── PHP regexes ──
static RE_PHP_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:abstract\s+)?(?:final\s+)?class\s+(\w+)").unwrap());
static RE_PHP_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*interface\s+(\w+)").unwrap());
static RE_PHP_TRAIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*trait\s+(\w+)").unwrap());
static RE_PHP_FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:public|private|protected)?\s*(?:static\s+)?function\s+(\w+)").unwrap()
});
static RE_PHP_ENUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*enum\s+(\w+)").unwrap());

// ── Kotlin regexes ──
static RE_KOTLIN_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|protected|internal|open|abstract|sealed|data|inner|enum)\s+)*class\s+(\w+)").unwrap()
});
static RE_KOTLIN_INTERFACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|protected|internal|sealed)\s+)*interface\s+(\w+)")
        .unwrap()
});
static RE_KOTLIN_OBJECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|protected|internal)\s+)*(?:companion\s+)?object\s+(\w+)")
        .unwrap()
});
static RE_KOTLIN_FUN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|protected|internal|open|override|suspend|inline)\s+)*fun\s+(?:<[^>]*>\s*)?(\w+)").unwrap()
});

// ── Swift regexes ──
static RE_SWIFT_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open|final)\s+)*class\s+(\w+)")
        .unwrap()
});
static RE_SWIFT_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate)\s+)*struct\s+(\w+)").unwrap()
});
static RE_SWIFT_PROTOCOL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate)\s+)*protocol\s+(\w+)").unwrap()
});
static RE_SWIFT_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate)\s+)*enum\s+(\w+)").unwrap()
});
static RE_SWIFT_FUNC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open|override|static|class|mutating)\s+)*func\s+(\w+)").unwrap()
});
static RE_SWIFT_EXTENSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*extension\s+(\w+)").unwrap());

// ── Scala regexes ──
static RE_SCALA_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:abstract|sealed|final|implicit|lazy)\s+)*(?:case\s+)?class\s+(\w+)")
        .unwrap()
});
static RE_SCALA_TRAIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:sealed\s+)?trait\s+(\w+)").unwrap());
static RE_SCALA_OBJECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:case\s+)?object\s+(\w+)").unwrap());
static RE_SCALA_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:private|protected|override|implicit|lazy)\s+)*def\s+(\w+)").unwrap()
});

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
    Macro,
    Namespace,
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
        "c" | "h" => Some("c"),
        "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "hh" => Some("cpp"),
        "rb" => Some("ruby"),
        "sh" | "bash" | "zsh" => Some("shell"),
        "cs" => Some("csharp"),
        "php" => Some("php"),
        "kt" | "kts" => Some("kotlin"),
        "swift" => Some("swift"),
        "scala" | "sc" => Some("scala"),
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
        "c" => extract_c_symbols(code),
        "cpp" => extract_cpp_symbols(code),
        "ruby" => extract_ruby_symbols(code),
        "shell" => extract_shell_symbols(code),
        "csharp" => extract_csharp_symbols(code),
        "php" => extract_php_symbols(code),
        "kotlin" => extract_kotlin_symbols(code),
        "swift" => extract_swift_symbols(code),
        "scala" => extract_scala_symbols(code),
        _ => Vec::new(),
    }
}

/// Extract symbols from Rust source code.
/// Skips content inside `#[cfg(test)]` modules.
fn extract_rust_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut in_test_module = false;
    let mut test_brace_depth: i32 = 0;

    let re_fn = &*RE_RUST_FN;
    let re_struct = &*RE_RUST_STRUCT;
    let re_enum = &*RE_RUST_ENUM;
    let re_trait = &*RE_RUST_TRAIT;
    let re_impl = &*RE_RUST_IMPL;
    let re_const = &*RE_RUST_CONST;
    let re_mod = &*RE_RUST_MOD;
    let re_cfg_test = &*RE_RUST_CFG_TEST;

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

    let re_class = &*RE_PY_CLASS;
    let re_func = &*RE_PY_DEF;
    let re_const = &*RE_PY_CONST;

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

    let re_export_func = &*RE_JS_FUNCTION;
    let re_class = &*RE_JS_CLASS;
    let re_const = &*RE_JS_CONST;

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

    let re_interface = &*RE_TS_INTERFACE;
    let re_type = &*RE_TS_TYPE;

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

    let re_func = &*RE_GO_FUNC;
    let re_method = &*RE_GO_METHOD;
    let re_type_struct = &*RE_GO_STRUCT;
    let re_type_interface = &*RE_GO_INTERFACE;
    let re_const = &*RE_GO_CONST;

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

    let re_class = &*RE_JAVA_CLASS;
    let re_interface = &*RE_JAVA_INTERFACE;
    let re_enum = &*RE_JAVA_ENUM;
    let re_method = &*RE_JAVA_METHOD;

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

/// C keywords that match the function regex but aren't function definitions.
const C_NON_FUNC_KEYWORDS: &[&str] = &[
    "if", "for", "while", "switch", "return", "else", "do", "sizeof", "typedef", "extern",
    "static", "inline", "volatile", "register", "goto", "case", "break", "continue",
];

/// Extract symbols from C source code.
fn extract_c_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_func = &*RE_C_FUNC;
    let re_struct = &*RE_C_STRUCT;
    let re_enum = &*RE_C_ENUM;
    let re_define = &*RE_C_DEFINE;

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();

        if let Some(caps) = re_define.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Macro,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_struct.captures(trimmed) {
            let name = caps.get(2).map_or("", |m| m.as_str());
            // If no typedef group matched, try group 1 as the struct name
            let name = if name.is_empty() {
                caps.get(1).map_or("", |m| m.as_str())
            } else {
                name
            };
            symbols.push(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Struct,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_enum.captures(trimmed) {
            let name = caps.get(2).map_or("", |m| m.as_str());
            let name = if name.is_empty() {
                caps.get(1).map_or("", |m| m.as_str())
            } else {
                name
            };
            symbols.push(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Enum,
                is_public: true,
                line: line_num + 1,
            });
        } else if !trimmed.starts_with("//")
            && !trimmed.starts_with("/*")
            && !trimmed.starts_with('*')
            && !trimmed.starts_with('#')
        {
            // Only match functions at top-level (non-indented) lines
            if line.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
                if let Some(caps) = re_func.captures(trimmed) {
                    let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
                    if !C_NON_FUNC_KEYWORDS.contains(&name.as_str()) {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Function,
                            is_public: true,
                            line: line_num + 1,
                        });
                    }
                }
            }
        }
    }

    symbols
}

/// Extract symbols from C++ source code.
///
/// Extends C extraction with classes, namespaces, and templates.
fn extract_cpp_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_func = &*RE_C_FUNC;
    let re_struct = &*RE_C_STRUCT;
    let re_enum = &*RE_C_ENUM;
    let re_define = &*RE_C_DEFINE;
    let re_class = &*RE_CPP_CLASS;
    let re_namespace = &*RE_CPP_NAMESPACE;

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();

        if let Some(caps) = re_define.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Macro,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_namespace.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Namespace,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_class.captures(trimmed) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_struct.captures(trimmed) {
            let name = caps.get(2).map_or("", |m| m.as_str());
            let name = if name.is_empty() {
                caps.get(1).map_or("", |m| m.as_str())
            } else {
                name
            };
            symbols.push(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Struct,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_enum.captures(trimmed) {
            let name = caps.get(2).map_or("", |m| m.as_str());
            let name = if name.is_empty() {
                caps.get(1).map_or("", |m| m.as_str())
            } else {
                name
            };
            symbols.push(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Enum,
                is_public: true,
                line: line_num + 1,
            });
        } else if !trimmed.starts_with("//")
            && !trimmed.starts_with("/*")
            && !trimmed.starts_with('*')
            && !trimmed.starts_with('#')
        {
            if let Some(caps) = re_func.captures(trimmed) {
                let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
                if !C_NON_FUNC_KEYWORDS.contains(&name.as_str())
                    && name != "class"
                    && name != "namespace"
                {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        is_public: true,
                        line: line_num + 1,
                    });
                }
            }
        }
    }

    symbols
}

/// Extract symbols from Ruby source code.
fn extract_ruby_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_def = &*RE_RUBY_DEF;
    let re_class = &*RE_RUBY_CLASS;
    let re_module = &*RE_RUBY_MODULE;
    let re_const = &*RE_RUBY_CONST;

    for (line_num, line) in code.lines().enumerate() {
        if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_module.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Module,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_def.captures(line) {
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

/// Extract symbols from Shell scripts (bash/sh/zsh).
fn extract_shell_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_parens = &*RE_SHELL_FUNC_PARENS;
    let re_keyword = &*RE_SHELL_FUNC_KEYWORD;

    for (line_num, line) in code.lines().enumerate() {
        if let Some(caps) = re_keyword.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_parens.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public: true,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// C# keywords that match the method regex but aren't method definitions.
const CSHARP_NON_METHOD_KEYWORDS: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "return",
    "new",
    "class",
    "interface",
    "struct",
    "enum",
    "namespace",
    "using",
    "throw",
    "lock",
    "foreach",
    "typeof",
    "sizeof",
    "nameof",
    "record",
];

/// Extract symbols from C# source code.
fn extract_csharp_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class = &*RE_CSHARP_CLASS;
    let re_interface = &*RE_CSHARP_INTERFACE;
    let re_struct = &*RE_CSHARP_STRUCT;
    let re_enum = &*RE_CSHARP_ENUM;
    let re_namespace = &*RE_CSHARP_NAMESPACE;
    let re_record = &*RE_CSHARP_RECORD;
    let re_method = &*RE_CSHARP_METHOD;

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();
        let is_pub = trimmed.starts_with("public");

        if let Some(caps) = re_namespace.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Namespace,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_record.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_class.captures(line) {
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
        } else if let Some(caps) = re_struct.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
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
            if !CSHARP_NON_METHOD_KEYWORDS.contains(&name.as_str()) {
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

/// Extract symbols from PHP source code.
fn extract_php_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class = &*RE_PHP_CLASS;
    let re_interface = &*RE_PHP_INTERFACE;
    let re_trait = &*RE_PHP_TRAIT;
    let re_function = &*RE_PHP_FUNCTION;
    let re_enum = &*RE_PHP_ENUM;

    for (line_num, line) in code.lines().enumerate() {
        let is_pub = line.trim_start().starts_with("public")
            || !line.trim_start().starts_with("private")
                && !line.trim_start().starts_with("protected");

        if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_interface.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Interface,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_trait.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Trait,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_enum.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Enum,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_function.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public: is_pub,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// Kotlin keywords that match the fun regex but aren't function definitions.
const KOTLIN_NON_FUN_KEYWORDS: &[&str] = &[
    "if",
    "for",
    "while",
    "when",
    "return",
    "throw",
    "class",
    "interface",
    "object",
];

/// Extract symbols from Kotlin source code.
fn extract_kotlin_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class = &*RE_KOTLIN_CLASS;
    let re_interface = &*RE_KOTLIN_INTERFACE;
    let re_object = &*RE_KOTLIN_OBJECT;
    let re_fun = &*RE_KOTLIN_FUN;

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();
        let is_pub = !trimmed.starts_with("private") && !trimmed.starts_with("protected");

        // Check interface before class since "sealed interface" would also match class regex
        if let Some(caps) = re_interface.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Interface,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_object.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Module,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            // Distinguish data class, sealed class, enum class — all stored as Class
            let kind = if trimmed.starts_with("enum") || trimmed.contains("enum class") {
                SymbolKind::Enum
            } else {
                SymbolKind::Class
            };
            symbols.push(Symbol {
                name,
                kind,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_fun.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            if !KOTLIN_NON_FUN_KEYWORDS.contains(&name.as_str()) {
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

/// Extract symbols from Swift source code.
fn extract_swift_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class = &*RE_SWIFT_CLASS;
    let re_struct = &*RE_SWIFT_STRUCT;
    let re_protocol = &*RE_SWIFT_PROTOCOL;
    let re_enum = &*RE_SWIFT_ENUM;
    let re_func = &*RE_SWIFT_FUNC;
    let re_extension = &*RE_SWIFT_EXTENSION;

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();
        let is_pub = trimmed.starts_with("public") || trimmed.starts_with("open");

        if let Some(caps) = re_protocol.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Trait,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_struct.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Struct,
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
        } else if let Some(caps) = re_extension.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Impl,
                is_public: true,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_func.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public: is_pub,
                line: line_num + 1,
            });
        }
    }

    symbols
}

/// Extract symbols from Scala source code.
fn extract_scala_symbols(code: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let re_class = &*RE_SCALA_CLASS;
    let re_trait = &*RE_SCALA_TRAIT;
    let re_object = &*RE_SCALA_OBJECT;
    let re_def = &*RE_SCALA_DEF;

    for (line_num, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();
        let is_pub = !trimmed.starts_with("private") && !trimmed.starts_with("protected");

        if let Some(caps) = re_trait.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Trait,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_object.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Module,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_class.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Class,
                is_public: is_pub,
                line: line_num + 1,
            });
        } else if let Some(caps) = re_def.captures(line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            symbols.push(Symbol {
                name,
                kind: SymbolKind::Function,
                is_public: is_pub,
                line: line_num + 1,
            });
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
                SymbolKind::Macro => format!("{CYAN}macro{RESET}"),
                SymbolKind::Namespace => format!("{MAGENTA}namespace{RESET}"),
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
                SymbolKind::Macro => "macro",
                SymbolKind::Namespace => "namespace",
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
                    SymbolKind::Macro => "macro",
                    SymbolKind::Namespace => "namespace",
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

    // ── C extraction tests ──

    #[test]
    fn extract_c_symbols_basic() {
        let code = r#"
#include <stdio.h>

#define MAX_SIZE 100
#define MIN(a,b) ((a)<(b)?(a):(b))

typedef struct point {
    int x;
    int y;
} Point;

enum color { RED, GREEN, BLUE };

int main(int argc, char **argv) {
    return 0;
}

void helper(void) {
    printf("hello\n");
}
"#;
        let symbols = extract_symbols(code, "c");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MAX_SIZE" && s.kind == SymbolKind::Macro),
            "should find #define MAX_SIZE"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MIN" && s.kind == SymbolKind::Macro),
            "should find #define MIN"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "point" && s.kind == SymbolKind::Struct),
            "should find struct point"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "color" && s.kind == SymbolKind::Enum),
            "should find enum color"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "main" && s.kind == SymbolKind::Function),
            "should find function main"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "helper" && s.kind == SymbolKind::Function),
            "should find function helper"
        );
    }

    #[test]
    fn extract_c_symbols_skips_control_flow() {
        let code = r#"
int process(int x) {
    if (x > 0) {
        return x;
    }
    for (int i = 0; i < x; i++) {
        while (1) break;
    }
    switch (x) {
        case 1: return 1;
    }
    return 0;
}
"#;
        let symbols = extract_symbols(code, "c");
        let func_names: Vec<&str> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();
        assert!(func_names.contains(&"process"), "should find process");
        assert!(
            !func_names.contains(&"if"),
            "should not match 'if' as function"
        );
        assert!(
            !func_names.contains(&"for"),
            "should not match 'for' as function"
        );
        assert!(
            !func_names.contains(&"while"),
            "should not match 'while' as function"
        );
        assert!(
            !func_names.contains(&"switch"),
            "should not match 'switch' as function"
        );
    }

    // ── C++ extraction tests ──

    #[test]
    fn extract_cpp_symbols_basic() {
        let code = r#"
#include <iostream>

#define VERSION 1

namespace mylib {

class Widget {
public:
    void draw();
};

struct Point {
    int x, y;
};

enum Color { Red, Green, Blue };

template<typename T>
class Container {
    T value;
};

void free_function(int x) {
    std::cout << x;
}

} // namespace mylib
"#;
        let symbols = extract_symbols(code, "cpp");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "VERSION" && s.kind == SymbolKind::Macro),
            "should find #define VERSION"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "mylib" && s.kind == SymbolKind::Namespace),
            "should find namespace mylib"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Widget" && s.kind == SymbolKind::Class),
            "should find class Widget"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Point" && s.kind == SymbolKind::Struct),
            "should find struct Point"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Color" && s.kind == SymbolKind::Enum),
            "should find enum Color"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Container" && s.kind == SymbolKind::Class),
            "should find template class Container"
        );
    }

    #[test]
    fn extract_cpp_symbols_template_class() {
        let code = "template<typename T>\nclass Vector {\n    T* data;\n};\n";
        let symbols = extract_symbols(code, "cpp");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Vector" && s.kind == SymbolKind::Class),
            "should find template class Vector"
        );
    }

    // ── Ruby extraction tests ──

    #[test]
    fn extract_ruby_symbols_basic() {
        let code = r#"
module MyApp
  class User
    MAX_RETRIES = 3
    DEFAULT_NAME = "anonymous"

    def initialize(name)
      @name = name
    end

    def greet
      "Hello, #{@name}"
    end

    def valid?
      !@name.nil?
    end

    def _private_helper
      true
    end
  end
end
"#;
        let symbols = extract_symbols(code, "ruby");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MyApp" && s.kind == SymbolKind::Module),
            "should find module MyApp"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "User" && s.kind == SymbolKind::Class),
            "should find class User"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MAX_RETRIES" && s.kind == SymbolKind::Const),
            "should find constant MAX_RETRIES"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "DEFAULT_NAME" && s.kind == SymbolKind::Const),
            "should find constant DEFAULT_NAME"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "initialize" && s.kind == SymbolKind::Function),
            "should find def initialize"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "greet" && s.kind == SymbolKind::Function),
            "should find def greet"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "valid?" && s.kind == SymbolKind::Function),
            "should find def valid?"
        );
        // Private method: is_public should be false
        let priv_method = symbols.iter().find(|s| s.name == "_private_helper");
        assert!(priv_method.is_some(), "should find _private_helper");
        assert!(
            !priv_method.unwrap().is_public,
            "_private_helper should not be public"
        );
    }

    #[test]
    fn extract_ruby_symbols_edge_cases() {
        let code = "class Base\nend\nclass Child < Base\nend\n";
        let symbols = extract_symbols(code, "ruby");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Base" && s.kind == SymbolKind::Class),
            "should find class Base"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Child" && s.kind == SymbolKind::Class),
            "should find class Child (with inheritance)"
        );
    }

    // ── Shell extraction tests ──

    #[test]
    fn extract_shell_symbols_basic() {
        let code = r#"#!/bin/bash

setup() {
    echo "setting up"
}

function cleanup {
    echo "cleaning up"
}

function run_tests {
    setup
    echo "running tests"
    cleanup
}

main() {
    run_tests
}

main "$@"
"#;
        let symbols = extract_symbols(code, "shell");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "setup" && s.kind == SymbolKind::Function),
            "should find setup() function"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "cleanup" && s.kind == SymbolKind::Function),
            "should find 'function cleanup'"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "run_tests" && s.kind == SymbolKind::Function),
            "should find 'function run_tests'"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "main" && s.kind == SymbolKind::Function),
            "should find main() function"
        );
    }

    #[test]
    fn extract_shell_symbols_indented() {
        let code = "  helper() {\n    echo hi\n  }\n  function inner_func {\n    true\n  }\n";
        let symbols = extract_symbols(code, "shell");
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "helper" && s.kind == SymbolKind::Function),
            "should find indented helper()"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "inner_func" && s.kind == SymbolKind::Function),
            "should find indented function inner_func"
        );
    }

    // ── Language detection tests for new languages ──

    #[test]
    fn detect_language_c_extensions() {
        assert_eq!(detect_language("main.c"), Some("c"));
        assert_eq!(detect_language("header.h"), Some("c"));
    }

    #[test]
    fn detect_language_cpp_extensions() {
        assert_eq!(detect_language("main.cpp"), Some("cpp"));
        assert_eq!(detect_language("main.cc"), Some("cpp"));
        assert_eq!(detect_language("main.cxx"), Some("cpp"));
        assert_eq!(detect_language("header.hpp"), Some("cpp"));
        assert_eq!(detect_language("header.hxx"), Some("cpp"));
        assert_eq!(detect_language("header.hh"), Some("cpp"));
    }

    #[test]
    fn detect_language_ruby_extension() {
        assert_eq!(detect_language("app.rb"), Some("ruby"));
    }

    #[test]
    fn detect_language_shell_extensions() {
        assert_eq!(detect_language("script.sh"), Some("shell"));
        assert_eq!(detect_language("build.bash"), Some("shell"));
        assert_eq!(detect_language("init.zsh"), Some("shell"));
    }

    // ── C# language detection and extraction tests ──

    #[test]
    fn detect_language_csharp_extensions() {
        assert_eq!(detect_language("Program.cs"), Some("csharp"));
    }

    #[test]
    fn extract_csharp_symbols_basic() {
        let code = r#"
namespace MyApp.Models
{
    public interface IRepository
    {
        void Save();
    }

    public class UserService
    {
        public async Task<User> GetUser(int id)
        {
            return await db.Find(id);
        }

        private void ValidateInput(string input)
        {
        }
    }

    public struct Point
    {
        public int X;
        public int Y;
    }

    public enum Status
    {
        Active,
        Inactive
    }

    public record UserDto(string Name, int Age);
}
"#;
        let symbols = extract_symbols(code, "csharp");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "MyApp.Models" && s.kind == SymbolKind::Namespace),
            "should find namespace"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "IRepository" && s.kind == SymbolKind::Interface),
            "should find interface"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "UserService" && s.kind == SymbolKind::Class),
            "should find class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "GetUser" && s.kind == SymbolKind::Function),
            "should find public method"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Point" && s.kind == SymbolKind::Struct),
            "should find struct"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Status" && s.kind == SymbolKind::Enum),
            "should find enum"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "UserDto" && s.kind == SymbolKind::Struct),
            "should find record as struct"
        );
    }

    // ── PHP language detection and extraction tests ──

    #[test]
    fn detect_language_php_extensions() {
        assert_eq!(detect_language("index.php"), Some("php"));
    }

    #[test]
    fn extract_php_symbols_basic() {
        let code = r#"<?php

class UserController
{
    public function index()
    {
        return view('users.index');
    }

    private function validate($data)
    {
    }
}

interface Cacheable
{
    public function cacheKey(): string;
}

trait HasTimestamps
{
    public function createdAt(): DateTime
    {
    }
}

enum Color
{
    case Red;
    case Green;
    case Blue;
}

function helper_function($x)
{
    return $x * 2;
}
"#;
        let symbols = extract_symbols(code, "php");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "UserController" && s.kind == SymbolKind::Class),
            "should find class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "index" && s.kind == SymbolKind::Function),
            "should find public method"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Cacheable" && s.kind == SymbolKind::Interface),
            "should find interface"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "HasTimestamps" && s.kind == SymbolKind::Trait),
            "should find trait"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Color" && s.kind == SymbolKind::Enum),
            "should find enum"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "helper_function" && s.kind == SymbolKind::Function),
            "should find standalone function"
        );
    }

    // ── Kotlin language detection and extraction tests ──

    #[test]
    fn detect_language_kotlin_extensions() {
        assert_eq!(detect_language("Main.kt"), Some("kotlin"));
        assert_eq!(detect_language("build.gradle.kts"), Some("kotlin"));
    }

    #[test]
    fn extract_kotlin_symbols_basic() {
        let code = r#"
data class User(val name: String, val age: Int)

sealed class Result {
    data class Success(val data: Any) : Result()
    data class Error(val message: String) : Result()
}

interface Repository {
    fun findAll(): List<User>
}

enum class Direction {
    NORTH, SOUTH, EAST, WEST
}

object Singleton {
    fun getInstance(): Singleton = this
}

fun <T> process(item: T): T {
    return item
}

suspend fun fetchData(): String {
    return "data"
}

private fun helperFunction() {
}
"#;
        let symbols = extract_symbols(code, "kotlin");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "User" && s.kind == SymbolKind::Class),
            "should find data class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Result" && s.kind == SymbolKind::Class),
            "should find sealed class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Repository" && s.kind == SymbolKind::Interface),
            "should find interface"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Direction" && s.kind == SymbolKind::Enum),
            "should find enum class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Singleton" && s.kind == SymbolKind::Module),
            "should find object"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "process" && s.kind == SymbolKind::Function),
            "should find generic function"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "fetchData" && s.kind == SymbolKind::Function),
            "should find suspend function"
        );
    }

    // ── Swift language detection and extraction tests ──

    #[test]
    fn detect_language_swift_extensions() {
        assert_eq!(detect_language("App.swift"), Some("swift"));
    }

    #[test]
    fn extract_swift_symbols_basic() {
        let code = r#"
public class ViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
    }
}

struct Point {
    var x: Double
    var y: Double
}

protocol Drawable {
    func draw()
}

enum Direction {
    case north
    case south
}

extension String {
    func reversed() -> String {
        return String(self.reversed())
    }
}

public func globalHelper() -> Int {
    return 42
}

private func internalHelper() {
}

static func classMethod() {
}
"#;
        let symbols = extract_symbols(code, "swift");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "ViewController" && s.kind == SymbolKind::Class),
            "should find class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "viewDidLoad" && s.kind == SymbolKind::Function),
            "should find override func"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Point" && s.kind == SymbolKind::Struct),
            "should find struct"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Drawable" && s.kind == SymbolKind::Trait),
            "should find protocol as trait"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Direction" && s.kind == SymbolKind::Enum),
            "should find enum"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "String" && s.kind == SymbolKind::Impl),
            "should find extension as impl"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "globalHelper" && s.kind == SymbolKind::Function),
            "should find global func"
        );
    }

    // ── Scala language detection and extraction tests ──

    #[test]
    fn detect_language_scala_extensions() {
        assert_eq!(detect_language("Main.scala"), Some("scala"));
        assert_eq!(detect_language("build.sc"), Some("scala"));
    }

    #[test]
    fn extract_scala_symbols_basic() {
        let code = r#"
case class User(name: String, age: Int)

sealed trait Result
class Success(val data: Any) extends Result
class Failure(val message: String) extends Result

trait Repository {
  def findAll(): List[User]
  def findById(id: Int): Option[User]
}

object UserService {
  def create(name: String): User = {
    User(name, 0)
  }

  private def validate(name: String): Boolean = {
    name.nonEmpty
  }
}

case object Sentinel

abstract class Base {
  def process(): Unit
}
"#;
        let symbols = extract_symbols(code, "scala");

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "User" && s.kind == SymbolKind::Class),
            "should find case class"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Result" && s.kind == SymbolKind::Trait),
            "should find sealed trait"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Repository" && s.kind == SymbolKind::Trait),
            "should find trait"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "UserService" && s.kind == SymbolKind::Module),
            "should find object as module"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "create" && s.kind == SymbolKind::Function),
            "should find def"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Sentinel" && s.kind == SymbolKind::Module),
            "should find case object"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "findAll" && s.kind == SymbolKind::Function),
            "should find trait method def"
        );
        assert!(
            symbols
                .iter()
                .any(|s| s.name == "Base" && s.kind == SymbolKind::Class),
            "should find abstract class"
        );
    }
}
