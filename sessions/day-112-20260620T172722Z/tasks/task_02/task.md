Title: Harden search tool pattern with -- separator and extended regex error detection
Files: src/tools.rs
Issue: none
Origin: planner

Evidence:
- Graph pressure #5: search_error_count=1 — search/grep errors created avoidable evolution friction.
- `ProjectSearchTool::execute` at line 202-220 calls `build_project_rg_args` or `build_project_grep_args`, which append `pattern` and `path` as final arguments (lines 337-338, 377-378). Neither function inserts a `--` separator before user-supplied arguments. A pattern starting with `-` (e.g., `--help`, `-v`) would be interpreted as a flag by rg/grep.
- The regex error detection at lines 243-257 covers several rg error patterns but misses `"regex syntax error"`, `"regex parse error"`, and `"regex engine"` — common rg error messages that would escape detection.
- Historical evidence: search_error_count=1 in state events; tool-failure reconciliation shows search pattern errors as a recurring minor friction category.

Edit Surface:
- src/tools.rs (build_project_rg_args around line 309-339, build_project_grep_args around line 350-379, regex error detection around line 243-257)

Verifier:
- cargo test --bin yyds -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Fallback:
- If `cargo test` fails or search tests break unexpectedly, revert. Do not touch scripts.

Objective:
Prevent search tool failures from patterns that start with `-` (flag injection) and catch more regex error messages from rg so the agent receives actionable hints instead of raw stderr.

Why this matters:
The search tool is the agent's primary codebase navigation mechanism (1632 invocations in graph hotspots, second only to bash and read_file). When a pattern like `--help` or `-v` is passed literally, rg interprets it as a flag, producing confusing output or silent wrong results. The `--` separator is the standard Unix convention to signal "end of options, everything after is positional." Additionally, regex errors that escape the current detection logic produce raw stderr that the agent may not know how to interpret, wasting turns on retries.

Success Criteria:
- `build_project_rg_args` inserts `--` before the pattern argument.
- `build_project_grep_args` inserts `--` before the pattern argument.
- Regex error detection covers additional rg error message patterns: `"regex syntax"`, `"regex parse"`, `"regex engine"`.
- Existing search tool tests pass.
- A pattern like `--help` or `-v` is treated as a literal search string, not a flag.

Verification:
- `cargo test tools` — ProjectSearchTool tests
- `cargo test --bin yyds -- --test-threads=1`
- `cargo test --test integration -- --test-threads=1`

Expected Evidence:
- search_error_count in future sessions drops to 0 or stays at 1 (no new entries).
- Tool-failure reconciliation shows no search-pattern-escaping failures in new sessions.
- Dashboard tool-name breakdown for search shows zero exit-code-2 errors from flag misinterpretation.

## Implementation Notes

### Step 1: Add `--` separator

In `build_project_rg_args` (line 337), change:
```rust
args.push(pattern.to_string());
args.push(path.to_string());
```
to:
```rust
args.push("--".to_string());
args.push(pattern.to_string());
args.push(path.to_string());
```

In `build_project_grep_args` (line 377), make the same change:
```rust
args.push("--".to_string());
args.push(pattern.to_string());
args.push(path.to_string());
```

The `--` goes BEFORE the pattern, not after. This ensures `pattern` is never interpreted as a flag.

### Step 2: Extend regex error detection

In `ProjectSearchTool::execute` (around line 245-251), extend the regex error detection conditions. Current check:
```rust
let is_regex_error = regex
    && (stderr_lower.contains("unmatched")
        || stderr_lower.contains("invalid")
        || stderr_lower.contains("regex parse")
        || stderr_lower.contains("unclosed")
        || stderr_lower.contains("empty pattern")
        || stderr_lower.contains("repetition"));
```

Add these patterns:
- `"regex syntax"` — rg error: "regex syntax error"
- `"regex engine"` — rg internal: "regex engine error"

Do NOT remove any existing patterns. Only add the two new ones.

Do not modify the hint message — it already says `"try regex=false for literal search, or escape regex metacharacters"` which is the correct advice for all regex errors.

### Testing

The `--` separator is safe because rg and grep both support `--` as end-of-options. It's standard POSIX. If tests assert specific argument ordering, adjust the assertion to include `--` before the pattern.
