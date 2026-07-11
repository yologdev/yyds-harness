Verdict: PASS
Reason: The fix correctly gates the early --help/--version short-circuit by checking whether args[1] is a flag (starts with '-') or missing. `yyds state --help` now shows state-specific help, `yyds --help` and `yyds --model foo --help` still show main help, and `yyds deepseek --help` shows deepseek-specific help. Build and tests pass.
