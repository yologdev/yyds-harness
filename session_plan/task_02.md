Title: Expand repo map language support — add C, C++, Ruby, and Shell extractors
Files: src/commands_map.rs
Issue: none

## What

The repo map (`/map` and system prompt injection) currently supports 6 languages: Rust, Python, JavaScript, TypeScript, Go, Java. This leaves common languages unsupported — C, C++, Ruby, and Shell are among the most-used languages in real projects. Aider supports 100+ languages via tree-sitter; we close the gap incrementally by adding regex-based extractors for 4 more languages.

## Implementation

In `src/commands_map.rs`:

### 1. Update `detect_language()`
Add these extension mappings:
- `"c" | "h"` → `"c"`
- `"cc" | "cpp" | "cxx" | "hpp" | "hxx" | "hh"` → `"cpp"`
- `"rb"` → `"ruby"`
- `"sh" | "bash" | "zsh"` → `"shell"`

### 2. Update `extract_symbols()` match
Add cases for `"c"`, `"cpp"`, `"ruby"`, `"shell"` that call new extractor functions.

### 3. Add `extract_c_symbols(code: &str) -> Vec<Symbol>`
Regex patterns:
- Functions: lines matching `^[a-zA-Z_][\w\s\*]+\s+(\w+)\s*\(` at the start of line (not indented, not `if/for/while/switch/return`)
- Structs: `^(typedef\s+)?struct\s+(\w+)`
- Enums: `^(typedef\s+)?enum\s+(\w+)`
- Macros/constants: `^#define\s+(\w+)`

### 4. Add `extract_cpp_symbols(code: &str) -> Vec<Symbol>`
Extend C symbols with:
- Classes: `^class\s+(\w+)`
- Namespaces: `^namespace\s+(\w+)`
- Templates: `^template\s*<[^>]*>\s*(class|struct)\s+(\w+)`

### 5. Add `extract_ruby_symbols(code: &str) -> Vec<Symbol>`
- Functions/methods: `^\s*def\s+(\w+[\?\!]?)`
- Classes: `^\s*class\s+(\w+)`
- Modules: `^\s*module\s+(\w+)`
- Constants: `^\s*([A-Z][A-Z_0-9]+)\s*=`

### 6. Add `extract_shell_symbols(code: &str) -> Vec<Symbol>`
- Functions: `^\s*(\w+)\s*\(\)\s*\{` or `^\s*function\s+(\w+)`

### 7. Add tests
For each new language, add at least 2 tests:
- Basic symbol extraction (function, struct/class, constant)
- Edge cases (comments containing function-like patterns should ideally be skipped, but false positives are acceptable for regex-based v1)

## Sizing
This is a single-file change to `commands_map.rs`. Each extractor is ~20-40 lines. Total addition ~200-300 lines including tests. All extractors follow the same pattern as the existing 6.

## Verification
`cargo test commands_map` should pass. Manually verify with `/map` on a repo containing C/Ruby/Shell files (or use test fixtures).
