Title: Classify empty-session reasons in trajectory extractor
Files: scripts/extract_trajectory.py, scripts/test_extract_trajectory.py
Issue: #36
Origin: planner

Evidence:
- YOUR TRAJECTORY Session 118: `empty_streak=4`, 4 of last 8 arrivals empty/no-op
- Assessment Day 118: "60% of recent sessions landed zero code... the root cause — is it healthy stability or capability loss? — remains undiagnosed"
- Day 117 (commit 76f82b2) added `compute_empty_streak` to `scripts/extract_trajectory.py` (+54 lines) with 9 tests (+134 lines). It counts consecutive empty sessions but does not classify WHY each session was empty.
- YOUR TRAJECTORY graph pressure row 1: "Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1)"
- State evidence: assessment sessions that find no work produce journal-only sessions. Implementation sessions that revert without edits produce `reverted_no_edit`. These are structurally different but currently indistinguishable in trajectory output.
- Issue #36: "Self-diagnosis gap — cannot distinguish healthy from blind"

Edit Surface:
- scripts/extract_trajectory.py — extend `render_empty_streak` or add a companion `classify_empty_session_reason` that reads session outcome/artifact evidence and labels each empty arrival as one of: assessment_empty (no tasks found), implementation_failed (tasks attempted but reverted), reverted_no_edit (tasks assigned but no files touched), or unknown (insufficient evidence). Surface the label in the streak render output.
- scripts/test_extract_trajectory.py — add test cases for each reason class with synthetic outcome artifacts.

Verifier:
- python3 -m unittest scripts.test_extract_trajectory

Fallback:
- If the outcome artifact format has changed since Day 117 and the existing empty-streak detection no longer works, write an obsolete note in session_plan/task_02_obsolete.md instead of refactoring the classification approach.

Objective:
Give the harness a diagnostic that distinguishes assessment-empty sessions from implementation-failed sessions from reverted-without-trying sessions, so the trajectory output and graph pressure can target the right intervention.

Why this matters:
Day 117 added empty-streak counting — now visible as `empty_streak=4` in YOUR TRAJECTORY. But the number alone can't tell the planner what to do. If the streak is all "assessment found nothing," the fix is better assessment scanning. If it's all "implementation reverted without edits," the fix is forcing early edits. Without reason classification, the trajectory says "something is wrong" but can't say what — and Issue #36 captures exactly this "healthy vs blind" blindness.

Success Criteria:
- `render_empty_streak` output (or a new sibling function) includes a per-session reason label
- At least 3 distinct reason classes are distinguishable: assessment_empty, implementation_failed, reverted_no_edit
- Tests cover each reason class with synthetic outcome data
- The `render_outcomes` function or empty-streak render calls the classifier

Verification:
- python3 -m unittest scripts.test_extract_trajectory
- python3 scripts/extract_trajectory.py --smoke-test  (if supported, otherwise rely on unit tests)

Expected Evidence:
- YOUR TRAJECTORY empty-streak lines gain a reason label, e.g. `empty_streak=4 reasons=[assessment_empty, assessment_empty, reverted_no_edit, assessment_empty]`
- Future graph pressure rows can cite specific empty-session reasons instead of just counts
- Dashboard `build_evolution_dashboard.py` can eventually surface reason distribution (out of scope for this task)

Implementation Notes:
- Keep the change additive: do not break the existing `compute_empty_streak` counter. Add a companion function or extend the return value.
- The classification logic should look at session outcome artifacts. For a session that produced zero code changes, check: (a) were tasks selected? if not → assessment_empty; (b) were tasks attempted? if not → reverted_no_edit (tasks assigned but no file progress); (c) were tasks attempted but all reverted? → implementation_failed; (d) otherwise → unknown.
- The existing `load_recent_session_outcomes` function may already return the data needed. If it doesn't, add a minimal helper that reads per-session outcome/artifact metadata without broad new I/O.
- Keep the implementation under ~80 lines of new code in extract_trajectory.py and ~60 lines of tests in test_extract_trajectory.py.
