Verdict: PASS
Reason: The "summary" match arm was added at line 104-112 with correct `--limit` parsing (consistent with "tail"/"why" arms), calls `handle_state_summary`, and the usage text was updated to include `summary [--limit N] [--tail]`. Build and tests pass. The "why" arm is unchanged. Implementation matches task requirements exactly.
