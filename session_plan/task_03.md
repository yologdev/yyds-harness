Title: Error-aware /run — offer to analyze failures
Files: src/commands_run.rs
Issue: none

## Context

When a user runs `/run cargo test` or `!make` and the command fails, yoyo currently just prints the exit code and moves on. Cursor's debug mode automatically captures error output and offers to fix it. This task bridges that gap by making `/run` capture stderr on failure and return a structured result the REPL can use to offer analysis.

## What to do

### 1. Refactor `run_shell_command` to return a result

Change `run_shell_command` to return a struct instead of printing directly:

```rust
pub struct RunResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: std::time::Duration,
    pub success: bool,
}
```

Extract the current printing logic into a separate `print_run_result` function. The `run_shell_command` function becomes:
- Spawn the process (same as now)
- Capture stdout and stderr into strings (in addition to streaming to terminal)
- Return `RunResult`

**Important:** Keep streaming stdout/stderr to the terminal in real-time (user experience shouldn't degrade). Capture into buffers simultaneously using the existing thread approach but collecting lines into a `Vec<String>` alongside printing.

### 2. Modify `handle_run` to offer error analysis

After `run_shell_command` returns:
- If `exit_code == 0` → done (same as now)
- If `exit_code != 0` → print a hint line:
  ```
  💡 Command failed. Ask me to analyze the error, or say /fix to auto-fix.
  ```
  
  Also store the last failed run result in a module-level static so `/fix` or the agent can reference it:
  ```rust
  static LAST_FAILED_RUN: Mutex<Option<RunResult>> = ...;
  ```
  
  Add a `pub fn get_last_failed_run() -> Option<RunResult>` function.

### 3. Add tests

- Test `RunResult` construction
- Test that `get_last_failed_run` returns `None` initially
- Test that storing and retrieving works
- Test the hint message is correct format

### 4. Do NOT modify dispatch.rs

The `/run` routing already works. Just change the behavior inside `commands_run.rs`.

## Verification

- `cargo build && cargo test`  
- `cargo clippy --all-targets -- -D warnings` clean
- The streaming output still works (stdout and stderr print in real-time)
- After a failed command, the hint line appears
