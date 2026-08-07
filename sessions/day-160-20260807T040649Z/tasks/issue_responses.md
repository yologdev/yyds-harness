# Issue Responses — Day 160 Planning

## #165: Prevent retroactive FailureObserved for deliberate no-op sessions
**Action: implement as task_01**

This is the most actionable open issue — concrete fix location in `scripts/append_terminal_state_events.py`, clear test surface, and the state doctor confirms the pollution is still live. The previous attempt passed tests but was killed by evaluator timeout. Retrying with identical scope.

## #164: Planning-only session: all 2 selected tasks reverted (Day 159)
**Action: acknowledge, no response needed**

This is a tracking issue auto-filed by the harness when tasks were reverted. The root causes were evaluator timeout and scope mismatch — not code defects. The lesson ("smaller tasks") is already applied: task_01 is a single-function, single-file change.

## #163: Classify planning failures by cause
**Action: defer**

The previous implementation attempt couldn't land code — the agent discovered that "per-phase event metrics" the task assumed existed don't actually exist. This task needs replanning with accurate evidence about what data IS available. Diagnostic work is lower priority than fixing actual data pollution (#165). Will revisit when `scripts/log_feedback.py` is better understood.

## #162: Close lifecycle feedback gaps
**Action: defer**

Reverted for scope mismatch (agent touched `.skill_evolve_counter` and `memory/learnings.jsonl` instead of the planned Python scripts). Also, task_01 already touches `scripts/append_terminal_state_events.py` — two tasks on the same file in one session would risk merge conflicts. Defer to next session.

## #105: Record DeepSeek prompt cache metrics during prompt runs
**Action: blocked — waiting on #90 upstream**

Cannot proceed until yoagent's `Usage` struct carries `cache_read_input_tokens` and `cache_creation_input_tokens`. The yyds-side pipeline is ready (`record_cache_metrics` in state.rs, cache-report command, gnome keys). This is a two-field change in another repo.

## #131: Evaluator timeouts in evolve.sh cause false task reverts
**Action: no progress — do-not-modify territory**

The pattern continues: #165 was reverted because the evaluator timed out on passing code. The fix lives in `scripts/evolve.sh` (either bump timeout or collect early verdicts). I can't touch that file. Still needs human eyes.

## #90: yoagent Usage struct drops DeepSeek cache fields
**Action: no progress — waiting on upstream**

Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens`. The DeepSeek API returns them on every chat completion — the stream-check and fim-complete diagnostic paths prove it. The `deepseek cache-report` command still returns "no DeepSeek cache metrics recorded" each session as a living reminder. Needs someone with yoagent repo access.
