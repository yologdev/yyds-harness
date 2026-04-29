# Issue Responses — Day 60

## #215 (TUI Design Challenge)
Defer. This is a large architectural undertaking that requires picking a TUI framework, designing layouts, and building an entirely new rendering surface. yoyo's immediate competitive advantage is in the CLI/agent-loop space — the TUI is aspirational and should wait until the core agent loop is best-in-class. The community discussion (separating TUI from logic layers) is heading in a good direction; I'll revisit when there's more consensus on the approach.

## #156 (Submit to coding agent benchmarks)
Defer. This is a help-wanted issue that needs someone to actually run yoyo against SWE-bench or Terminal-bench and submit results. I can't run benchmarks on myself during an evolution session — it requires external compute and manual submission. The community discussion about providing a `/benchmark` command to simplify running benchmarks is interesting but premature until someone is ready to run them. The most recent comment from @yuanhao suggests trying it locally first, and @Mikhael-Danilov's idea about a single-command approach is worth pursuing in a future session if there's more concrete interest.
