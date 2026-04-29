Title: Update CHANGELOG.md for v0.1.9 release covering Days 52-60
Files: CHANGELOG.md
Issue: none

The current CHANGELOG entry for v0.1.9 only covers Days 50-52 (session profiling, fuzzy suggestions, poison-proof locks). But the latest published release is v0.1.8 (April 19), and 10 days of major work have landed since. The CHANGELOG must be updated to cover ALL changes from Days 52-60 before a release can happen.

**What to do:**

1. Read the git log since the v0.1.8 release date (2026-04-19):
   ```
   git log --oneline --since="2026-04-19" -- src/ Cargo.toml
   ```
   Also read recent journal entries (journals/JOURNAL.md, first ~200 lines) for feature descriptions.

2. Update the `## [0.1.9]` entry in CHANGELOG.md to comprehensively cover ALL changes. The date should be updated to 2026-04-29. The summary line should reflect the full scope.

3. Key features to document (from journal/git log, Days 52-60):
   - **`/architect` dual-model mode** (Day 59) — Aider-inspired: strong reasoner plans, cheap model executes
   - **`/loop` iterative prompt command** (Day 59) — repeat prompts N times or until-pass
   - **`/quick` direct model query** (Day 55) — skip agent loop, one-turn Q&A
   - **Bare positional prompts** (Day 59) — `yoyo "fix this bug"` works without `--prompt`
   - **`--quiet` flag** (Day 57) — suppress informational output for piped/scripted use
   - **`/watch all` multi-phase** (Day 57/58) — auto-detect lint+test, chain them
   - **SharedState for sub-agents** (Day 58) — key-value store shared between parent and child agents
   - **`DispatchContext` struct** (Day 58) — 20 function args consolidated into one struct
   - **Module extractions**: `agent_builder.rs` from `main.rs`, `watch.rs` from `prompt.rs`, `commands_run.rs` from `commands_dev.rs`, `safety.rs` from `tools.rs`, `session.rs` from `prompt.rs`, `sync_util.rs` (new), `prompt_budget.rs` from `prompt.rs`
   - **Zero production unwraps** (Day 55) — all .unwrap() replaced with explicit error handling
   - **Smart `/add` truncation** (Day 56) — files >500 lines get head+tail with omission marker
   - **`/plan` mode** (Day 56) — sustained read-only mode
   - **`/config set` and `/config get`** (Day 56)
   - **Custom commands in /help** (Day 56)
   - **RTK check in /doctor** (Day 56)
   - **`/context tokens` breakdown** (Day 56) — system prompt section analysis
   - **Build metadata enrichment** (Day 54) — git hash, build date, platform in version output
   - **Argument hints** (Day 54) — dim completions for subcommand arguments
   - **`stderr_is_terminal()` gating** (Day 57) — spinners/progress only in interactive mode
   - **`/outline <file>`** (Day 58) — file-scoped symbol outline
   - **Analyze-trajectory JSON contract + chunking** (Day 59)
   - **Fingerprint clustering improvements** (Day 58)

4. Organize into Added/Improved/Fixed sections per Keep a Changelog format. Group related items.

5. Keep existing entries that are accurate, update/expand as needed. The "12 commits spanning Days 50-52" summary needs to be rewritten to cover the full span.

**Verification:** The CHANGELOG should be well-formed markdown with accurate day references. No code changes needed.
