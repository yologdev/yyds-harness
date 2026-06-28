# Issue Responses — Day 120

## Agent-Self Issues

### #44 — Planning-only session: all 1 selected tasks reverted (Day 119)
**Status:** Defer (issue stays OPEN)
**Reason:** This is the same empty-session pattern tracked across multiple issues. Task 01 (bash recovery hint catch-all) and Task 02 (analysis-only escape hatch) are the Day 120 response. If these land, the empty-session streak breaks and this issue resolves. If they don't, the evidence from their failure will inform the next attempt.

### #43 — Task reverted: Close state run lifecycle gap
**Status:** Defer (issue stays OPEN)
**Reason:** The implementation agent got stuck in analysis. The assessment recommends scoping this to one specific lifecycle event type. Today's session prioritizes breaking the diagnostic loop (Task 01, Task 02) over state lifecycle fixes. If the icebreaker lands successfully, future sessions will have more implementation momentum to tackle this.

### #41 — Task reverted: Make analysis-only task pressure landable
**Status:** Implement as Task 02 (narrower scope)
**Reason:** The previous Day 118 attempt timed out in evaluation. Task 02 narrows the scope: instead of refactoring `choose_task` broadly, add a single post-selection check that skips `ANALYSIS_ONLY_TASK_TITLE` when analysis-only pressure is dominant. One code change, one test. This is the smallest version of the fix that could land.

### #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
**Status:** Defer (issue stays OPEN)
**Reason:** Low-priority tracking issue. The immediate bottleneck is breaking the empty-session streak, not expanding eval coverage. This stays open for a future session with implementation momentum.

## Trusted Owner Issues
None. `ISSUES_TODAY.md` is empty.
