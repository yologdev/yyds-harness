Title: Extract crashes subcommand handler from commands_state.rs
Files: src/commands_state.rs, src/commands_state_crashes.rs, src/lib.rs
Issue: none
Origin: planner

Objective:
Extract the `/state crashes` handler (handle_crashes, CrashEntry, build_crashes_report, format_relative_ms) from the 23,848-line commands_state.rs into a new commands_state_crashes.rs module. This is the first chunk extracted from the monolith, establishing a pattern for future extractions (evals, patches, lineage, cache handlers).

Why this matters:
`commands_state.rs` is 17% of the codebase (23,848 lines, 580 functions, 151 tests). The journal has noted this for 3+ days. The crashes handler is ~185 lines, self-contained (CrashEntry is only used here, format_relative_ms is only called from build_crashes_report), and has no external test dependencies. Extracting it first establishes the pattern and reduces the monolith by a measurable amount. The assessment identifies it as one of 6 extractable chunks.

Success Criteria:
- `cargo build` and `cargo test` pass with all existing tests
- `cargo test --lib -- commands_state` passes (all 151 tests in the module)
- `yyds state crashes --limit 5` produces identical output before and after
- The new `commands_state_crashes.rs` contains handle_crashes, CrashEntry, build_crashes_report, and format_relative_ms
- commands_state.rs is reduced by ~185 lines

Verification:
- `cargo build`
- `cargo test --lib -- commands_state`  (all existing tests must pass)
- `cargo test --test integration`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings 2>&1 | head -20`

Expected Evidence:
- `yyds state crashes --limit 5` produces identical output (human-readable and JSON modes)
- No compile errors from moved symbol visibility
- commands_state.rs line count decreases by ~185
- New file `src/commands_state_crashes.rs` added to git tracking

Description:

### Step 1: Make shared utilities pub(crate)

In `src/commands_state.rs`, these functions are used by the crashes handler and need pub(crate) visibility for extraction:

- `flag_value` — used for parsing `--limit` and `--json` flags
- `default_events_path` — used to find the state events file
- `read_events` — used to load events from JSONL
- `event_string` — used to extract string fields from event JSON
- `event_timestamp_ms` — used to get timestamps from events
- `format_timestamp_ms` — used to format timestamps in display output

Find each function definition and add `pub(crate)` before `fn`. These are currently private (no visibility modifier). Use search to locate exact definitions:
```
grep -n "^fn flag_value\|^fn default_events_path\|^fn read_events\|^fn event_string\|^fn event_timestamp_ms\|^fn format_timestamp_ms" src/commands_state.rs
```

### Step 2: Create src/commands_state_crashes.rs

Create the new file with:
```rust
//! /state crashes subcommand — detect and report crashed sessions.
//! Extracted from commands_state.rs.

use crate::format::*;
use crate::state::{read_events, event_string, event_timestamp_ms, format_timestamp_ms};
use serde_json::Value;
use std::path::Path;

// ... (move handle_crashes, CrashEntry, build_crashes_report, format_relative_ms here)
```

The exact content to move from commands_state.rs:
- `fn handle_crashes(args: &[String])` (lines 742-756)
- `struct CrashEntry` (lines 758-765)
- `fn build_crashes_report(...)` (lines 767-927)
- `fn format_relative_ms(diff_ms: i64)` (lines 929-938+)

The handler uses: `flag_value`, `default_events_path`, `read_events`, `event_string`, `event_timestamp_ms`, `format_timestamp_ms`, `YELLOW`, `RESET`, `CrashEntry`, `format_relative_ms`.

### Step 3: Add pub(crate) re-export in commands_state.rs

After removing the crashes code from commands_state.rs, add at the top of the dispatch match:
```rust
"crashes" => crate::commands_state_crashes::handle_crashes(&args[3..]),
```

This replaces the current line 70:
```rust
"crashes" => handle_crashes(&args[3..]),
```

### Step 4: Register module in lib.rs

Add to `src/lib.rs` in the module declarations section:
```rust
mod commands_state_crashes;
```

Find the existing `mod commands_state;` declaration and add the new one nearby (alphabetical order would place it right after).

### Step 5: Verify

Run the full test suite. Specifically verify that:
- The crashes handler still works: `yyds state crashes --limit 5`
- JSON output still works: `yyds state crashes --limit 5 --json`
- All existing commands_state tests pass

### Risk notes

If making functions pub(crate) causes compilation issues (e.g., with re-exports or name conflicts), fall back to keeping them private and using `pub(super)` instead. The key insight is that the crashes handler and its types are self-contained and don't depend on other private types in commands_state.rs.

If any test in commands_state.rs specifically tests the crashes handler by name, those tests should move to the new module. Search for test functions referencing "crashes", "CrashEntry", or "build_crashes_report" and move them:
```
grep -n "crashes\|CrashEntry\|build_crashes_report" src/commands_state.rs | grep "fn test\|#\[test\]"
```
