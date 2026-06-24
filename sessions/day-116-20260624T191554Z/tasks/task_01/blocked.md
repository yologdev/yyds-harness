# Task blocked by analysis-only implementation attempt

The implementation agent exited without landing file progress or emitting
TASK_TERMINAL_EVIDENCE on the first attempt. The harness stopped the task
instead of spending a second attempt on more analysis.

This task should be replanned with narrower scope, clearer owning files, or
stronger pre-confirmed evidence before another implementation attempt.

Recent transcript tail:
```
```python
if _has_protected_files(task):
    continue
```

This check is inside the TASKS loop, so it applies to tasks in TASKS. But what about the FALLBACK tasks?

The `no-task-sessions` fallback at line 727 has:
```python
"files": "scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py"
```

And the stable-codebase fallback... let me check.

Actually, let me look at the code flow for when NO TASKS match. Line 727:

```python
return fallback
```

But wait, this returns immediately — it doesn't go through the candidate contradiction check. And it doesn't apply `_has_protected_files`.

But this shouldn't matter because the fallback's files are all scripts/*.py files that aren't in PROTECTED_IMPLEMENTATION_FILES.

OK let me step back and think about what the task is actually asking me to do. The task title is: "Make analysis-only task pressure produce landable implementation tasks"

The key issue: when analysis-only pressure exists, the task selected should be "landable" — meaning:
1. Files list contains no protected files
2. At most 3 source-owned files

The current code already has these checks. But maybe there's an edge case I'm missing.

Let me run the tests to see if they pass:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 25 ────────────────────────────────────────────────────────────╮
  ▶ $ cd /home/runner/work/yyds-harness/yyds-harness && python3 scripts/preseed_sessio… ✓ (210ms)

── Thinking ────────────────────────────────────────────────────────────────
The tests pass. Now let me understand what's really needed here. Let me read the full `choose_task` function flow again, end to end, paying close attention to what happens when `analysis_only_active` is True.

Looking at the flow:
1. Line 672: `analysis_only_active = _has_analysis_only_pressure(metrics)`
2. Lines 674-705: Loop through TASKS, checking:
   - Keys match: `any(key in lower for key in task["keys"])`
   - Reject keys don't match
   - Not recently blocked (for analysis-only task)
   - No protected files
   - Files exist
   - If analysis_only_active and >3 files: skip
   - Lifecycle tasks: skip if analysis_only_active
3. Lines 710-712: If analysis_only_active, sort by `_has_src_files`
4. Lines 714-718: Return first non-contradicted candidate
5. Lines 720-725: If all contradicted, return first with annotation
6. Lines 727-755: If NO candidates, return fallback

So when analysis_only_active is True:
- Tasks with >3 files are skipped
- Lifecycle tasks are skipped
- Candidates are sorted with src-file tasks first
- Protected-file tasks are already skipped

Now let me think about what's MISSING. The task says:
> The implementation should ensure that when analysis-only pressure is detected, the selected task's Files list is validated against PROTECTED_IMPLEMENTATION_FILES before writing.

The bold part is "before writing." Currently, the validation happens during candidate selection (line 686), but NOT at the writing stage (line 1432). So if somehow a task with protected files slips through (maybe via a bug in `_has_protected_files` or via the fallback path), it would be written without validation.

But more importantly, the task wants to add a SECOND validation layer at the writing stage. And it also wants to add a helper in `state_graph_tools.py` for reusing protected-file validation.

Let me look at `state_graph_tools.py` again to see where protected-file validation could be exposed. The `source_file` function at line 328 already does some filtering. But there's no dedicated protected-file checker.

Let me look at what `state_graph_tools.py` might need. Let me search for how `source_file` is used and where protected files are checked in state_graph_tools:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 26 ────────────────────────────────────────────────────────────╮
  ▶ read scripts/state_graph_tools.py:500..560 ✓ (62ms)


```
