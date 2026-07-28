# Task 03: Classify input-validation model calls separately from unmatched lifecycle completions

Title: Classify input-validation model calls separately from unmatched lifecycle completions
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner (refined from harness-seed lifecycle task — narrowed to one concrete classification fix)

Evidence:
- Trajectory: deepseek_model_call_unmatched_completed_count=103 — graph pressure #2
- Lifecycle causes: model_abnormal/model_completion_without_start=8; 103 unmatched completed events
- Assessment: "103 unmatched ModelCallCompleted events without corresponding ModelCallStarted. This is either a recording gap or a real lifecycle bug in the prompt execution path."
- The `append_terminal_state_events.py` script already classifies some lifecycle events but may be conflating input-validation completions (which are deliberately fire-and-forget) with genuine unmatched completions
- The seed lifecycle task identified: "Pre-agent input-validation exits stay classified separately from non-validation unmatched completions"
- Issue #134 implementation attempt got stuck in broad analysis — this task is narrower and only touches one script file

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 scripts/append_terminal_state_events.py --help (smoke test)
- python3 -c "import scripts.append_terminal_state_events; print('import OK')"

Fallback:
- If input-validation completions are already separately classified, verify with state evidence and mark this task obsolete.
- If the script has no concept of input-validation vs. non-validation classification, add a comment documenting the gap and return — don't restructure the entire script.

Objective:
Ensure that input-validation model call completions (fire-and-forget calls that deliberately don't need matching ModelCallStarted events) are classified separately from genuinely unmatched completions in `append_terminal_state_events.py`, so the `deepseek_model_call_unmatched_completed_count` metric reflects only real lifecycle gaps.

Why this matters:
The trajectory reports 103 unmatched ModelCallCompleted events as graph pressure #2. Some of these may be input-validation completions — model calls made during pre-agent input checking that are intentionally fire-and-forget and don't need lifecycle pairing. If these are counted alongside genuine orphans, the metric is inflated and can't be used to detect real lifecycle degradation. Separating them makes the metric trustworthy and helps focus future lifecycle fixes on actual gaps.

Success Criteria:
- Input-validation model call completions are classified with a distinct label or excluded from the unmatched count
- The unmatched completed count in trajectory/state snapshots becomes more accurate (may not drop immediately if most 103 are genuine orphans, but new runs add fewer false positives)
- The script continues to handle non-input-validation completions correctly (no regression)

Verification:
- python3 scripts/append_terminal_state_events.py --help  (smoke test)
- Manual: check if the script has existing input-validation detection logic and whether it's applied to the unmatched-count query

Expected Evidence:
- Future state snapshots show `deepseek_model_call_unmatched_completed_count` growing more slowly (new runs add only real orphans)
- Lifecycle causes in trajectory distinguish input-validation from genuine unmatched
- Log feedback lessons stop conflating input-validation gaps with real lifecycle gaps

Implementation Notes:
- This is a classification-only change in a single Python script — no Rust code changes, no event schema changes
- Search `append_terminal_state_events.py` for "input_validation" or "is_input_validation" — the concept may already exist
- If input-validation detection exists but isn't applied to the unmatched-count query, apply it
- If input-validation detection doesn't exist, look at `ModelCallCompleted` payload fields that could distinguish input-validation calls (e.g., model_call_id pattern, run_id pattern, token counts of in:0 out:0)
- Zero-token completions (in:0 out:0) are a strong signal of harness-internal or validation calls — consider classifying them separately
- Keep the change scoped to classification labels and filtering — do not change the event emission code in src/state.rs
- This task is script-only verification; the implementation agent should NOT spend more than 10 minutes investigating before either making the change or marking it obsolete
