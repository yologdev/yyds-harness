Title: Close task unlanded source-edit gap in completion gate
Files: scripts/task_completion_gate.py, scripts/test_task_lineage_feedback.py, scripts/test_build_evolution_dashboard.py
Issue: none
Origin: planner
validated_against_assessment: true

Evidence:
- YOUR TRAJECTORY shows `task_unlanded_source_count=1` — one task touched source files but no landed source commit was recorded. The gate (`scripts/task_completion_gate.py`) is called by evolve.sh at line 2781 with `--auto-commit` to catch this, but the unlanded edit still occurred.
- Graph-derived next-task pressure: "Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit."
- The completion gate's `verify()` function already handles auto-commit for detected uncommitted files, but the detection chain may have a gap: `uncommitted_source_files()` checks `git diff --cached`, `git diff`, and `git ls-files --others --exclude-standard`, filtered through `task_lineage.source_file()`. If the source_file filter is too strict, or if git state is inconsistent at gate invocation time, files could be missed.
- The `auto_commit()` function returns `ok: True` when `git diff --cached --quiet` shows no staged changes after `git add` — this could mask cases where git add silently did nothing (e.g., files were already committed but the diff showed them as new because they exist only in the worktree, or the files don't exist on disk but were listed by `git diff --name-only`).

Edit Surface:
- scripts/task_completion_gate.py, scripts/test_task_lineage_feedback.py, scripts/test_build_evolution_dashboard.py

Verifier:
- python3 scripts/task_completion_gate.py --test

Fallback:
- If the self-tests all pass and code review finds no bugs in the gate logic, the unlanded edit is likely caused by a timing/state issue in evolve.sh (protected file). In that case, add a diagnostic to the gate that records WHY unlanded edits happen (state of git index, diff output, touched files) so future sessions can diagnose without guessing. Do not attempt to modify evolve.sh.

Objective:
Ensure every task that touches source files produces either a landed commit or a clear diagnostic explaining why the commit didn't land.

Why this matters:
Unlanded source edits corrupt the evolution signal. The dashboard reports a task as "verified" based on eval output, but if the source commit never lands, the verification is against a ghost — the code changes that passed tests aren't in the repo. This makes task_success_rate an unreliable metric and wastes API budget on phantom successes.

Success Criteria:
- `task_completion_gate.py --test` passes (unchanged behavior for known-good cases).
- Added self-test(s) cover edge cases: files listed by `git diff --name-only` that don't exist on disk, files that are gitignored but still listed, files that match the source_file pattern but are in a submodule or detached worktree.
- The gate's diagnostic output (the JSON payload written to evolve.sh) includes enough detail to distinguish "no source files were touched" from "source files were touched but the commit failed" from "source files were committed but the detection missed them."
- Any bug found in the detection chain is fixed.

Verification:
- python3 scripts/task_completion_gate.py --test
- python3 -m pytest scripts/test_task_lineage_feedback.py -x -k "task_completion" 2>/dev/null || python3 -m unittest scripts.test_task_lineage_feedback.TestTaskLineageFeedback.test_task_completion_gate_detects_unlanded_source_edits 2>/dev/null || echo "test not found by name — run broader suite"
- python3 -m unittest scripts.test_build_evolution_dashboard -k 2>/dev/null || echo "dashboard tests: check manually if needed"

Expected Evidence:
- Next trajectory shows task_unlanded_source_count=0.
- Future task lineage records show either a source_commit_sha in the landing evidence or a diagnostic reason explaining the gap.
- The gate's JSON payload in audit artifacts includes `uncommitted_source_files` and `source_files` fields that distinguish the detection layers.

Implementation Notes:
- Focus on `uncommitted_source_files()` (line 36) and `auto_commit()` (line 80) in task_completion_gate.py.
- Key areas to investigate:
  1. Does `git diff --name-only` return files that have been deleted? If so, `git add` on a deleted file might silently succeed without staging anything.
  2. Does `task_lineage.source_file()` filter out files that are source files but in unexpected locations? Check the filter logic.
  3. The `auto_commit` function returns `ok: True` when `git diff --cached --quiet` returns 0 after add. This means "nothing was staged" — but the function interprets it as success. Consider whether this should be a warning instead.
  4. If `uncommitted_source_files()` returns files but `git add` fails silently (return code 0 but nothing staged), the commit path is skipped and the gate reports success.
- Add one or two self-tests for edge cases you identify. Do not refactor the entire file.
- Do not modify `scripts/evolve.sh` (protected). If the root cause is in evolve.sh, add a diagnostic note to the gate's output and document the finding.
