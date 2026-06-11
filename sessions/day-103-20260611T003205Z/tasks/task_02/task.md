Title: Extract diagnostics sub-handlers from commands_state.rs into new module
Files: src/commands_state.rs, src/commands_state_diagnostics.rs (new)
Issue: none
Origin: planner

Objective:
Extract the failure, crash, and diagnostics sub-handlers from the 23,848-line
commands_state.rs into a new src/commands_state_diagnostics.rs module. This is the
first step of splitting the largest file in the codebase (16.5% of all source).

Why this matters:
commands_state.rs is a single-file switchboard of ~200 sub-handlers at 23,848 lines.
It's been flagged as structural debt for multiple sessions (Days 100, 101, 102)
without action. Every future change to the state system has to navigate this file.
Extracting the diagnostics cluster first is natural because it's a self-contained
concern (failures, crashes, rollbacks) with clear boundaries around lines 723-941.

The evolution policy says large refactors must be broken into one-module-at-a-time
tasks. This is the first extraction — small, independently verifiable, and sets the
pattern for future extractions.

Success Criteria:
- A new file src/commands_state_diagnostics.rs exists with failure/crash/rollback handlers
- The extracted code is identical (no refactoring, just move + re-export)
- commands_state.rs is shorter than before
- cargo build && cargo test pass with zero regressions
- handle_state_subcommand still dispatches all subcommands correctly

Verification:
- cargo build --lib
- cargo test --lib -- commands_state
- cargo test --lib -- state
- cargo test (full suite)
- yyds state failures --help (smoke test the moved commands)

Expected Evidence:
- commands_state.rs line count decreases (target: ~200 lines less)
- State events show commands_state_diagnostics.rs was created
- All state subcommands continue to work

---

## What to do

### 1. Identify the diagnostics cluster

The diagnostics-related handlers and builders live around lines 723-941 of
src/commands_state.rs:

```
fn handle_failures(args: &[String])           (~line 723)
fn handle_crashes(args: &[String])            (~line 742)
fn build_crashes_report(...)                  (~line 767)
fn handle_cache(args: &[String])              (~line 942) — NOT diagnostics
```

Also related (from the rollback/policy area):
```
fn handle_rollbacks(args: &[String])          (~line 690)
fn build_rollback_rows(...)                   (~line 2169)
fn build_rollback_report(...)                 (~line 2262)
fn build_rollback_payload(...)                (~line 2314)
fn build_failure_fix_report(...)              (~line 2026)
fn build_recent_failure_report(...)           (~line 1789)
```

Wait — some of these are far apart. This is the challenge with a 23K-line file.
The handlers are interleaved with everything else.

### 2. Strategy: Start small

For this first extraction, only move the **tight cluster** that handles
failures and crashes. This is the cleanest boundary:

Move these functions to `src/commands_state_diagnostics.rs`:
- `handle_failures(args: &[String])`
- `handle_crashes(args: &[String])`
- `build_crashes_report(...)`
- Any private helper functions used ONLY by the above
- Any structs/types used ONLY by the above (e.g., crash report types)

### 3. Create src/commands_state_diagnostics.rs

```rust
//! State diagnostics: failure and crash reporting sub-handlers.
//!
//! Extracted from commands_state.rs to reduce file size.
//! These handlers are dispatched from handle_state_subcommand.

use serde_json::Value;
// ... other imports copied from commands_state.rs

pub fn handle_failures(args: &[String]) { ... }
pub fn handle_crashes(args: &[String]) { ... }
pub fn build_crashes_report(...) -> ... { ... }
// ... any private helpers
```

### 4. Update lib.rs

Add the new module declaration in `src/lib.rs`:
```rust
mod commands_state_diagnostics;
```

### 5. Update commands_state.rs

- Remove the moved functions
- Add `use crate::commands_state_diagnostics::{handle_failures, handle_crashes};`
- In `handle_state_subcommand`, the dispatch still calls `handle_failures(args)` etc.
  If they were called as local functions, update to use the module-qualified path.

### 6. Keep it bounded

- Move ONLY the failures + crashes cluster (around lines 723-941)
- Do NOT move rollback handlers or cache handlers
- Do NOT refactor — this is a pure extraction, preserving exact behavior
- If the cluster boundary is unclear, err on the side of moving LESS
- Aim to extract ~200 lines, not more

### 7. If blocked

If the functions are too intertwined (helper functions shared with non-diagnostics
code, types used elsewhere), note in the commit message what prevented extraction
and what the next attempt should target instead.
