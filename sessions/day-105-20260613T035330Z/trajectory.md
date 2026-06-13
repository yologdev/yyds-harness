# YOUR TRAJECTORY

Last computed: 2026-06-13T03:56Z. Day 105. Window: last 10 sessions / 14 days.

## Recent session outcomes (last 10)
day-104 (2026-06-12 18:21:44): tasks 0/1 ⚠️ — 0/1 strict verified; 1 no touched files; 1 no passing verifier
day-104 (2026-06-12 12:12:45): tasks 1/2 ⚠️ — 1/2 strict verified; 1 no touched files; 1 no passing verifier
day-104 (2026-06-12 04:27:18): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-103 (2026-06-11 19:21:24): tasks 0/1 ⚠️ — 0/1 strict verified; 1 no touched files; 1 no passing verifier
day-103 (2026-06-11 15:50:44): tasks 2/3 ⚠️ — 2/3 strict verified; 1 no touched files; 1 no passing verifier
day-103 (2026-06-11 13:10:28): tasks 0/0 • — no tasks attempted
day-103 (2026-06-11 12:36:09): tasks 0/0 • — no tasks attempted
day-103 (2026-06-11 10:54:53): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-103 (2026-06-11 08:44:38): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-103 (2026-06-11 04:31:55): tasks 0/1 ⚠️ — 0/1 strict verified; 1 no passing verifier; 1 source edits not landed

## Per-task activity (last 14 days)
"Fix `state why last-failure` windowing blind spot": 1 attempt(s), last day-104

## Reverts in window
0 of last ~10 sessions had reverts.

## Recurring CI errors (failed runs in window)
[2×] test watch::tests::test_watch_result_failed_with_error ... ok
[2×] assertion `left == right` failed
[2×] ^[[1m^[[91merror^[[0m: test failed, to rerun pass `--lib`
[2×] ##[error]process completed with exit code 101.
[2×] error_count: 10

## Provider/API health
10 sessions, no provider errors detected.

## GitHub Actions log feedback
latest score=0.7219 confidence=1.0 recurring_failures=0 state_capture=1.0
Top lessons for next run:
- search tool or grep produced an error -> escape generated search patterns and avoid binary/.git/target paths in evidence scans
- max task turn count is high: 26 -> split broad tasks earlier or add task-specific context so implementation converges in fewer turns
- the seed task_01.md has a factual error: "deepseek cache-report returning no metrics" — th -> inspect the failing phase and add a targeted harness guard or eval fixture
Repeated across prior log feedback:
- 2x │ search error: grep: src/main.rs: no such file or directory
- 2x │ command timed out after 180s
- 2x │ command timed out after 120s

## Structured state snapshot
claims: 257/342 proven; 85 unresolved
- missing 36x deepseek_model_call_lifecycle_balanced latest=day-104-20260612T180824Z
- missing 27x state_run_lifecycle_balanced latest=day-104-20260612T180824Z
- observed 22x assessment_artifact_and_transcript_state latest=day-103-20260611T184741Z
task states: verified_landed=11; reverted_no_git_visible_changes=6; scope_mismatch=4; verifier_unproven=4; reverted_unlanded_source_edits=3
tool failures: search_regex_error=57; search_binary_match=19; missing_file_read=11; read_error=11; search_error=9

## Graph-derived next-task pressure
... (truncated to fit token budget)
