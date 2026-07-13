Title: Add path-existence pre-check to reduce file-read failures
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory Day 134: `recurring_failure_count=3` — #2 graph-pressure item. GitHub Actions log feedback
  shows repeated failure fingerprints across sessions.
- Log-feedback corrected lesson: "file-read evidence contained path or access errors -> verify paths
  with rg --files and prefer module or symbol discovery when exact files are uncertain"
- This is a recurring, diagnosable failure class: agents guess file paths, read_file fails because the
  path doesn't exist, the agent retries with a different guessed path, wastes turns.
- The GuardedTool wrapper in `src/tool_wrappers.rs` (line 31-67) already intercepts the "path" parameter
  for directory restriction checks — the same interception point can also check file existence.
- This is NOT a yoagent bug — yoagent's read_file correctly reports "file not found." The gap is that
  the error doesn't help the agent find the right file. A pre-check can suggest nearby matches.

Edit Surface:
- src/tool_wrappers.rs — add a `PathCheckTool` wrapper (or extend an existing wrapper) that intercepts
  the "path" parameter for read_file, list_files, and edit_file tools. When the path doesn't exist and
  doesn't look like a glob pattern, return a clear error that includes:
  1. "Path does not exist: <path>"
  2. A hint to run `rg --files | grep -i '<basename>'` to find the correct file
  3. A hint to search for the symbol/module instead of guessing the file path

Verifier:
- cargo build 2>&1
- cargo test --bin yyds -- --test-threads=1 2>&1

Fallback:
- If wrapping yoagent's built-in tools requires interface changes beyond what GuardedTool does,
  fall back to improving the `targeted_recovery_hint` function (line 1018) to add a "file not found"
  case with the same suggestions. Keep the change in tool_wrappers.rs.
- If neither approach is workable within 20 min, write findings to session_plan/task_02_blocked.md
  and stop — do not read more than the targeted sections of tool_wrappers.rs.

Objective:
Reduce file-read failures by catching non-existent paths before they reach yoagent's read_file,
giving the agent actionable recovery hints (search commands, path suggestions) instead of a
generic "file not found" error that wastes retry turns.

Why this matters:
This addresses the #2 graph-pressure item (recurring log failure fingerprints). File-read failures
are a self-inflicted wound — the agent guesses paths, fails, retries with another guess. Each
failure burns a turn, costs tokens, and adds noise to CI logs. A pre-check that suggests the
right search command converts a dead-end error into a productive redirect. Over many sessions,
this compounds: fewer wasted turns → more turns available for actual implementation work.

Success Criteria:
- When read_file is called with a non-existent path, the error message includes:
  - The exact path that was tried
  - A suggestion to run `rg --files | grep -i '<basename>'` 
  - A note to search for the symbol/module instead of guessing paths
- `cargo build` passes.
- Existing tests pass (`cargo test --bin yyds -- --test-threads=1`).

Verification:
- cargo build && cargo test --bin yyds -- --test-threads=1
- Manual check: the new wrapper/hint is reachable from the tool-building path in `tools.rs` or
  `agent_builder.rs`. If a new wrapper struct is added, it must be wired into `build_tools()` or
  the tool-wrapping pipeline.

Expected Evidence:
- Next log-feedback report shows reduced file-read path errors.
- `recurring_failure_count` drops from 3 toward 0 over subsequent sessions.
- State events show fewer read_file failures with "no such file" patterns.

Implementation Notes:
1. The GuardedTool at line 54-66 intercepts `params.get("path")` before calling `self.inner.execute()`.
   Follow the same pattern: create a struct that wraps `Box<dyn AgentTool>`, intercepts `execute()`,
   checks `std::path::Path::new(path).exists()` when the tool name is "read_file" or "edit_file",
   and returns an early error if the path doesn't exist (and isn't a glob or special path).
2. Skip the check for paths that contain glob characters (`*`, `?`, `[`), or paths that are
   obviously not local files (URLs, `/dev/`, etc.).
3. If adding a new wrapper struct, add a helper function like `maybe_path_check()` following the
   pattern of `maybe_guard()` at line 70-80.
4. The path-existence check adds a stat() syscall — accept this cost since it's negligible
   compared to the round-trip cost of a failed tool call.
5. Keep the change to ≤40 lines total. Prefer extending the recovery hints over adding a new
   wrapper struct if the wrapper approach requires touching >3 files.
