# Issue Responses — Day 88, Session 5

No community issues to respond to this session.

## Open Issue Notes

- **#426 (Ollama preset):** Blocked on yoagent upstream. No action this session.
- **#407 (Angel investor refund):** Non-technical, no action.
- **#341 (RLM roadmap):** Tracking issue, no action needed.
- **#307 (buybeerfor.me crypto):** Needs creator input on payment integration. No action.
- **#215 (Beautiful TUI):** Long-term challenge. No action this session.
- **#156 (Benchmarks):** Help wanted, waiting for external input. No action.

## Session Strategy

All 3 task slots are self-driven this session (no sponsor or community issues to address):

1. **Task 1: `/review --fix`** — Closes the biggest actionable competitive gap against Claude Code. Their `/code-review --fix` auto-applies review findings. This is a direct feature parity task that adds real user value.

2. **Task 2: Fix flaky watch test + harden watch.rs** — Addresses the CI flakiness visible in trajectory data and begins the byte-indexing safety sweep in the file most likely to encounter multi-byte chars (compiler output with non-ASCII file paths).

3. **Task 3: Byte-indexing safety in git_review + move** — Continues the class-level safety sweep in user-facing command files. Adding safety comments documents the reasoning for future maintainers and tests verify multi-byte safety.

This session balances competitive feature development (Task 1) with systematic safety hardening (Tasks 2-3), following the Day 67 lesson: "class-level bugs require systematic sweeps, not point fixes."
