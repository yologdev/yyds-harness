# Task 1 Obsolete Note

**Task**: "Make analysis-only task pressure landable"
**Day**: 118 (2026-06-26 03:50)
**Decision**: OBSOLETE — task success criteria already satisfied by current code

## Evidence

All three success criteria are mechanically verifiable in the current codebase:

### Criterion 1: Graph-derived analysis-only/no-edit pressure selects a concrete seed before lifecycle cleanup
`scripts/preseed_session_plan.py` lines 692-695: when `analysis_only_active` is True, the lifecycle task is explicitly skipped even when `lifecycle_metrics_present` is True. Confirmed by test at lines 995-1006 which asserts analysis-only task wins over lifecycle task when both pressures exist.

### Criterion 2: The selected seed Files list contains no protected implementation files
`scripts/preseed_session_plan.py` lines 686-687: `_has_protected_files(task)` runs on every candidate before selection. `PROTECTED_IMPLEMENTATION_FILES` (lines 30-36) excludes evolve.sh, format_issues.py, build_site.py, self-assess/SKILL.md, evolve/SKILL.md. The analysis-only task's own Files field (`scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py`) contains none of these. Confirmed by: `assert not _has_protected_files(task)` at lines 1023 and 1073.

### Criterion 3: Preseed self-tests cover the analysis-only/no-edit pressure path
`scripts/preseed_session_plan.py` tests at lines 983-1093 cover:
- `reverted_no_edit` alone → analysis-only task (line 991)
- `reverted_no_edit` + lifecycle metrics → analysis-only task, lifecycle skipped (line 1004)
- `task_analysis_only_attempt_count` → file-count ≤3 guard (line 1020)
- Protected-files guard on selected analysis-only task (line 1023)
- Evidence-aware re-ranking: src-file candidate wins over analysis-only (line 1040)
- When only analysis-only task matches keys, it's still selected (line 1059)
- `task_no_edit_revert_count` alone → analysis-only task (line 1070)
- Recently blocked analysis-only seed is not re-selected (line 1091)

### Verification
```
$ python3 scripts/preseed_session_plan.py --test
preseed_session_plan self-tests passed
```

## Why This Task Cannot Land Further Changes

The task itself is a meta-task about task-selection quality. When the preseed selects it (because analysis-only pressure metrics are positive), the implementation agent receives a circular self-referential assignment: "fix the task selection system that just selected you." The code already satisfies all three success criteria, so there is no honest code change to make.

The operational gap — implementation agents exiting without file progress on meta-tasks — is a harness-implementation-loop problem, not a preseed task-selection problem. The preseed correctly selects a landable seed with non-protected files; it cannot control whether the implementation agent can execute on it.

## Recommendation

The `_analysis_only_seed_recently_blocked` guard (line 630-648) already prevents re-selection when the analysis-only task failed in a recent session. This is the correct defense-in-depth: the task fires once to attempt the fix, and if it can't land, the guard prevents it from cycling. A future session with fresh evidence (e.g., a different task producing analysis-only pressure) would select a different landable task via the evidence-aware re-ranking path (lines 1027-1048).

No source changes needed. The preseed logic is correct. The task's success criteria are satisfied.
