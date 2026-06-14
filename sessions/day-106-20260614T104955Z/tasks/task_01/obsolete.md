# task_01_obsolete — Seed Contradiction

**Original seed**: Improve cold-start state failure diagnostics
**Origin**: harness-seed (preseed_session_plan.py)

## Contradiction Evidence

The seed task claims: *"The assessment found `state why last-failure` returning only `no state event found` during fresh-state sessions."*

The fresh Phase A1 assessment contradicts this premise. It reports:

> `state why last-failure` shows an older `read_file` failure targeting `session_plan/assessment.md: No such file or directory` from run `run-1780830016614-137949`. This is a stale diagnostic from a previous assessment phase — not a current bug. The `--limit` hint correctly notes "the most recent 200 events were scanned; the target may be further back."

The command returned a **real diagnostic** with a helpful limit hint — not "no state event found." Additionally, Day 104 already addressed cold-start state diagnostics: "The 04:05 session improved cold-start error in `/state why` to explain what state events are."

## Verdict

**OBSOLETE** — the seed's stated problem is not supported by fresh evidence AND the underlying intent (cold-start diagnostics improvement) was already addressed on Day 104. This task would be a no-op or redundant.

## Replacement

See task_01.md for the replacement task (evidence-backed from fresh assessment).
