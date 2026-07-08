Title: Make healthy-codebase fallback produce a src/-touching task instead of journal-only
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Graph pressure: `planner_no_task_count=1` — "The planner produced no concrete task files."
- Assessment: "Day 129 (20:02) session produced zero task files — the planner found nothing to do despite a codebase with open issues and known friction points."
- Log feedback lesson: "planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts"
- The `_healthy_codebase_fallback()` function (line 955) returns a task with `files: "journals/JOURNAL.md"` — this produces a journal-only task that doesn't touch src/, so strict verification passes trivially without any code improvement.
- The Day 129 task manifest fix (`scripts/task_manifest.py`) now skips tasks with empty Files, but the healthy-codebase fallback has non-empty Files (journals/JOURNAL.md), so it doesn't get filtered.
- The codebase has open issue #37 (eval fixtures) and known friction points (state lifecycle gaps, cache metrics), so a "healthy codebase" assessment doesn't mean zero possible improvements — it means zero *obvious breakage*.

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest

Fallback:
- If the assessment already updated between planning and implementation to show non-healthy signals, the fallback won't activate and the task is satisfied by any candidate match. Mark as done.
- If `_healthy_codebase_fallback` was already changed in a prior session and now returns a src/-touching task, verify and mark as done.

Objective:
Modify `_healthy_codebase_fallback()` in `scripts/preseed_session_plan.py` so that when the assessment finds a healthy codebase, the fallback produces a small src/-touching improvement task instead of a journal-only task. The fallback task should either create a new eval fixture (under `eval/fixtures/local-smoke/`) or make a small, verifiable improvement to an existing src/ file — something that passes `cargo build && cargo test`.

Why this matters:
When the planner always produces a src/-touching task (even as a fallback), the implementation phase always has something verifiable to work on. This prevents the "no tasks attempted" sessions that waste evolution cycles. A journal-only fallback is honest but unproductive — the harness burns API credits to write a diary entry when it could be making a small, concrete improvement. This directly addresses the `planner_no_task_count` graph pressure and raises `task_success_rate`.

Success Criteria:
- `_healthy_codebase_fallback()` returns a task whose Files: entry includes at least one file under `src/` or `eval/fixtures/local-smoke/`.
- The fallback task's verifier includes `cargo build && cargo test` (or a focused subset) — it produces verifiable code changes.
- The fallback task is small enough to complete in 20 minutes (one fixture, one small fix, etc.).
- The fallback avoids self-reference — it does NOT modify the planning pipeline itself (no `scripts/preseed_session_plan.py` in its Files).
- `python3 scripts/preseed_session_plan.py --test` passes.
- `python3 -m unittest scripts.test_task_manifest` passes.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest
- Manual check: the fallback task's files field should include src/ or eval/fixtures/local-smoke/, not just journals/.

Expected Evidence:
- `planner_no_task_count` gnome drops from 1 toward 0 in subsequent sessions.
- Future healthy-codebase sessions produce tasks that touch src/ and pass cargo test.
- Task lineage shows `scripts/preseed_session_plan.py` as the changed file with a narrower non-self-referential fallback.

Detailed description:

The current `_healthy_codebase_fallback()` at line 955 of `scripts/preseed_session_plan.py` returns a journal-only task. Replace it with a fallback that produces a small src/ improvement. The implementation agent should:

1. Read the current `_healthy_codebase_fallback()` function and understand its structure.
2. Design a replacement fallback task. Good options include:
   - An eval fixture task: "Add a small eval fixture under eval/fixtures/local-smoke/ for a DeepSeek harness behavior (e.g., cache hit detection, transport error classification, or FIM routing)." This is always landable (fixtures are additive, pass cargo test), uses the existing fixture format, and directly improves eval coverage (issue #37).
   - A small code improvement: "Fix one clippy warning in src/" or "Add a unit test for an untested function in src/."
3. The replacement must use the existing task dict format (keys: title, files, objective, why, success, verification, evidence).
4. The replacement must NOT use `scripts/preseed_session_plan.py` as its files target (no self-reference).
5. Add the new fallback task's title to the TASKS list or handle it inline — ensure it follows the same validation path.
6. If the fallback adds an eval fixture, the fixture file should be named following the existing convention (NNN-description.json) and placed in `eval/fixtures/local-smoke/`.

The implementation agent should also add a test to `scripts/test_task_manifest.py` (if the test file supports it) or `scripts/preseed_session_plan.py --test` to verify the healthy-codebase fallback returns a src/-touching task.
