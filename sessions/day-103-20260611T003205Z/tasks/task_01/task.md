Title: Wire crash reporter into harness startup and context loading failure paths
Files: src/lib.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Extend crash diagnostic capture to cover harness startup failures and context loading
failures — two paths that caused silent crashes during Days 100-102 with zero
diagnostic visibility.

Why this matters:
The crash reporter (stash_diagnostic_error / take_diagnostic_error in state.rs)
exists but is only called from state init failure in src/lib.rs line ~1032. Days
100-102 had 8+ sessions crash before any tool call fired — likely during harness
startup, context loading, or MCP connection. Without diagnostic capture on these
paths, every crash is a mystery. Wiring the reporter into more entry points
makes future crashes diagnosable instead of invisible.

Success Criteria:
- stash_diagnostic_error() is called when harness startup fails (before agent
  creation, MCP connection, or prompt execution)
- stash_diagnostic_error() is called when context loading fails (project context,
  semantic index, embedding index)
- Existing tests pass (cargo build && cargo test)
- The /state crashes command shows captured errors from startup/context paths

Verification:
- cargo build --lib
- cargo test --lib -- state
- cargo test --lib -- release (no regressions)
- Manual: verify stash_diagnostic_error is called in at least 3 new code paths

Expected Evidence:
- State events: CrashDiagnosticStashed events appear for startup/context failures
- /state crashes output includes errors from non-init paths
- Fewer mystery crashes in future sessions

---

## What to do

The crash reporter lives in `src/state.rs`:
- `pub fn stash_diagnostic_error(error: &str)` — stashes an error string
- `pub fn take_diagnostic_error() -> Option<String>` — retrieves it
- Already called in `src/lib.rs` around line 1032 during state init failure

### 1. Add crash capture to harness startup in src/lib.rs

In `run_cli()` (or whichever function orchestrates startup), find these failure
points and call `stash_diagnostic_error()` before propagating the error:

- Agent creation failure (build_agent / AgentConfig)
- MCP server connection failure
- DeepSeek model configuration failure
- Skill loading failure

Use the pattern already established at line ~1032 as a guide. The stash should
happen right before the error is returned/propagated, with a descriptive message
like "yyds startup: agent creation failed: {details}".

### 2. Add crash capture to context loading in src/context.rs

In the context loading functions (likely `load_project_context` or
`build_deepseek_context_preview`), add `stash_diagnostic_error()` calls for:

- Project context file read failures (YOYO.md, CLAUDE.md, etc.)
- Semantic index build failures
- Embedding index build failures
- Git status read failures (if that blocks context loading)

Use descriptive messages like "yyds context: semantic index build failed: {details}".

### 3. Keep it bounded

- Only add stash calls to paths that are genuinely fatal (session can't proceed)
- Don't add for recoverable warnings
- Each call site should be 2-3 lines max
- Do NOT refactor or restructure code — just add the stash calls

### Code pattern

```rust
// Before error propagation, add:
yoyo_ds_harness::state::stash_diagnostic_error(
    &format!("yyds startup: agent creation failed: {}", err)
);
```

Import `state` module at the top of files that don't already use it.
