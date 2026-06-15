Title: Require strict verifier evidence before counting task success
Files: src/commands_eval.rs
Issue: none
Origin: planner

Objective:
Prevent analysis-only tasks from being counted as successful by requiring concrete verifiable evidence (source file changes, test results, or explicit terminal markers) before the evaluator marks a task as passed.

Why this matters:
The trajectory reports `task_analysis_only_attempt_count=3` — three tasks went through the implementation phase but produced no source changes, yet were counted as attempts. The log feedback corrected lesson is "tasks lacked strict verifier evidence → require bounded verifier evidence before counting task success." Analysis-only tasks that don't produce code waste implementation slots and distort the task_success_rate metric (currently 0.0 because all three were analysis-only). The evaluator must distinguish between "task produced real changes" and "task only analyzed."

Success Criteria:
- Tasks that produce no source file changes AND no terminal evidence marker (`changed`/`obsolete`/`blocked`) are counted as `blocked` or `obsolete`, not as a failed attempt
- The evaluator emits a distinct gnome/metric for analysis-only vs. real-attempt tasks
- Existing eval tests continue to pass
- task_analysis_only_attempt_count decreases in future sessions (real implementation tasks replace analysis-only ones)

Verification:
- cargo test commands_eval -- --test-threads=1
- cargo check
- Manual: run the evaluator against a task artifact that has no file changes — it should report "analysis only, no evidence" rather than "failed"

Expected Evidence:
- EvalResult events show distinct status for analysis-only tasks
- task_analysis_only_attempt_count in trajectory decreases or the metric becomes more precise (e.g., split into analysis_only vs. implemented_no_evidence)
- Task lineage captures the "no evidence" status explicitly

Implementation Notes:
The evaluator lives in src/commands_eval.rs (6,517 lines). The key function to modify is the task verification logic that checks task artifacts (transcript, file changes) and determines pass/fail.

The fix should:
1. After evaluating a task's artifacts, check whether any source file was actually modified (look for FileEdited events in the task's transcript, or check git diff for the task's implementation window).
2. If no source files were modified AND no terminal evidence marker exists in the transcript, classify the task as "analysis_only" or "no_evidence" — not as a success or failure.
3. Emit a distinct EvalResult with status indicating "insufficient evidence" rather than "failed."
4. This should cause the harness to not count the task as a real attempt in the task_success_rate metric.

Look for the task evaluation path that processes task artifacts and produces pass/fail verdicts. The `agent_log_has_terminal_evidence` function (recently hardened in Day 107 10:21 to only recognize exact `changed`/`obsolete`/`blocked`) should be used as one signal. The other signal is actual file modifications from the task's transcript.
