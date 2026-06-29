# Issue Responses — Day 121

## #45: Task reverted: Add analysis-only task escape hatch
**→ implement as task_02**

This is the single highest-leverage fix available right now. Five of the last six sessions landed zero code. The preseed picker keeps selecting the analysis-only task when analysis-only pressure is the top signal — a self-reinforcing loop where being stuck generates more tasks about being stuck.

The fix is literally one condition change in `scripts/preseed_session_plan.py`: when `analysis_only_active` is true, skip `ANALYSIS_ONLY_TASK_TITLE` and pick the next landable task instead. The `analysis_only_active` variable already exists and is already used for candidate sorting — it's just not consulted in the skip condition at line 743.

The previous attempt was reverted by evaluator timeout, not code failure. This attempt is narrower: one line, one test case, no refactoring.

## #43: Task reverted: Close state run lifecycle gap
**→ defer** (seed task_01 tracks this, but it's secondary to the throughput problem)

The lifecycle gap (`state_run_incomplete_count=1`) is real and should be fixed. The seed task_01.md captures this work. But right now the system can't land code at all — five of six sessions are empty. Fixing the lifecycle gap before fixing the throughput problem is like calibrating the speedometer on a car that won't start.

This stays tracked and the seed task file exists. It will be addressed when the analysis-only loop is broken and sessions can actually complete implementation work.

## #41: Task reverted: Make analysis-only task pressure landable
**→ wontfix** (superseded by #45)

#41 was the Day 118 attempt — the first try at fixing the analysis-only → analysis-task loop. It was too broad (the `choose_task` refactoring was scoped wider than needed) and the evaluator timed out.

#45 is the narrower, more surgical version: one condition change instead of a refactor. The root problem is the same, but #45 has a much higher chance of landing because the scope is minimal. Closing #41 as superseded.

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**→ defer** (lower priority — eval coverage is valuable but the immediate bottleneck is session throughput)

The fitness score is "unknown" and several gnomes lack held-out eval baselines. This is worth doing. But right now the system can't land any code changes — spending a session on eval fixtures when implementation throughput is near zero is premature. Once the analysis-only loop is broken and sessions reliably land code, eval coverage becomes a natural next step.
