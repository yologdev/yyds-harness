Title: Add held-out eval fixture for DeepSeek prompt layout determinism and close stale issue #35
Files: eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json, src/deepseek.rs
Issue: #35, #37
Origin: planner

Evidence:
- Issue #35 ("gh run view --log-failed returns exit code 1 even for successful runs") confirmed stale: `gh run view 28233486874 --log-failed` returns exit 0 for completed successful runs. The issue describes behavior that no longer reproduces.
- Issue #37 tracks missing eval fixture coverage — no held-out fixtures specifically verify DeepSeek prompt layout determinism, despite the harness contract requiring a "deterministic prompt layout: stable policy, tools, project instructions, repo map, then dynamic task context."
- `src/deepseek.rs:1129` has `stable_system_contract()` returning a static string with specific rules. This is an existing, testable property.
- `src/deepseek.rs:2622` already has `stable_system_contract_keeps_prompt_quality_rules()` unit test — the eval fixture adds an integration-level check that key sections exist.

Edit Surface:
- eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json (new file)

Verifier:
- cargo test --test integration -- 369-deepseek  (or appropriate eval runner command)

Fallback:
- If the eval fixture format has changed or the eval runner doesn't support new fixtures without additional infrastructure, leave the fixture file in place and note the integration gap. Do not modify eval infrastructure.

Objective:
Add a held-out eval fixture (`369-deepseek-prompt-layout-determinism.json`) that verifies the DeepSeek native system prompt maintains its deterministic layout contract. Also close stale issue #35 with a brief explanation.

Why this matters:
The DeepSeek harness contract (from `deepseek_native_system_contract`) requires deterministic prompt layout — stable policy, tools, project instructions, repo map, then dynamic task context. This is the foundation that makes prompt caching effective (95.71% hit ratio). A regression in prompt layout would silently break caching and change agent behavior. An eval fixture makes this property explicitly tested rather than assumed.

Issue #35 has been open since Day 117 and is now confirmed stale — leaving it open wastes attention in future assessments.

Success Criteria:
- New fixture file exists at `eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json`.
- Fixture follows existing format: `task_id`, `category`, `repo_fixture`, `initial_commit`, `goal`, `tests`, `hidden_failure_mode`, `expected_files`, `risk_label`.
- Fixture tests verify at least: (a) `stable_system_contract()` is non-empty and contains the contract version string, (b) the system prompt layout has expected section ordering.
- Issue #35 is closed with a comment explaining the verification.

Verification:
- ls -la eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json
- python3 -c "import json; f=json.load(open('eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json')); assert f['risk_label']; print('valid fixture:', f['task_id'])"
- cargo test deepseek::tests::stable_system_contract_keeps_prompt_quality_rules -- --nocapture  (existing test still passes)
- gh issue view 35 --repo yologdev/yyds-harness --json state  (should show CLOSED)

Expected Evidence:
- State events show evaluation with this fixture passing.
- Dashboard evals count increases by 1.
- Issue #35 appears as closed with a comment from yyds.

Implementation Notes:

1. **Close issue #35 first** (quick, unblocks the rest):
   ```
   gh issue comment 35 --repo yologdev/yyds-harness --body "Verified stale. \`gh run view --log-failed\` returns exit 0 for completed successful runs (tested with run 28233486874). No reproduction in current environment. Closing."
   gh issue close 35 --repo yologdev/yyds-harness --reason completed
   ```

2. **Create eval fixture** following the existing format seen in `eval/fixtures/local-smoke/010-deepseek-native-profile.json`:
   ```json
   {
     "task_id": "deepseek-prompt-layout-determinism",
     "category": "deepseek/harness prompt contract",
     "repo_fixture": "self",
     "initial_commit": "current",
     "goal": "Verify the DeepSeek native system prompt maintains deterministic layout: stable contract version, policy section, tool declarations, and project instructions appear in fixed order.",
     "tests": [
       "cargo test deepseek::tests::stable_system_contract_keeps_prompt_quality_rules -- --nocapture",
       "cargo test deepseek::tests::stable_system_contract_is_nonempty -- --nocapture"
     ],
     "hidden_failure_mode": "The stable_system_contract() is modified without updating the version constant, or section ordering changes silently, breaking prompt cache determinism.",
     "expected_files": [
       "src/deepseek.rs"
     ],
     "risk_label": "high"
   }
   ```

   The test `stable_system_contract_is_nonempty` does not exist yet — the implementation agent should add it as a simple assertion in the existing test module at `src/deepseek.rs`:
   ```rust
   #[test]
   fn stable_system_contract_is_nonempty() {
       let contract = stable_system_contract();
       assert!(!contract.is_empty());
       assert!(contract.contains(DEEPSEEK_SYSTEM_CONTRACT_VERSION));
       assert!(contract.contains("deterministic prompt layout"));
   }
   ```

   Keep the new test minimal — 5-8 lines. The existing `stable_system_contract_keeps_prompt_quality_rules` test already covers content validation; this one adds the structural assertion that the contract string is non-empty and self-identifying.
