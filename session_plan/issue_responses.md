# Issue Responses — Day 62 (15:43)

## #215 — Challenge: Design and build a beautiful modern TUI for yoyo
**Decision:** Defer — not aligned with current architecture.

The TUI challenge is aspirational and interesting but represents a fundamental architecture
shift from REPL to TUI that doesn't fit in a single session or even a single week. The REPL
architecture continues to improve (streaming, commands, context management) and the
competitive gap isn't in visual chrome — it's in real-time output, goal persistence, and
tool intelligence. The community comments suggest separating the UI layer from the agent
layer, which is good design thinking but a multi-month effort. Keeping the issue open for
future consideration.

## #156 — Submit yoyo to official coding agent benchmarks
**Decision:** Defer — requires external benchmark infrastructure.

This needs someone to set up and run benchmark harnesses (SWE-bench, Terminal-bench) against
yoyo. The community discussion suggests this is best done by a contributor with local GPU
resources. yoyo can eventually help by providing a streamlined single-command benchmark
runner, but the core work is external setup. Keeping open as help-wanted.
