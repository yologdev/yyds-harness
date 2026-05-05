Title: Extract REPL startup banner into a dedicated function in repl.rs
Files: src/repl.rs
Issue: none

`run_repl` is still 326 lines. The first ~100 lines (582-680) are pure startup information
printing: model name, provider, thinking level, temperature, skills count, MCP count, verbose
flag, permissions, fallback, git branch, cwd, update notification, previous session hint, and
auto-watch detection. This is a self-contained block with no control flow dependencies on the
rest of `run_repl` — it reads from `agent_config` and `repl_config` and prints to stdout/stderr.

**Extract `fn print_startup_info(agent_config: &AgentConfig, repl_config: &ReplConfig)`:**
- Move lines 582-680 (from `let cwd = ...` through the auto-watch block and its println) into
  a new private function
- The function takes `agent_config` and `repl_config` as parameters
- It calls `print_banner()` at the top (already there)
- It handles all the conditional printing (provider, model, base_url, thinking, temperature,
  skills, mcp, openapi, verbose, auto_approve, permissions, fallback, git, cwd)
- It handles the update notification
- It handles the previous session hint
- It handles auto-watch detection and setup (this calls `set_watch_command` as a side effect,
  which is fine — it returns nothing and just prints)
- `run_repl` calls it once at the top, then proceeds to the editor setup and main loop

**What NOT to change:**
- Don't touch the main loop or any other part of `run_repl`
- Don't change any existing tests
- Don't change any behavior — pure extraction

**Verification:** `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`

This reduces `run_repl` from ~326 to ~226 lines, making the main loop more visible and the
startup sequence independently readable.
