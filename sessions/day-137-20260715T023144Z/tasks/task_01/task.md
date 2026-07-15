Title: Record DeepSeek prompt cache metrics during prompt runs
Files: src/prompt.rs, src/deepseek.rs, src/state.rs
Issue: none
Origin: harness-seed
validated_against_assessment: true

Evidence:
- Current assessment matched this harness seed: The assessment found `deepseek cache-report` returning no metrics even though the DeepSeek protocol layer and gnome keys exist. Cache observability is required to optimize stable-prefix prompt layout and cost/latency.

Edit Surface:
- src/prompt.rs, src/deepseek.rs, src/state.rs

Verifier:
- cargo test deepseek prompt state

Fallback:
- If current assessment, source, or recent changes show this failure class is already fixed or no longer live, write an obsolete-task note instead of editing.

Objective:
Ensure successful DeepSeek prompt executions record prompt cache hit/miss token usage into yoagent-state so `deepseek cache-report` and gnome KPIs have real data.

Why this matters:
The assessment found `deepseek cache-report` returning no metrics even though the DeepSeek protocol layer and gnome keys exist. Cache observability is required to optimize stable-prefix prompt layout and cost/latency.

Success Criteria:
- Prompt usage with cache hit/miss tokens emits CacheMetricsRecorded state events.
- `deepseek cache-report` can read those events after a DeepSeek run.
- No request-side `cache_control` is added for DeepSeek.

Verification:
- cargo test deepseek prompt state
- cargo check

Expected Evidence:
- State summary includes DeepSeek cache hit/miss token gnomes after a run with usage data.
- Dashboard cache ratio remains sourced from numeric usage/state events, not prose.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 137 (02:31); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
