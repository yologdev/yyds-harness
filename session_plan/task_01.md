Title: Extract dispatch_command from repl.rs into dispatch module
Files: src/repl.rs, src/dispatch.rs
Issue: none

The 602-line `dispatch_command` async function in `repl.rs` (line 302) is the single largest function in the codebase and has been deferred across 2+ sessions. It's a monolithic match statement routing 40+ slash commands with 9 parameters.

**What to do:**

1. Move the `dispatch_command` function from `repl.rs` into `src/dispatch.rs` (which already has `try_dispatch_subcommand` for shell-mode dispatch).

2. The function signature is:
   ```rust
   async fn dispatch_command(
       input: &str,
       agent: &mut yoagent::agent::Agent,
       agent_config: &mut AgentConfig,
       session_total: &mut Usage,
       session_changes: &SessionChanges,
       turn_history: &mut TurnHistory,
       bg_tracker: &commands::BackgroundJobTracker,
       spawn_tracker: &commands::SpawnTracker,
   ) -> Option<DispatchResult>
   ```

3. Make it `pub(crate) async fn dispatch_command(...)` in `dispatch.rs`.

4. In `repl.rs`, replace the function body with a call to `crate::dispatch::dispatch_command(...)`.

5. Move any types/enums that `dispatch_command` returns (like `DispatchResult` if it exists) to `dispatch.rs` as well.

6. Ensure all necessary imports are added to `dispatch.rs` — the function calls into many `commands_*` modules.

7. Run `cargo build && cargo test` to verify.

**Key constraint:** This is a pure move refactor — zero behavior changes. The match arms stay exactly as they are. Don't reorganize or split the match further; just move the function to its proper home.

**Why:** `repl.rs` should be about the REPL loop mechanics (readline, multiline collection, input processing). Command dispatch is a separate concern that already has a home in `dispatch.rs`.
