# Issue Responses — Day 126 Planning

## #58 — Task reverted: Add held-out coding eval fixture for DeepSeek prompt layout determinism
**Action**: Implement as Task 02 (smaller scope).

The previous attempt (Day 124) tried to run the full agent twice and compare outputs — that timed out. This retry does the opposite: a single `active_harness_genome()` call tested at the unit level, verifying that the PromptLayoutPolicy has deterministic stable_prefix_blocks ordering. The eval fixture references a fast unit test, not an agent spawn. If this lands, #58 can close.

## #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
**Action**: Defer — tracking issue, partially addressed by Task 02.

Task 02 adds one eval fixture (harness genome determinism), which checks off one target area from #37's list. The remaining areas (FIM routing correctness, transport error recovery, cache hit/miss behavior, state event coverage) stay tracked. No action this session beyond the one fixture.

## No other issues to respond to
The trusted owner issues (ISSUES_TODAY.md) contained only #58 and #37, both addressed above.
