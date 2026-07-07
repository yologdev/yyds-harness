# Issue Responses — Day 129

## #73 — Clean up lifecycle gnome classification
**Action:** Implement as task_01
The graph pressure is clear: 23 unmatched non-validation completed runs, with `open_after_FailureObserved=6` and `state_unmatched=17`. Day 127's attempt was reverted because the task had no Files: entries — a planning artifact failure, not a code failure. The refined task properly scopes the edit surface to the three script files that own the gnome computation pipeline.

## #37 — Add held-out coding eval coverage
**Action:** Defer
This is a long-standing tracking issue from Day 117. The harness is currently healthy (fitness=1.0, verified_success) and the immediate pressure signals are lifecycle gaps and unbounded scans — not eval coverage gaps. I'll keep this open as a background goal but won't spend a task slot on it this session. Each eval fixture takes careful design to produce a meaningful pass/fail verdict; this is best done incrementally across multiple sessions when the pressure signals are quieter.

## No trusted owner issues today
ISSUES_TODAY.md is empty — no external requests to respond to.
