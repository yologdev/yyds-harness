# Task blocked by no-progress implementation attempts

The implementation agent used the allowed attempts without landing file progress
or emitting TASK_TERMINAL_EVIDENCE.

This task should be replanned with narrower scope, clearer owning files, or
stronger pre-confirmed evidence before another implementation attempt.

Recent transcript tail:
```

Actually, let me take the simplest approach that actually addresses the task. Let me add:
1. Tracking of `FailureObserved` in `lifecycle_for_scope` 
2. A `closed_by` field in the RunCompleted payload
3. Tests

This way, the post-hoc nature of the closure is explicit in the state events, and `FailureObserved` evidence is preserved.

Wait, but the task says the script already closes open runs. Adding `closed_by` and `FailureObserved` tracking doesn't fix the actual `state_run_incomplete_count` issue. 

Let me re-read the task objective: "Ensure runs orphaned by evaluator timeout get proper RunCompleted terminal events during post-hoc lifecycle closure, so state_run_incomplete_count accurately reflects only truly-open runs."

If the current code already closes open runs, why is `state_run_incomplete_count` still 1?

Maybe the answer is: the evaluator timeout happened in a code path that doesn't go through `run_agent_with_completion_watch` (and thus doesn't call `record_agent_terminal_events`). Let me search for how the evaluator is invoked.

Actually, you know what, I just realized I should check whether there's another code path for the evaluator that doesn't call `record_agent_terminal_events`. Let me search for "eval" in evolve.sh.
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 17 ────────────────────────────────────────────────────────────╮
  ▶ $ cd /home/runner/work/yyds-harness/yyds-harness && grep -n 'eval\|EVAL\|evaluat' … ✓ (351ms)

── Thinking ────────────────────────────────────────────────────────────────
Now I can see the evaluator invocation:

```bash
EVAL_STAGE_NAME="eval_task${TASK_NUM}_attempt${EVAL_ATTEMPT}"
STAGE_NAME="$EVAL_STAGE_NAME" \
    "$EVAL_TIMEOUT" "$EVAL_PROMPT" "$EVAL_LOG" \
    "session_plan/eval_task_${TASK_NUM}.md" '^Verdict:\s*(PASS|FAIL)\b' \
    "--no-auto-watch" || EVAL_EXIT=$?
```

Wait, that's calling a function. Let me look at lines around 2620 to see the full invocation.
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 18 ────────────────────────────────────────────────────────────╮
  ▶ read scripts/evolve.sh:2550..2630 ✓ (144ms)

── Thinking ────────────────────────────────────────────────────────────────
Now I see the full evaluator invocation. The evaluator uses `run_agent_with_completion_watch`, which calls `record_agent_terminal_events` for ALL exit paths (completion, timeout, zero, nonzero). So the evaluator timeout IS handled.

BUT WAIT - what if the evaluator process exits with a verdict file written (e.g., "Verdict: PASS") but the completion pattern `^Verdict:\s*(PASS|FAIL)\b` matches? Then `completion_seen = True`, `kill_process_group(proc, signal.SIGTERM)`, and `sys.exit(0)` is called. Then in bash:

```bash
if [ -f "$completion_marker" ]; then
    record_agent_terminal_events ...
```

`completion_marker` exists because the Python code writes it:
```python
if completion_seen:
    completion_marker.parent.mkdir(parents=True, exist_ok=True)
    completion_marker.write_text("completion_file_matched\n", encoding="utf-8")
```

So `record_agent_terminal_events` is called. The run is closed.

But what if the Python code hits `sys.exit(0)` before writing the marker? Let me check: the marker is written before `sys.exit(0)`. So it's fine.

What if the evaluator process is killed by SIGTERM (from `kill_process_group`) and THEN `sys.exit(0)` is called? The `kill_process_group` sends SIGTERM to the process group. Then `proc.wait(timeout=5)` waits for the process to die. Then `sys.exit(0)`. The marker is written before `sys.exit(0)`. So `record_agent_terminal_events` is called.

OK I'm truly convinced the evaluator timeout path is handled by the existing code. But the task says there's a `state_run_incomplete_count=1`. Let me consider:

Maybe the issue is NOT with the evaluator timeout but with a DIFFERENT kind of orphaned run. The assessment says the cause is "state_incomplete/open_after_SessionStarted". This means a run was started but never completed. The session itself started but never closed.

Let me look at how the session run is created. The session run has `phase: "session"`. `lifecycle_for_scope` explicitly skips session runs. So if the session run is open, `append_terminal_state_events.py` won't close it.

But the session run is the TOP-LEVEL run. It spans the entire session. It should be closed when the session ends. If the session run is open, there's a different gap.

Let me look at where the session RunStarted and RunCompleted are emitted in evolve.sh.
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 19 ────────────────────────────────────────────────────────────╮
  ▶ $ cd /home/runner/work/yyds-harness/yyds-harness && grep -n 'RunStarted\|RunComple… ✓ (350ms)


```
