Title: Expand /map language support — add C#, PHP, Kotlin, Swift, Scala
Files: src/commands_map.rs
Issue: none

## Description

The `/map` command currently supports 10 languages (rust, python, javascript, typescript, go, java, c, cpp, ruby, shell). Aider supports 100+ via tree-sitter. This task adds 5 more commonly-used languages to close the gap incrementally: C#, PHP, Kotlin, Swift, and Scala. Going from 10→15 languages.

These are all top-20 languages by usage (TIOBE, GitHub stats) and have distinctive, regex-parseable symbol patterns.

## Implementation

1. **`detect_language()`** — Add extensions:
   - `"cs"` → `"csharp"`
   - `"php"` → `"php"`
   - `"kt" | "kts"` → `"kotlin"`
   - `"swift"` → `"swift"`
   - `"scala" | "sc"` → `"scala"`

2. **`extract_symbols()`** — Add dispatch arms for each new language.

3. **Add extraction functions** — One per language, using the same regex pattern style as existing extractors:

   **`extract_csharp_symbols`**: Match `class`, `interface`, `struct`, `enum`, `namespace`, `public/private/protected/internal ... method(`, `record`.
   
   **`extract_php_symbols`**: Match `class`, `interface`, `trait`, `function`, `enum`.
   
   **`extract_kotlin_symbols`**: Match `class`, `interface`, `object`, `fun`, `data class`, `sealed class`, `enum class`.
   
   **`extract_swift_symbols`**: Match `class`, `struct`, `protocol`, `enum`, `func`, `extension`.
   
   **`extract_scala_symbols`**: Match `class`, `trait`, `object`, `def`, `case class`.

   Each extractor should follow the existing pattern:
   - Use `lazy_static!` or inline regex compilation (follow existing style — check if file uses `once_cell::sync::Lazy`)
   - Return `Vec<Symbol>` with appropriate `SymbolKind` (Function, Struct, Trait, etc.)
   - Skip test blocks where possible (e.g., `@Test` annotations in Kotlin, `#[test]` equivalent)

4. **Add tests** — For each new language:
   - `detect_language_*_extensions` — verify extension mapping
   - `extract_*_symbols_basic` — verify a small code snippet produces expected symbols

## Verification

```bash
cargo build && cargo test
# Specifically: cargo test -- commands_map --nocapture
```
