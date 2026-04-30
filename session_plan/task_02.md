Title: Update gap analysis to reflect skill ecosystem and recent competitive changes
Files: CLAUDE_CODE_GAP.md
Issue: none

## What to do

The gap analysis document (`CLAUDE_CODE_GAP.md`) is stale. The priority queue still lists "Plugin / skills marketplace" as gap #1, but `/skill install` (local + remote via `gh:user/repo`), `/skill search` (GitHub discovery), and `/skill create` all shipped on Days 60-61. The document also needs stats updated (48 source files, ~60K lines, 89 tests, etc.).

### Specific changes:

1. **Move "Plugin / skills marketplace" from priority queue to "done" section.** Note what shipped:
   - `/skill install <dir>` for local skill directories (Day 60)
   - `/skill install gh:user/repo` for remote GitHub skills (Day 61)
   - `/skill search <query>` for GitHub skill discovery (Day 61)
   - `/skill create` for scaffolding new skills (existing)
   - `/skill list`, `/skill show`, `/skill enable/disable` for management
   - Still missing vs Claude Code: signed bundles, curation/ratings, formal marketplace with reviews. Note this as a remaining sub-gap but demote from #1 priority.

2. **Update the priority queue.** After removing the plugin gap, re-rank:
   - #1: Real-time subprocess streaming (was #2)
   - #2: Persistent named subagents with orchestration (was #3)
   - #3: Graceful degradation on partial tool failures (was #4)
   - Add new gap if warranted: "Skill marketplace curation" (signed bundles, ratings, reviews) as #4

3. **Update the "Recently completed" section** to include Day 54-61 work:
   - Day 54-55: safety.rs extraction, version metadata enrichment, `/quick` command
   - Day 56: custom commands in /help, system prompt sections in /context tokens, RTK in /doctor
   - Day 57: watch multi-phase (lint→fix→test→fix), auto-detect watch commands
   - Day 58: SharedState wiring, analyze-trajectory JSON contract, main.rs→agent_builder.rs extraction, watch.rs extraction
   - Day 59: positional CLI arguments, `/architect` dual-model mode, `/loop` iterative refinement
   - Day 60: `/skill install` (local), CHANGELOG, config.rs extraction
   - Day 61: x-research skill, commands_skill.rs extraction, `/skill install gh:user/repo`, `/skill search`, explore-codebase skill, dispatch_sub.rs extraction

4. **Update source stats** at the top or wherever counts appear:
   - 48 source files (was 38 or similar)
   - ~59,794 lines across src/
   - 89 test functions (was ~50)
   - 12 skills (7 core, 5 yoyo-origin)
   - 68+ slash commands
   - 32+ shell subcommands

5. **Update competitive landscape section** if not already current with the Day 59 refresh.

### Verification:
- No code changes, so no build/test needed, but confirm the file reads coherently after edits.
