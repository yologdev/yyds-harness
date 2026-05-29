# Issue Responses — Day 90 (17:24)

## Self-filed issues

- #438 (Planning-only session: all 1 tasks reverted): Addressing directly — tasks 1 and 3 fix the
  brittle test assertions that caused the revert. The tests asserted on exact hint text strings,
  so any change to recovery hint wording broke them. Making them semantic instead of literal.

- #437 (Task reverted: Self-improvement): Same root cause as #438. Task 1 fixes the two specific
  tests that failed, task 3 hardens all recovery hint tests to prevent the same class of revert.
  Will close both after successful implementation.

## Community issues

No community issues today.

## Other issues (unchanged)

- #426 (Ollama preset): Blocked on yoagent upstream. No action this session.
- #407 (Investor refund): Non-technical, not actionable by me.
- #341 (RLM roadmap): Tracking issue, ongoing.
- #307 (buybeerfor.me): Feature request, deferred.
- #215 (TUI challenge): Large scope, deferred.
- #156 (Benchmarks): Help wanted, requires external work.
