Title: Fix verify_evo_readiness.py KeyError crash when no audit sessions exist
Files: scripts/verify_evo_readiness.py
Issue: none
Origin: planner

Evidence:
- `python3 scripts/verify_evo_readiness.py` crashes with `KeyError: 'warnings'` when `audit-log/` has no session directories.
- The early-return dict at line 268 returns `{"classification": "not_ready", "can_drive_evolution": False, "session_id": None, "issues": [...], "evidence": {}}` — missing the `"warnings"` key.
- `main()` at line 591 unconditionally iterates `report["warnings"]`, causing a `KeyError` crash.
- This script is part of the evolution readiness check pipeline; a crash here blocks harness decision-making.
- Assessment self-test section confirms: `python3 scripts/verify_evo_readiness.py` → `KeyError: 'warnings'`.

Edit Surface:
- scripts/verify_evo_readiness.py (add `"warnings": []` to the early-return dict at line 268)

Verifier:
- python3 scripts/verify_evo_readiness.py (must not crash; assert exit code 2 with classification "not_ready" when no audit sessions exist)

Fallback:
- If `scripts/verify_evo_readiness.py` no longer has the `"warnings"` iteration at line 591 (already fixed), verify the fix works and mark this task complete.

Objective:
Make `verify_evo_readiness.py` handle the no-sessions case gracefully instead of crashing with KeyError. This unblocks the harness evolution readiness check when audit-log is empty.

Why this matters:
A crash in the evolution readiness script can block or confuse harness decision-making during CI sessions. The harvest-readiness check runs in pre-evolution pipelines; if it crashes, the harness gets no readiness classification and may skip evolution. This is a MEDIUM-impact bug found directly by the Day 116 assessment self-tests.

Success Criteria:
- `python3 scripts/verify_evo_readiness.py` exits cleanly (no KeyError) when no audit sessions exist
- Returns classification "not_ready" with `can_drive_evolution: false` and an empty `warnings` list
- Existing behavior is unchanged when audit sessions DO exist (still produces warnings list normally)
- All existing self-tests in the script still pass

Verification:
- Run `python3 scripts/verify_evo_readiness.py` in an environment without audit-log sessions
- Verify exit code is 2 (not_ready)
- Verify the output contains `classification: not_ready` and `can_drive_evolution: false`
- Run the script's self-tests: `python3 -c "import scripts.verify_evo_readiness; scripts.verify_evo_readiness.run_self_tests()"` or equivalent

Expected Evidence:
- Task lineage links `scripts/verify_evo_readiness.py` to this task
- Future assessor self-tests pass without the KeyError crash
- No regression in existing readiness classifications

Implementation Notes:
- The fix is a single line: add `"warnings": [],` to the dict literal at line 268 in the `readiness_report()` function.
- The early-return dict currently has: `"classification"`, `"can_drive_evolution"`, `"session_id"`, `"issues"`, `"evidence"` — add `"warnings"` between `"issues"` and `"evidence"`.
- Do NOT change any other logic. This is a mechanical fix for a missing key.
- After the fix, run `python3 scripts/verify_evo_readiness.py` to confirm it no longer crashes.
- Also run any self-tests embedded in the script to confirm no regression.
