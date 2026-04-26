Title: Extract early-exit CLI modes from main() into helper functions
Files: src/main.rs
Issue: none

## What to do

Continue decomposing `main()` by extracting the session-restore, early-exit branches (--prompt and piped mode), and pre-REPL setup into named helpers. This removes another ~80-100 lines of procedural logic from main().

### Specifically:

1. **Extract session restore** into a helper function `restore_session(agent: &mut Agent, session_path: &str)`:
   - Takes the agent and session path
   - Handles the `std::fs::read_to_string` → `agent.restore_messages` → success/error printing
   - Currently ~15 lines in main() (around the `if continue_session { ... }` block)

2. **Extract the pre-main setup block** (lines ~808-920) into a function `apply_early_flags(args: &[String]) -> Option<Config>`:
   - This includes: --no-color check, --no-bell check, --no-rtk check, parse_args call, --print-system-prompt exit, verbose/audit flags, and the setup wizard check
   - Returns `None` if main should exit (help/version/print-system-prompt handled), `Some(config)` otherwise
   - OR alternatively, just group the flag-checking into `apply_cli_flags(args: &[String])` that handles --no-color, --no-bell, --no-rtk and keep parse_args separate

3. The goal is to make the remaining `main()` body read as a clear pipeline:
   ```
   apply_cli_flags(&args);
   let config = parse_args(&args)?;
   // early exits
   // build agent
   // connect external servers  (from task_01)
   // restore session if --continue
   // dispatch to single-prompt / piped / REPL
   ```

### Design constraints:
- Keep the function signatures simple — pass only what's needed
- Don't change any behavior — pure extraction refactor
- The `is_interactive` flag and setup wizard interaction must still work correctly

### Verification:
- `cargo build` must pass
- `cargo test` must pass
- `cargo clippy --all-targets -- -D warnings` must pass
