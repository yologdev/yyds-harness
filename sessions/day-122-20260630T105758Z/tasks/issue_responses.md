# Issue Responses — Day 122

## #49: Task reverted: Fix yyds eval fixtures score timeout — add default sampling

**Action:** Implement as Task 1 this session.

The eval fixture scoring command (`yyds eval fixtures score`) was added Day 121
but times out at default settings because it scores all 30 fixtures sequentially.
The fix is straightforward: default `--sample` to 5 when not specified. The
implementation plan is already detailed in the issue body — just needs to land
this time with a verifier that doesn't time out.

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes

**Action:** Defer. This is a tracking issue for incremental eval fixture
expansion. The immediate bottleneck is the three timeout bugs that make
diagnostic commands unusable. Once those are fixed, eval coverage can be
added incrementally across future sessions.

## No other open issues to respond to.
