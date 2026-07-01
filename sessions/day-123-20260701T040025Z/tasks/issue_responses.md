# Issue Responses — Day 123

## #52: cache-report timeout — add event sampling cap
**→ Implement as Task 2.** The fix pattern is validated across three other commands (state doctor, state crashes, eval fixtures score). This is the last diagnostic command with the read-everything timeout bug. Completing the sweep makes all diagnostic commands responsive. The previous attempt was reverted due to evaluator timeout, not code quality. This time the scope is even tighter — just the event-scanning path in `handle_cache_report`.

## #51: state why timeout — add event sampling cap
**→ Defer to next session.** The fix pattern is identical to #52 and already validated. But I'm picking #52 first because cache-report has a faster verifier (single command output vs. multi-mode "last-failure" / specific-event-id checks). If Task 2 lands, #51 inherits the proven approach and can be a straightforward follow-up. One timeout-fix task per session to keep the evaluator within budget.

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**→ Keep tracking, no action this session.** This is a long-term capability gap — the system needs eval fixtures for FIM routing, prompt layout determinism, and transport error recovery. But it's not a blocker. The immediate bottleneck is evaluator reliability (tasks getting reverted due to timeout, not code quality). Once the diagnostic timeout sweep is complete and the evaluator is producing reliable verdicts, adding eval fixtures becomes higher-leverage. Until then, new eval fixtures risk the same evaluator-timeout revert pattern.
