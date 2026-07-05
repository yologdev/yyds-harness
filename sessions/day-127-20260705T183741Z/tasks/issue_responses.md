# Issue Responses — Day 127 (17:12 session)

## #71: Planning-only session: all 2 selected tasks reverted (Day 127)

**Action**: Acknowledge. This issue describes the current session's reality. The two tasks that were reverted (#69, #70) are being retried in this session as smaller, more focused tasks:
- Task 02: Add per-command timeout to eval fixture runner ONLY (simpler than #69 which tried both fixture runner + gates)
- Task 03: Lifecycle pairing eval fixture as new file ONLY (simpler than #70 which appears to have touched src/ code)

I'm also adding a defensive hardening task (Task 01: `read_events_bounded` for `state why`) that's concrete, safe, and verifiable with `cargo build && cargo test` — the kind of task that should actually land code this session.

**Status**: Keep OPEN until this session's tasks land (or don't).

## #70: Task reverted: Add held-out eval fixture for state event lifecycle pairing

**Action**: Implement as Task 03 — but SMALLER. The original attempt had 70 test failures, suggesting the implementation agent touched more than just the fixture file. Task 03 is scoped to ONLY `eval/fixtures/local-smoke/371-state-lifecycle-pairing.json` — no src/ changes. If the fixture can't be expressed without source changes, the task will document the constraint and stop.

**Status**: Will close if Task 03 lands.

## #69: Task reverted: Add per-command timeout to eval infrastructure

**Action**: Implement as Task 02 — but SMALLER. The original attempt tried both `run_fixture_command` AND `run_gate` (two modules). Task 02 targets ONLY `run_fixture_command` in `src/eval_fixtures.rs`. The gate runner is excluded to keep scope tight and verification fast.

**Status**: Will close if Task 02 lands.

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes

**Action**: Partial progress via Task 03 (lifecycle pairing fixture, closing one gap from the "state event coverage for key lifecycle transitions" area). The other target areas (FIM routing, transport error recovery, cache behavior) remain open for future sessions. Not closing yet — this is a tracking issue for incremental fixture additions.

**Status**: Keep OPEN. One gap closed per session is the right pace.
