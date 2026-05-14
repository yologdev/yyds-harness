Title: Fuzzy patch application — fallback strategies when strict git apply fails
Files: src/commands_file.rs
Issue: none

## Context

The assessment identifies "Smart apply/patch — `/apply` works but less robust than Claude Code's fuzzy matching" as a 🟡 partial gap. Currently `apply_patch` shells out to `git apply` with no fallback. If the patch doesn't apply cleanly (e.g., context lines shifted by a few lines), it fails with no recovery attempt.

## What to do

Upgrade `apply_patch` in `commands_file.rs` to attempt multiple strategies in sequence when strict `git apply` fails:

1. **First attempt:** `git apply <path>` (current behavior, strict)
2. **Fallback 1:** `git apply --3way <path>` (uses 3-way merge, handles many context drift cases)
3. **Fallback 2:** `git apply -C1 <path>` (reduce context lines required to 1, handles moderate drift)
4. **Fallback 3:** `git apply --recount <path>` (handles patches with wrong line counts)

The function signature stays the same. When a fallback succeeds, the output message should mention which strategy worked (e.g., "Patch applied with relaxed matching (--3way)") so the user knows it wasn't a perfect match.

For `check_only` mode, use the same cascade but with `--check` added.

Also apply the same cascade to `apply_patch_from_string` since it delegates to `apply_patch`.

## Tests to add

- `test_apply_patch_fallback_strategies` — create a temp git repo, create a file, commit it, modify the file (shift lines), create a patch against the old version, verify that the cascading apply succeeds
- `test_apply_patch_check_only_fallback` — same but with check_only=true
- `test_apply_patch_reports_fallback_strategy` — verify the output message mentions the fallback method used

## What NOT to do

- Don't change the function signature
- Don't add new dependencies
- Don't touch any other files
- Keep the existing behavior when strict apply succeeds (no regression)
