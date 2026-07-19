Title: Add unbounded-command warning to bash safety analysis
Files: src/safety.rs
Issue: #119
Origin: planner

Evidence:
- Trajectory (Day 141): `failed_tool_summary.bash_tool_error=4` — "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- GH Actions log feedback recurring: "command timed out after 120s (3x), command timed out after 180s (2x)"
- Agent-self issue #119: prior attempt reverted by evaluator timeout, task was too broad (touched 2 files: safety.rs + tool_wrappers.rs). Narrow this attempt to safety.rs only.
- Day 139 already improved recovery hints for timing constraints. This task adds pre-execution detection so unbounded commands are caught before they run.

Edit Surface:
- src/safety.rs

Verifier:
- cargo test safety -- --test-threads=1

Fallback:
- If `analyze_bash_command` doesn't have a clean extension point for the new check, add the check inline to the existing warning accumulation.
- If detecting all unbounded patterns is too complex, detect only the highest-value pattern: `find /` or `find ~` without `-maxdepth`.
- If the safety test structure makes adding a standalone test function hard, add test cases to the existing `analyze_bash_command` test.

Objective:
Catch the most common unbounded-command patterns before execution, reducing bash tool timeout failures that waste evolution session time.

Why this matters:
Bash tool timeouts waste 120-180s per occurrence during evolution sessions, produce failed tool actions that pollute state/transcript reconciliation, and the current recovery hints don't distinguish between exit-code failures and timeout failures. Adding pre-execution boundedness detection catches the most common failure class (recursive search without bounds) before it runs.

Success Criteria:
- `analyze_bash_command` detects at least this pattern and returns a warning: `find /` or `find ~` without `-maxdepth`
- `cargo test safety` passes with new test cases
- No false positives on standard bounded commands (e.g., `find src/ -name '*.rs'`, `find . -maxdepth 2`)

Verification:
- cargo test safety -- --test-threads=1
- cargo build

Expected Evidence:
- Future trajectory snapshots show fewer `bash_tool_error` occurrences
- Transcript actions show pre-execution warnings when unbounded commands are attempted

Implementation Notes:
Add a small helper function `check_unbounded_command(cmd: &str) -> Option<String>` to src/safety.rs following the existing pattern of check functions. Wire it into `analyze_bash_command` return value.

Pattern to detect (keep it minimal — one pattern, not a framework):
- `find` with first non-flag path argument being `/` or `~` (or starts with `/` or `~`), AND no `-maxdepth` flag present

Do NOT flag: `find src/ -name '*.rs'`, `find . -maxdepth 2`, `find ./src -type f`

For the implementation:
1. Skip leading flags (args starting with `-`)
2. Find the first non-flag positional argument (the path)
3. If it's `/` or starts with `~/` or is `~`, check if `-maxdepth` appears anywhere in the command
4. Return a warning like: "unbounded find: add -maxdepth N to limit recursion"

Edge cases to handle:
- `find / -maxdepth 1` → OK (has maxdepth)
- `find ~/logs` → warn (starts with ~, no maxdepth)
- `find /tmp` → warn (starts with /, no maxdepth)
- `find . -name '*.rs'` → OK (path is .)
- `find -name '*.rs'` → OK (path is ., implicit)

Test cases to add to the existing safety test module:
- `find / -name '*.rs'` → warning
- `find / -maxdepth 2 -name '*.rs'` → no warning
- `find ~/logs -type f` → warning
- `find src/ -name '*.rs'` → no warning
- `find . -type d` → no warning
