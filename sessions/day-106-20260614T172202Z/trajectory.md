# YOUR TRAJECTORY

Last computed: 2026-06-14T17:25Z. Day 106. Window: last 10 sessions / 14 days.

## Recent session outcomes (newest 6 of 10)
day-106 (2026-06-14 11:05:28): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-106 (2026-06-14 04:31:28): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-105 (2026-06-13 17:45:38): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-105 (2026-06-13 10:44:51): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-105 (2026-06-13 04:23:50): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_protected_file_edits=1, reverted_scope_mismatch=1
day-104 (2026-06-12 18:21:44): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
... 4 older session outcome(s) omitted

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.0): Selected or attempted tasks did not all finish as verified successful...
- Validate seeded tasks against fresh assessment (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence; validate seeds...
- Require strict verifier evidence for tasks (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator...
- Break recurring log failure fingerprints (recurring_failure_count=2): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.6125 confidence=1.0 recurring_failures=2 state_capture=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- file-read evidence contained path or access errors -> verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain
- seeded tasks contradicted the fresh assessment -> validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 3x fatal: no pattern given
- 2x │ command timed out after 300s
- 2x [2×] error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 295/387 proven; 92 non-proven (missing=69, observed=23); 7 recent; recent non-proven claims: model_lifecycle=3 missing, run_lifecycle=3 missing, assessment_artifact=1 observed
- lifecycle aggregate: observed=34/43, unhealthy=21, run_incomplete=43, model_incomplete=24
- recent task issues: reverted_protected_file_edits=1, reverted_scope_mismatch=1, reverted_seed_contradicted=1
... (truncated to fit token budget)
