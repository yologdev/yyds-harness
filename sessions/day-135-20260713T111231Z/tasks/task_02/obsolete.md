# Task 02 Obsolete — Recovery hints already implemented

**Date:** Day 135 (2026-07-13 11:12)  
**Title:** Add bounded-command and path-verification recovery hints to bash tool failures  
**Verdict:** Obsolete — both requested hint categories already exist with full coverage.

## Evidence of existing coverage

### 1. Bash: explicit paths, bounded commands, `$?` inspection

**`tool_recovery_hint` (src/prompt_retry.rs):**

- **Attempt 1** (lines 138–146): Instructs to inspect exit code and stderr, verify file paths with `test -f <path>` / `ls <dir>` / `rg --files | head`, and retry with bounded commands via `head -n 50` / `tail -n 20`.

- **Attempt 2+** (lines 126–132): Escalates to "simpler bounded command with explicit absolute paths", "check exit output first", "verify paths exist with `ls` or `test -f`", "avoid unbounded/recursive commands".

**`targeted_recovery_hint` (src/tool_wrappers.rs):**

- **Exit-code branch** (lines 1033–1042): "Use explicit paths like `./script.sh` to avoid PATH ambiguity, and check `$?` immediately after the failing command to understand the exit code. Start multi-command scripts with `set -e` (and `set -o pipefail` for pipelines) to fail fast on the first error. Retry with `set -x` at the start of your command to trace each step. Inspect both stdout AND stderr output before retrying… Add bounded limits: pipe through `head -n 50`, `tail -n 20`…"

- **Generic fallback** (lines 1097–1104): "Check `$?` immediately for the exit code, use explicit paths (`./script`, not `script`), and retry with a simpler bounded command. Break pipelines into individual steps to isolate the failure."

- **Additional targeted patterns**: timeout (lines 1048–1053), spawn failure (lines 1054–1060), no-such-file (lines 1061–1067), permission denied (lines 1068–1074), command not found (lines 1075–1081), E2BIG (lines 1082–1088), broken pipe (lines 1089–1095).

**Integration test** (src/tool_wrappers.rs, lines 3179–3213): `test_recovery_hint_with_targeted_hint_appended` verifies the combined output includes `$?`, `set -x`, `stdout`, `stderr`, and `head -n` guidance.

### 2. read_file / search: path verification and symbol-search fallback

**`tool_recovery_hint` (src/prompt_retry.rs):**

- **`read_file` attempt 1** (lines 157–162): "run `rg --files | grep <name>` … or use `list_files` on the parent directory to see what's actually there."

- **`read_file` attempt 2+** (lines 108–112): "run `rg --files` to list all tracked files and find the exact path, then use `cat <exact-path>` or `head -n 100 <exact-path>` to read it. Or use `rg -n '<symbol>' src/` to find which file defines the symbol you need."

- **`search` attempt 1** (lines 163–169): "run `rg --files | grep <name>`. For symbol-to-file mapping, try `rg -n '<symbol>' src/`. Then retry with a verified path."

- **`search` attempt 2+** (lines 114–117): "use `grep -rn '<pattern>' <path>` for regex search, or `find . -name '<pattern>'` for file name search."

**`targeted_recovery_hint` (src/tool_wrappers.rs):**

- **read_file / write_file / edit_file / list_files** (lines 1123–1137): handles "no such file or directory" with "Use `list_files` to discover the correct path".

### 3. Wiring into the tool execution path

`RecoveryHintTool` (src/tool_wrappers.rs, lines 1146–1207) wraps every tool execution. On failure:
1. `tool_recovery_hint` provides the generic hint (with attempt-based escalation)
2. `targeted_recovery_hint` appends a pattern-specific hint when applicable
3. Both are appended after `💡 Recovery hint:` and `🎯 Targeted hint:` markers

Wired in `src/tools.rs` via `with_recovery_hints()` for all core tools including bash, read_file, search, edit_file, write_file, rename_symbol, list_files (lines 1321–1391).

## Existing tests

- `test_targeted_recovery_hint_bash_exit_code` (line 3115): verifies exit-code-specific advice
- `test_targeted_recovery_hint_bash_non_exit_code_error` (line 3140): verifies generic bash hint fallback
- `test_targeted_recovery_hint_search_regex_error` (line 3153): verifies regex-specific advice for search
- `test_targeted_recovery_hint_search_non_regex_error_returns_none` (line 3173): verifies non-regex search errors get no targeted hint (generic hint still fires)
- `test_targeted_recovery_hint_read_file_no_such_file` (line 3164): verifies file-not-found advice for read_file
- `test_recovery_hint_with_targeted_hint_appended` (line 3179): integration test proving both hint layers fire together
- `test_recovery_hint_no_targeted_for_non_matching_error` (line 3215): proves targeted hint gracefully absent for non-matching errors

All 87+ tests pass.

## Conclusion

The task requested two recovery hint categories. Both are already fully implemented across `tool_recovery_hint` (generic escalation) and `targeted_recovery_hint` (pattern-specific), wired via `RecoveryHintTool` for all core tools, with 9 dedicated tests. No code changes needed.
