Title: Fix DeepSeek model lifecycle gaps (model_completion_without_start)
Files: src/prompt.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Eliminate `model_completion_without_start` lifecycle gaps so every ModelCallCompleted event has a matching ModelCallStarted in the same run, making state lifecycle analysis and caching metrics reliable.

Why this matters:
The trajectory reports `deepseek_model_call_abnormal_completed_count=2` with lifecycle cause `model_completion_without_start=2`. These are ModelCallCompleted events found without a preceding ModelCallStarted event in the same run. This corrupts downstream scoring, cache metrics, and lifecycle imbalance detection in `build_evolution_dashboard.py` and `state_graph_tools.py`. The model call lifecycle is the foundation for per-turn cost observability and DeepSeek cache behavior analysis.

Success Criteria:
- `model_completion_without_start` count drops to 0 in future trajectory/dashboard runs
- Every ModelCallCompleted emitted by `handle_prompt_events` has a model_call_id payload field that matches the ModelCallStarted in the same run
- The lifecycle_cause function in state_graph_tools.py can pair start/completion events by model_call_id instead of relying solely on run-scoped ordering
- Existing tests in state.rs (the sqlite_projection_links_model_and_tool_call_nodes test) continue to pass

Verification:
- cargo test state -- --test-threads=1
- cargo test prompt -- --test-threads=1
- cargo check
- After the fix, `yyds state graph lifecycle` should show 0 model_completion_without_start instances (requires a completed run to verify)

Expected Evidence:
- State events show model_call_id in both ModelCallStarted and ModelCallCompleted payloads
- Lifecycle imbalance detection in build_evolution_dashboard.py can pair events by model_call_id
- Trajectory next session shows deepseek_model_call_abnormal_completed_count decreasing

Implementation Notes:
The root cause is that `handle_prompt_events` in src/prompt.rs emits ModelCallStarted (line 767) and ModelCallCompleted in three places (AgentEnd at line 922, ctrl_c at line 993, loop exit at line 1033), but none of these include a model_call_id for explicit pairing. The Python analysis scripts (`state_graph_tools.py`, `build_evolution_dashboard.py`) detect `model_completion_without_start` when a ModelCallCompleted event exists without a preceding ModelCallStarted in the same run — likely because events span run boundaries or the event store has orphaned completions.

Fix approach:
1. In src/prompt.rs: Generate a unique model_call_id (e.g., using a counter or UUID) at the start of handle_prompt_events, include it in both the ModelCallStarted payload and all three ModelCallCompleted payload sites.
2. In src/state.rs: Update the existing test at line 3690 (sqlite_projection_links_model_and_tool_call_nodes) to verify model_call_id is present in the test fixture payloads.
3. Optionally: Update state_graph_tools.py's lifecycle_cause function to check for model_call_id pairing before falling back to run-scoped ordering. But keep the source change minimal — focus on src/ files.

The model_call_id should be a simple format like "mc-{counter}" where counter is an AtomicU64 in the state module, or use a timestamp-based id. Keep it simple and testable.
