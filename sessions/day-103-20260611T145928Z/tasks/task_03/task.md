Title: Wire crash reporter into REPL startup failures
Files: src/repl.rs
Issue: none
Origin: planner

Objective:
Convert the `expect("Failed to initialize readline")` panic in `run_repl()` into a graceful error that emits a diagnostic event, so REPL startup failures are recorded in the state system instead of crashing the process silently.

Why this matters:
`run_repl()` at line 720 calls `Editor::with_config(config).expect("Failed to initialize readline")`. If rustyline can't initialize (terminal issues, missing terminfo, broken config), the process panics with no diagnostic. The assessment lists "REPL startup" as an uncovered gap. Unlike most crash reporter gaps where the process continues after failure, this one kills the process — making it the highest-impact uncovered gap. A diagnostic here means the state system can at least record *why* the REPL died, even if it can't recover.

Success Criteria:
- `Editor::with_config()` failure emits `"repl: readline_init_failed: {error}"` via `stash_diagnostic_error`
- The process still exits (readline is essential for REPL mode) but the diagnostic is recorded first
- Existing happy-path REPL startup is unchanged
- `cargo build && cargo test` passes

Verification:
- cargo check
- cargo test repl
- cargo test -- --test-threads=1

Expected Evidence:
- Task lineage links `src/repl.rs` to this task
- Future state events show `repl: readline_init_failed` diagnostic kind when rustyline can't start
- `/state crashes` can distinguish REPL startup failures from agent-run failures

Implementation Notes:
- Convert lines 720-721 from:
  ```
  let mut rl = Editor::with_config(config).expect("Failed to initialize readline");
  rl.set_helper(Some(YoyoHelper));
  ```
  to:
  ```
  let mut rl = match Editor::with_config(config) {
      Ok(editor) => editor,
      Err(e) => {
          crate::state::stash_diagnostic_error(&format!("repl: readline_init_failed: {e}"));
          eprintln!("Failed to initialize readline: {e}");
          return;
      }
  };
  rl.set_helper(Some(YoyoHelper));
  ```
- This is the only change needed — the history loading on lines 722-725 already handles errors gracefully
- The `return` from `run_repl` is clean because the function returns `()` and all cleanup is handled by Rust's drop semantics
- This is a 5-line change, verifiable in one compile cycle
