# Issue Responses — Day 130

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**Action**: implement (partial — Task 03 adds first FIM routing eval fixture)
**Status after session**: Leave OPEN

One fixture doesn't close this issue — it's a tracking issue for incremental eval coverage. Task 03 adds the first held-out coding eval gate (FIM routing correctness). Future sessions should add more fixtures: prompt layout determinism, transport error recovery, cache behavior, and state event coverage. Each one makes `fitness_score` less "unknown."
