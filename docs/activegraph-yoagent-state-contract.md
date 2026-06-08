# ActiveGraph-Inspired yoagent-state Contract

ActiveGraph is a useful reference architecture, but yyds does not embed it as a
runtime dependency. yyds keeps its Rust/CLI/GitHub Actions architecture and
applies the same core invariant natively:

```text
yoagent-state events.jsonl = source of truth
SQLite projection + dashboard JSON = rebuildable read models
```

## What We Borrow

- **Actions as graph facts.** Tasks, commits, evals, decisions, files, failures,
  model calls, tool calls, and gnome metrics are modeled as state events and
  projected relations.
- **Projection is disposable.** `.yoyo/state/state.sqlite`,
  `state/summary.json`, and the evolution dashboard must be rebuildable from
  recorded session events.
- **Causal chain first.** A useful task record should connect:

```text
task_id -> planned files -> touched files -> commit sha -> eval verdict -> gnome deltas
```

- **Replay/fork thinking.** yyds uses audit-log sessions and git tags, such as
  `baseline`, as lightweight forks for comparison instead of adding a Postgres
  event store.

## What We Do Not Borrow Yet

- No Python ActiveGraph runtime in the product path.
- No Postgres requirement for the harness.
- No replacement of yoagent-state or the DeepSeek-native Rust loop.
- No separate web cockpit source of truth; the static dashboard remains a
  projection over audit-log artifacts.

## Required Session Artifacts

Every evolution session should emit:

- `state/events.jsonl` with yoagent-state canonical events.
- `state/summary.json` rebuilt from those events.
- `tasks/manifest.json` with planner-selected tasks or explicit planning
  failure.
- `tasks/task_XX/task.md` and `tasks/task_XX/decision.json` for each selected
  task.
- Task lineage tying planned files, touched files, source commits, evals, and
  gnome movement together.

If the planner produces no task files, the harness must record planning failure
instead of creating a fake generic task.

## New Native Tools

`scripts/state_graph_tools.py` provides read-only audit graph helpers:

```bash
python3 scripts/state_graph_tools.py replay-check --sessions-dir /path/to/audit-log/sessions
python3 scripts/state_graph_tools.py chain --session-dir /path/to/session
python3 scripts/state_graph_tools.py compare-baseline --sessions-dir /path/to/sessions --baseline previous --candidate latest
python3 scripts/state_graph_tools.py suggest --session-dir /path/to/session
```

These commands are intentionally script-level first: they can run in dashboard
builds, GitHub Actions, and future Rust CLI wrappers without changing the state
source of truth.

## Quality Signals

The graph helpers and dashboard should keep these signals visible:

- `task_manifest_available`
- `task_artifact_coverage`
- `task_spec_quality_score`
- `task_mechanical_verification_rate`
- `task_verification_rate`
- `planner_no_task_count`
- `evaluator_unverified_count`
- `state_replay_integrity_rate`
- `coding_log_score`
- `evolution_friction_count`
- `max_task_turn_count`
- `deepseek_cache_hit_ratio`

These are not vanity metrics. They decide what the next evolution session should
fix.
