Title: Distinguish cancelled runs from error exits in lifecycle terminal events
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Day 154 10:00 Task 1 already handled input-validation exits (skipping runs with error_detail="empty_input"). The remaining lifecycle noise comes from cancelled runs — sessions killed by external signals (GitHub Actions concurrency, SIGTERM) that leave open run/model-call lifecycles but are not real harness failures.
- `yyds state why last-failure` shows retroactive FailureObserved from Day 154 11:25 session — a cancelled run with exit status "error" but no real failure. append_terminal_state_events.py retroactively flags it as missing FailureObserved.
- 3 of 8 recent completed runs were cancelled in the 17:xx UTC time slot, each ~2.5hr duration. These cancellations pollute the failure signal.
- Trajectory shows session_success_rate=0.0 partly because cancelled sessions count as failures. deepseek_model_call_incomplete_count=1 may also be from a cancelled session where model calls couldn't close.
- Day 154 10:00 Task 2 (src/prompt.rs + src/state.rs) closed model-call lifecycle in panic path going forward, but cancelled runs are externally killed — the Rust panic hook can't fire for SIGTERM. The script-level terminal-event doctor is the right layer to handle this.

Edit Surface:
- scripts/append_terminal_state_events.py — detect cancelled/externally-killed runs and skip them (do not add retroactive FailureObserved or ModelCallCompleted for runs that were killed, not errored)
- scripts/log_feedback.py — recognize cancelled sessions in lifecycle analysis and don't penalize their missing terminal events as harness bugs
- scripts/summarize_state_gnomes.py — exclude cancelled-run lifecycle gaps from gnome counts (or flag them separately as "cancelled" rather than "incomplete")

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback

Fallback:
- If the cancellation detection signal (exit reason, run duration, or GitHub Actions concurrency marker) is not reliably available in the state events, document what was tried and mark the task blocked — do not add fragile heuristics.

Objective:
Stop treating externally-cancelled runs as harness failures. When a session is killed by SIGTERM or GitHub Actions concurrency, the missing terminal events are expected behavior, not bugs. The state doctor and log feedback should recognize this and not add retroactive FailureObserved or penalize lifecycle gaps from cancelled runs.

Why this matters:
Cancelled-session noise distorts three signals at once: FailureObserved count (makes the harness look crashier than it is), session success rate (cancelled ≠ failed), and model lifecycle completeness (SIGTERM prevents ModelCallCompleted, but that's not a code bug). Cleaning this up makes all three signals more trustworthy for task selection and fitness measurement.

Success Criteria:
- append_terminal_state_events.py no longer adds retroactive FailureObserved for runs that were externally cancelled (SIGTERM, GitHub Actions concurrency kill).
- log_feedback.py does not flag cancelled-session lifecycle gaps as "model_call_incomplete" or "run_incomplete" harness bugs.
- summarize_state_gnomes.py either excludes cancelled runs from lifecycle gnome counts or labels them distinctly so they don't inflate "incomplete" metrics.
- Existing tests for input-validation skip (Day 154 Task 1) continue to pass.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- python3 scripts/append_terminal_state_events.py --dry-run (if supported) to confirm cancelled runs are skipped

Expected Evidence:
- Future trajectory shows lower deepseek_model_call_incomplete_count and state_run_incomplete_count from new sessions.
- Cancelled runs in the 17:xx slot no longer produce "retroactive FailureObserved" in `yyds state why last-failure`.
- Dashboard failure signal is cleaner — cancelled runs don't inflate failure counts.

Implementation Notes:
- The detection signal for "cancelled" vs "errored" runs: check the run's error_detail, exit reason, or duration. A run killed by SIGTERM typically has no error_detail (or "signal: 15") and was mid-execution. A genuinely errored run has a specific error_detail like "tool_call_failed" or "api_error".
- Day 154 10:00 Task 1 already added input-validation classification in append_terminal_state_events.py — use the same pattern (a helper function that classifies the run's exit reason) and extend it with a "cancelled" category.
- The Rust panic hook (src/prompt.rs, src/state.rs) cannot fire for external kills — this is inherently a script-level post-hoc fix. Acknowledge this in comments.
- Keep changes minimal: add one classification helper, one skip condition, and update log_feedback's lifecycle analysis to check for the cancelled classification.
