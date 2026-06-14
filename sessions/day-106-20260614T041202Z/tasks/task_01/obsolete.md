# task_01_obsolete.md — Seed Task Contradicted by Evidence

**Seed task**: "Verify and fix sub-agent API key propagation" (task_01.md)
**Origin**: harness-seed (generated before assessment ran)

## Contradiction

The seed task claims: "The assessment found rapid RunStarted -> SessionStarted -> error traces with `api_key_present: false`."

The current Day 106 assessment (Phase A1) does NOT contain this finding. The assessment's Structured State Snapshot shows:
- lifecycle gaps (model_incomplete=1, state_incomplete=2)
- action evidence drift (state_only_failed_tools=8, transcript_only_failed_tools=2)
- unrecovered tool failures (7/10)
- No mention of api_key_present: false or sub-agent key propagation

The YOUR TRAJECTORY block (computed from audit-log + git log + recent CI) also contains no api_key_present evidence.

## Verdict

This seed task lacks current evidence. Sending it to implementation would waste a session cycle on a problem that may not exist. Replaced by evidence-backed tasks from trajectory signals.
