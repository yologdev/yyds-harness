# YOUR TRAJECTORY

Last computed: 2026-06-13T17:28Z. Day 105. Window: last 10 sessions / 14 days.

## Recent session outcomes (last 10)
day-105 (2026-06-13 10:44:51): tasks 0/1 ⚠️ — 0/1 strict verified; 1 no touched files; 1 no passing verifier
day-105 (2026-06-13 04:23:50): tasks 1/3 ⚠️ — 1/3 strict verified; 2 no passing verifier; 2 source edits not landed; 1 no planned-file overlap
day-104 (2026-06-12 18:21:44): tasks 0/1 ⚠️ — 0/1 strict verified; 1 no touched files; 1 no passing verifier
day-104 (2026-06-12 12:12:45): tasks 1/2 ⚠️ — 1/2 strict verified; 1 no touched files; 1 no passing verifier
day-104 (2026-06-12 04:27:18): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-103 (2026-06-11 19:21:24): tasks 0/1 ⚠️ — 0/1 strict verified; 1 no touched files; 1 no passing verifier
day-103 (2026-06-11 15:50:44): tasks 2/3 ⚠️ — 2/3 strict verified; 1 no touched files; 1 no passing verifier
day-103 (2026-06-11 13:10:28): tasks 0/0 • — no tasks attempted
day-103 (2026-06-11 12:36:09): tasks 0/0 • — no tasks attempted
day-103 (2026-06-11 10:54:53): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK

## Per-task activity (last 14 days)
"Add regex-error recovery hint to search tool error messages": 1 attempt(s), last day-105

## Reverts in window
0 of last ~10 sessions had reverts.

## Recurring CI errors (failed runs in window)
[2×] ^[[1m^[[91merror^[[0m: test failed, to rerun pass `--lib`
[2×] ##[error]process completed with exit code 101.
[2×] error_count: 10
[2×] ##[error]deployment cancelled.
[1×] test result: failed. 4206 passed; 1 failed; 1 ignored; 0 measured; 0 filtered ou

## Provider/API health
10 sessions, no provider errors detected.

## GitHub Actions log feedback
latest score=0.7063 confidence=1.0 recurring_failures=1 state_capture=1.0
Top lessons for next run:
- seeded task was contradicted by fresh assessment evidence -> validate seeded tasks against the fresh assessment before implementation and replace contradicted se
- max task turn count is high: 24 -> split broad tasks earlier or add task-specific context so implementation converges in fewer turns
- fatal: no pattern given -> inspect the failing phase and add a targeted harness guard or eval fixture
Repeated across prior log feedback:
- 3x │ command timed out after 120s
- 2x fatal: no pattern given
- 2x [2×] error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 270/360 proven; 90 unresolved
- missing 38x deepseek_model_call_lifecycle_balanced latest=day-105-20260613T103040Z
- missing 29x state_run_lifecycle_balanced latest=day-105-20260613T103040Z
- observed 23x assessment_artifact_and_transcript_state latest=day-105-20260613T035330Z
task states: verified_landed=12; reverted_no_edit=5; scope_mismatch=4; verifier_unproven=4; reverted_unlanded_source_edits=3
tool failures: search_regex_error=57; search_binary_match=19; missing_file_read=11; read_error=11; bash_tool_error=10
