Verdict: PASS
Reason: The `read_tail_events_capped` function caps event scanning at 20K using VecDeque tail-sampling, matching state doctor's approach. `timeout 10 yyds state crashes --limit 5` completes instantly (0.12s) and prints the expected truncation note + results. `--limit 20` also works. Three new unit tests cover truncation, under-limit, and empty-line handling. Build and tests pass.
