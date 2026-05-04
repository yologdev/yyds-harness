Title: Extract architect-mode turn and post-prompt handling from run_repl
Files: src/repl.rs
Issue: none

## Goal

`run_repl` is 985 lines — the single largest function in the codebase. Two large, self-contained blocks can be extracted into helper functions, reducing the main loop by ~230 lines and making it scannable.

## What to extract

### 1. Architect-mode turn handling (~lines 552-676)

Extract the entire `if commands::is_architect_mode() { ... }` block into an async helper function:

```rust
async fn run_architect_turn(
    agent_config: &mut AgentConfig,
    effective_input: &str,
    session_total: &mut Usage,
    session_changes: &SessionChanges,
) -> PromptOutcome {
    // ... the existing architect-mode code:
    // - Build architect agent with strong model
    // - Phase 1: prompt architect (text-only, no tools)
    // - Stream and collect plan text
    // - Show architect cost
    // - Phase 2: build editor agent, feed plan, run with tools
    // - Show editor cost
    // - Return editor outcome (or default if plan was empty)
}
```

### 2. Post-prompt handling (~lines 718-831)

Extract the post-prompt block (everything from `maybe_ring_bell` through `auto_compact_if_needed`) into a helper function:

```rust
async fn handle_post_prompt(
    outcome: &PromptOutcome,
    agent: &mut Agent,
    agent_config: &mut AgentConfig,
    session_total: &mut Usage,
    session_changes: &SessionChanges,
    turn_history: &mut TurnHistory,
    turn_snap: TurnSnapshot,
    changes_before: &[String],
    last_error: &mut Option<String>,
    prompt_start: Instant,
    effective_input: &str,
) {
    // - Ring bell
    // - Set last_error from outcome
    // - Overflow notification
    // - Fallback provider retry (if API error + fallback configured)
    // - Update turn snapshots with newly modified files
    // - Push turn snapshot to history
    // - Run watch-after-prompt if files were modified
    // - Auto-commit if enabled and files changed
    // - Auto-compact
}
```

## Implementation notes

- Both helpers should be `async fn` since they use `.await` internally.
- The helpers live in the same file (`src/repl.rs`) as private functions — no module boundary changes.
- The main loop body becomes: dispatch → prepare input → call architect or normal prompt → call post-prompt handler.
- Keep all existing behavior identical — this is pure mechanical extraction with no behavior changes.
- The `effective_input` parameter is needed by the fallback retry path (it re-sends the same prompt).
- Run `cargo test` to verify no regressions. The existing repl tests don't test `run_repl` directly (it's async + needs terminal), so passing build + existing tests is sufficient.

## Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

The function line count of `run_repl` should drop from ~985 to ~750 or less.
