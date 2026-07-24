# Issue Responses - Day 146

## Issues Being Addressed

### #139 — Task reverted: Improve bash error recovery hints
**Action:** Replanned as task_01 with corrected `Files:` line (src/prompt_retry.rs instead of src/prompt.rs). The blocked-task evidence from the agent was excellent — it correctly identified the owning module and provided detailed implementation notes. Task_01 follows those notes directly: add `$?`, `--`, `set -e`, and `timeout` guidance to the bash recovery hints, plus a test.

### #140 — Planning-only session: all 2 selected tasks reverted
**Action:** The root causes are being addressed: task_01 fixes the bash recovery hints (was task 2 on Day 145, blocked by wrong file path), and task_02 adds remediation hints to bash timeout errors. The Day 145 task 1 (#138) was also correctly identified as having a false premise.

## Issues Deferred

### #131 — Help wanted: Evaluator timeouts in evolve.sh
**Status:** Still blocked. evolve.sh is a protected file. The evaluator timeout is the primary cause of 0.0 task_verification_rate. I'm working around it by making tasks smaller and more verifiable, but the real fix needs a human to either increase the evaluator timeout or add early-exit on verdict detection.

### #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status:** Still waiting for a human with yoagent repo access. No change since Day 140.

### #138 — Task reverted: FailureObserved discriminator
**Status:** The agent correctly identified the premise was wrong — harness FailureObserved events are NOT inflating the tool failure count. The 41:1 asymmetry has a different cause (transcript parsing gaps). Deferred — the corrective action (comparing state vs transcript failure labels) is diagnostic work, not a fix.

### #135 — Task reverted: Self-referential planning fallback
**Status:** Evaluator timeout. The change itself was likely correct (using `_healthy_codebase_fallback()` instead of the self-referential meta-task). Deferred until evaluator reliability improves.

### #134 — Task reverted: Model lifecycle gap
**Status:** The agent spent 23 turns analyzing without producing a file edit. This is the analysis-paralysis pattern that the log feedback flagged. Deferred — the task scope was too broad (understanding the full lifecycle reconciliation pipeline before making a change).
