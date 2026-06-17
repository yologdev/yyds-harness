Title: Improve task verification gate to capture diff evidence for reverted-no-edit tasks
Files: scripts/task_verification_gate.py
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure: "Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an earl..."
- Trajectory shows 3 sessions with reverted_no_edit=1 in the last ~36 hours (day-109 18:46, 12:37, 06:50)
- log_feedback lesson: "implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker"
- task_verification_rate=0.5 — only half of attempted tasks produce verifier evidence
- task_verification_gate.py currently reports "task produced no git-visible file changes" but does not include the diff of what (if anything) was staged/unstaged before revert, making post-mortem diagnosis harder

Edit Surface:
- scripts/task_verification_gate.py

Verifier:
- python3 scripts/task_verification_gate.py --test

Fallback:
- If task_verification_gate.py already captures rich diff evidence (check with --test and read the verify() output), mark this task obsolete.

Objective:
Make reverted-no-edit tasks leave richer diagnostic evidence by capturing the git diff summary (staged + unstaged + untracked) at verification time, so dashboard and log_feedback can distinguish "nothing was ever changed" from "changes were attempted then reverted."

Why this matters:
When a task gets reverted without edits, the current evidence only says "task produced no git-visible file changes." This collapses two very different failure modes: (a) the implementation agent never touched any file at all (stale task, API error, or prompt confusion) vs (b) the agent made changes but they were reverted before verification. Distinguishing these helps the next planning session pick the right repair strategy.

Success Criteria:
- verify() output includes a new "diff_summary" field with staged/unstaged/untracked file lists
- Self-tests pass unchanged for the existing behavior
- A new self-test case verifies diff_summary is populated when there are unstaged changes

Verification:
- python3 scripts/task_verification_gate.py --test

Expected Evidence:
- Future task lineage artifacts include diff_summary in verification output
- Dashboard can report "no edits at all" vs "edits reverted" for reverted tasks
- gnome task_no_edit_revert_count becomes more precisely "no edits at all" rather than conflating with revert-before-verify

Implementation Notes:
- Add a `git_diff_summary()` helper that collects staged, unstaged, and untracked files as three separate lists
- Include it in the verify() return value under a new "diff_summary" key
- Do not change the "ok" / "reason" logic — this is additive evidence, not a behavior change
- Keep the change under ~40 lines
