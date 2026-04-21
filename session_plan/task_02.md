Title: Extract command dispatch from run_repl into a dedicated function
Files: src/repl.rs
Issue: none

`run_repl` is 945 lines — the single largest function in the codebase. The assessment identifies this as the top maintainability risk. The function does three things: (1) initialization/setup, (2) command dispatch (a giant match block), and (3) agent prompt handling + post-turn logic. This task extracts ONLY the command dispatch — the most mechanical and safest part.

**What to do:**

1. Create a new enum `CommandResult` at the top of repl.rs:
```rust
enum CommandResult {
    Continue,           // Command handled, go to next prompt
    Quit,               // User wants to exit
    SendToAgent(String), // Command produced a prompt to send to the agent
    NotACommand,        // Input isn't a slash command, fall through to agent
}
```

2. Extract the match block (approximately lines 410–940 relative to run_repl start) into a new async function:
```rust
async fn dispatch_command(
    input: &str,
    agent: &mut Agent,
    agent_config: &mut AgentConfig,
    session_total: &mut Usage,
    session_changes: &SessionChanges,
    turn_history: &mut TurnHistory,
    bg_tracker: &BackgroundJobTracker,
    spawn_tracker: &SpawnTracker,
    undo_context: &mut Option<String>,
) -> CommandResult
```

3. Move ALL the match arms for slash commands into `dispatch_command`. The match arms that just `continue` become `CommandResult::Continue`. The `"/quit" | "/exit"` arm becomes `CommandResult::Quit`. Arms that set `last_input` (like `/explain`, `/review`, `/plan`, `/spawn`, `/pr`) return `CommandResult::SendToAgent(prompt)`.

4. In run_repl's main loop, replace the match block with:
```rust
let cmd_result = dispatch_command(input, agent, agent_config, ...).await;
match cmd_result {
    CommandResult::Quit => break,
    CommandResult::Continue => continue,
    CommandResult::SendToAgent(prompt) => {
        last_input = Some(prompt);
        // fall through to agent prompt handling
    }
    CommandResult::NotACommand => {
        // fall through to agent prompt handling (the input itself is the prompt)
    }
}
```

**Critical constraints:**
- This is a PURE refactor. Zero behavior change.
- Do NOT try to split the agent prompt handling or post-turn logic — that's a separate, harder task.
- Do NOT try to reduce the parameter count of dispatch_command. A function with many params is fine when it replaces 500 lines of inline code.
- Keep all the match arms EXACTLY as they are — just move them. Don't refactor individual arms.
- Some arms reference local variables like `rl` (readline), `last_input`, `session_start`, etc. These may need to be passed as additional parameters to dispatch_command, OR the function can be restructured to handle them. Choose the simplest approach.

**Testing:** `cargo build && cargo test` — all existing tests pass unchanged since behavior is identical.

**Docs:** Update CLAUDE.md's Architecture section to mention `dispatch_command` under repl.rs.
