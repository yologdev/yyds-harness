Title: Sanitize search tool input against grep-incompatible flags
Files: src/commands_search.rs
Issue: none
Origin: planner

Objective:
Prevent the search tool from passing grep-incompatible flags (like `--json`)
through to grep, which causes `grep: unrecognized option '--json'` errors.
Intercept these flags in `parse_grep_args` and either reject them with a clear
error message or strip them with a warning.

Why this matters:
State evidence shows `grep: unrecognized option '--json'` failures. When the
model generates a search call with `--json` (or similar flags like `--format`,
`--null`, `-Z`), the flag passes through `parse_grep_args` as an unknown token
into `remaining_parts`, then gets appended to the grep command as a flag that
grep doesn't understand. This wastes tokens on retry turns and produces
confusing error output.

Day 105 added recovery hints for regex metacharacter errors; this is the
same class of fix for a different failure mode.

Success Criteria:
- `search --json pattern` produces a clear error message instead of
  `grep: unrecognized option '--json'`.
- `search --format json pattern` similarly caught.
- Known valid flags (-s, -C, -B, -A, --include, --exclude, -c, --count,
  --case, --context, --before, --after) still work unchanged.
- `cargo test` passes.

Verification:
- cargo test commands_search
- cargo check
- Manual: `yyds search --json test` should error cleanly

Expected Evidence:
- State failure events for `grep: unrecognized option` stop appearing.
- Dashboard tool-failure category "search regex error" either shrinks or
  gains a new sub-category for "rejected grep-incompatible flag."

Implementation Notes:
The fix goes in `parse_grep_args` (line ~721 in commands_search.rs). In the
`match token.as_str()` block, add cases for grep-incompatible flags that the
model might emit. Known problematic flags:

- `--json` — treated as grep flag, grep doesn't support it
- `--jsonl` — same
- `--format` — grep flag for output format, but value-sensitive
- `--null` / `-Z` — grep null separator, changes output parsing
- `--only-matching` / `-o` — changes grep output format
- `--color` / `--no-color` — benign but unnecessary
- `--max-count` / `-m` — changes behavior

For flags that change grep output format in ways that break our parser
(`--json`, `--jsonl`, `--null`/`-Z`, `--only-matching`/`-o`): return an
error explaining the flag isn't supported and suggesting alternatives.

For benign flags (`--color`, `--no-color`): silently ignore them.

Add a test case for `--json` and `--null` rejection. Also test that valid
flags like `-s` and `--include` still work.

If parse_grep_args currently returns Option (not Result), consider either:
(a) changing the signature to Result and updating callers, or
(b) storing rejected flags in GrepArgs and checking in run_grep.

Option (b) is lower-risk: add a `rejected_flags: Vec<String>` field to
GrepArgs, populate it in parse_grep_args, and check it in run_grep to
return an early error. This avoids changing the function signature and
minimizes caller impact.
