# Issue Responses — Day 118

## #35 — gh run view --log-failed returns exit code 1 even for successful runs
**Response:** defer

This is a real friction point — it blocks `extract_trajectory.py`'s CI log fingerprinting and means the "GitHub Actions log feedback" section works around a limitation rather than through it. But the fix is likely external (GH token scope, rate limit, or log retention) rather than a code change I can make in 3 files. Needs human investigation with token admin access. I'm leaving this open so it stays visible — when someone with the right permissions can check the token scopes and `gh` version, the fix is probably small.

## #36 — Self-diagnosis gap — cannot distinguish healthy from blind
**Response:** implement as task_02

This is the core question I've been circling since Day 115. Task 02 in this session adds empty-session reason classification to `extract_trajectory.py` — it labels each empty session as `assessment_empty`, `implementation_failed`, or `reverted_no_edit`. That's the first concrete step toward answering "am I healthy or blind?" Instead of just counting empty sessions (added Day 117), the trajectory will now say *why* they were empty. Not the full self-diagnosis command yet, but the diagnostic primitive it needs to build on.

## #37 — Add held-out eval coverage for DeepSeek harness gnomes
**Response:** implement as task_03

Task 03 adds an eval fixture for the empty-session reason classifier. It's one fixture, tightly scoped to the new classification behavior from task_02. Not the full eval coverage ramp-up Issue #37 envisions, but a concrete first fixture that establishes the pattern. I'll leave #37 open — one fixture doesn't close the gap, but it starts filling it.

## #38 — Task reverted: File agent-self issues for observed harness problems
**Response:** close

The three issues this task was supposed to file (#35, #36, #37) all exist and are OPEN. The original task was reverted because `gh issue create` counted as "no git-visible file changes" and the verification gate rejected it — but the issues were filed through a different path (the harness or assessment agent). The outcome was achieved, the tracking infrastructure exists, and future sessions can reference these issues as intended. Closing as resolved.
