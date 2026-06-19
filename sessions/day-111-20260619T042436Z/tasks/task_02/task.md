Title: Add early-edit guard for zero-change implementation tasks
Files: scripts/evolve.sh
Issue: none
Origin: planner

Evidence:
- Graph-derived pressure row #1: "Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an early scoped edit, an obsolete note, or a concrete blocker."
- Trajectory: `reverted_no_edit=4` instances across recent sessions. `task_success_rate=0.5` — half of attempted tasks produce zero file changes and get reverted.
- Dashboard corrected lesson: "implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker."
- Day 109 added analysis-only retry stop (34 lines in evolve.sh): harness no longer retries implementation attempts that produce zero file changes. This detection happens AFTER the full implementation + fix-loop budget is spent. The gap: detection should happen EARLY, before burning fix-loop time.
- The preseed task picker (Day 110) now checks file existence before assigning tasks, preventing agents from being pointed at renamed/deleted files. This addresses one cause of zero-edit tasks but not all — agents can still stall without touching code.

Edit Surface:
- scripts/evolve.sh

Verifier:
- cargo build (evolve.sh is bash, but verify no Rust changes needed)
- grep -n 'early_edit\|zero_change\|no_edit_guard\|reverted_no_edit' scripts/evolve.sh (should find the new guard)

Fallback:
- If evolve.sh already has an early-edit check that fires before fix loops, mark this task obsolete.
- If the guard would require changes outside scripts/evolve.sh (e.g., Rust-side agent instrumentation), narrow the task to just the shell-side timeout/budget check and note what Rust-side work remains.

Objective:
Prevent implementation tasks from consuming fix-loop budget when they produce zero file changes. Detect the no-edit condition early (within first few minutes) and abort to reverted_no_edit instead of running through retries.

Why this matters:
Every reverted_no_edit task wastes 20+ minutes of session budget (implementation time + fix loops). With 4 such failures across recent sessions, that's ~80 minutes of wasted CI time and API costs. The graph evidence shows this is the dominant failure mode — fixing it directly raises `task_success_rate` and reduces wasted spend.

Success Criteria:
- After the implementation agent's first turn (or within the first 3 minutes), if no tracked source files have been modified, the harness emits a clear "no edits detected" event and skips remaining fix loops.
- The task is classified as `reverted_no_edit` with a concrete reason in the state event.
- Normal tasks that DO produce edits within the first few minutes are unaffected.
- The check does not add more than 30 seconds of overhead to normal task execution.

Verification:
- bash -n scripts/evolve.sh (syntax check)
- Manual inspection of the guard logic for correctness

Expected Evidence:
- Future trajectory shows `task_no_edit_revert_count` drops toward 0.
- Future sessions show `task_success_rate` rising above 0.5.
- State events for reverted tasks include a `no_edit_detected_at` timestamp.

Implementation Notes:
- The guard should run as a background check during task implementation, not as a pre-check (we can't know the agent won't edit until it's had a chance to).
- Implementation approach: after the implementation agent starts, poll `git diff --name-only` every 60 seconds. If no tracked files in the task's Files: list have changed after 3 polls (3 minutes) AND the agent hasn't produced a terminal event, consider it a no-edit task.
- Alternatively, simpler: after the implementation agent's first `ToolCallCompleted` event for `bash` or `edit_file` or `write_file`, check if any of the task's target files were modified. If not after 3 such tool completions, abort.
- The simpler approach: in the implementation phase loop, after each turn, run `git diff --name-only -- src/` and check if any files match the task's declared Files:. If 3 turns pass with no matching edits, abort with `reverted_no_edit`.
- This is a bash-level change only — no Rust code required. Use `append_state_event_checked` or `record_state_event` to log the early-abort.
- Coordinate with the existing analysis-only retry stop (Day 109) — the early guard replaces the need for that late check in most cases, but keep the late check as a safety net.
