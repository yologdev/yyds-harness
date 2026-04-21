Title: Scaffold /extended command for long-running autonomous task mode
Files: src/commands.rs, src/repl.rs, src/cli.rs
Issue: #278

Community issue #278 requests an `/extended` command for long-running autonomous tasks — the kind where you give yoyo a big task and walk away. This is also identified in the assessment as a competitive gap (extended autonomous task mode). This task adds ONLY the scaffolding — command parsing, help text, REPL wiring, and a basic implementation that uses the existing agent loop with a budget.

**What to do:**

1. **In `src/commands.rs`:**
   - Add `"/extended"` to `KNOWN_COMMANDS` array.
   - Add completion entries for `/extended` in `command_arg_completions` (subcommands: none needed initially, but accept a prompt string).

2. **In `src/cli.rs`:**
   - Add `"extended"` to the shell subcommand dispatch in `try_dispatch_subcommand` so `yoyo extended "build a REST API"` works from shell.
   - Add help text for `/extended` in the help system.

3. **In `src/repl.rs`:**
   - Wire `/extended` in the command dispatch (or the new `dispatch_command` if task_02 landed first).
   - Implementation: parse the prompt from `/extended <prompt>`. If no prompt given, print usage and return.
   - Create a new `handle_extended` function (can be in repl.rs for now since it needs agent access):

```rust
async fn handle_extended(
    prompt: &str,
    agent: &mut Agent,
    model: &str,
    session_total: &mut Usage,
    session_changes: &SessionChanges,
) {
    // 1. Print banner: "🐙 Extended mode — working autonomously..."
    // 2. Set up a turn limit (default 20 turns, overridable with --turns N in the prompt)
    // 3. Build an enhanced system instruction that tells the agent:
    //    - You are in extended autonomous mode
    //    - Work step by step on the task
    //    - Run tests after changes
    //    - If you get stuck, explain what you tried
    //    - Do not ask the user questions — make your best judgment
    // 4. Run the agent loop using run_prompt_auto_retry with the enhanced prompt
    // 5. After each turn, check if the agent's response contains a "TASK COMPLETE" marker
    //    or if the turn limit is reached
    // 6. Print summary: turns used, files modified, test results
}
```

**Key design decisions:**
- Start simple: extended mode is just a regular prompt with a "don't ask questions, keep going" instruction and a turn counter. No separate agents yet (that's a follow-up).
- Default budget: 20 turns. Parse `--turns N` from the prompt string if present.
- The agent still uses the same tools, same permissions, same everything. The only difference is the system-level instruction to be autonomous.
- Print progress after each turn: "Turn 3/20 — modified 2 files"

**What NOT to do:**
- Don't build a separate evaluator agent (that's a follow-up)
- Don't build screenshot comparison / design iteration (that's way out of scope)
- Don't modify the agent's tool set or permissions
- Don't create a new file — keep handle_extended in repl.rs for now

**Verification:** `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
Add a test that verifies the turn limit parsing (e.g., extracting `--turns 10` from a prompt string).

**Docs:** Add `/extended` to the help text and mention in CLAUDE.md under the commands list if it's in a relevant section.
