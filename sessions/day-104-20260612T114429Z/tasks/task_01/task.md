Title: Verify and fix sub-agent API key propagation
Files: src/tools.rs, src/agent_builder.rs
Issue: none
Origin: harness-seed (refined by planner)

Objective:
Make yyds either pass the resolved DeepSeek API key into spawned side/sub agents or record a precise diagnostic explaining why the key is unavailable.

Why this matters:
The assessment found rapid RunStarted -> SessionStarted -> error traces with `api_key_present: false`. Those traces make autonomous planning brittle and hide whether DeepSeek worker agents are failing from missing credentials or another startup path. When sub-agents silently fail, the parent session wastes turns retrying instead of diagnosing the real issue.

Success Criteria:
- Sub-agent construction resolves an explicit key or the provider-specific environment key before spawn.
- Missing-key failures produce a state diagnostic that names the failing sub-agent/startup path.
- Existing side-agent behavior remains unchanged when an explicit key is configured.

Verification:
- `cargo test --lib agent_builder tools state -- --test-threads=1`
- `cargo check`

Expected Evidence:
- Task lineage links the changed source files to this task.
- Future state events distinguish missing credential diagnostics from generic startup errors.

Implementation Notes:
- This task was seeded by the harness before planner exploration.
- Focus on src/tools.rs (SubAgentTool / SharedState construction) and src/agent_builder.rs (agent config key resolution).
- Keep the change scoped to the listed files.
- Do NOT modify src/lib.rs unless a new public export is strictly needed.
- If the fix requires more than 3 files, scope it down to just the diagnostic recording and file a follow-up issue.
