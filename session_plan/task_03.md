Title: Add /skill search command for discovering skills on GitHub
Files: src/commands_skill.rs, src/help.rs
Issue: none

The #1 capability gap is the plugin/skills marketplace. yoyo now has `/skill install gh:user/repo` but no way to DISCOVER skills. This task adds `/skill search <query>` to search GitHub for community-created yoyo skills.

## What to build

A `/skill search <query>` subcommand that searches GitHub for repositories tagged with `yoyo-skill` topic or matching a naming convention, and displays results.

### Implementation

1. **In `src/commands_skill.rs`**:
   - Add `"search"` to `SKILL_SUBCOMMANDS` array
   - Add a `handle_skill_search(query: &str)` function that:
     a. Uses `gh search repos --topic yoyo-skill --limit 20 -- <query>` (via `std::process::Command`) to search GitHub repos with the `yoyo-skill` topic
     b. Falls back to `gh search repos "yoyo-skill" --limit 20 -- <query>` if no topic-based results
     c. Parses the output (tab-separated: owner/repo, description, visibility, updated)
     d. Displays results in a formatted table with repo name, description, and install command hint (`/skill install gh:owner/repo`)
     e. If `gh` CLI is not available, print a helpful error message explaining they need `gh` installed
     f. If no query is provided, show popular/recent yoyo-skill repos (no query filter)
   - Wire the `"search"` subcommand in `handle_skill` to call `handle_skill_search`

2. **In `src/help.rs`**:
   - Add `/skill search` to the skill command help text (find the existing `/skill` help entries and add the search subcommand description)

### Design notes

- This is a read-only operation — it just searches and displays, no state changes
- Uses `gh` CLI which is already a dependency for other features (issues, PRs)
- The `yoyo-skill` topic convention makes skills discoverable without a central registry
- Format output similar to how `/skill list` formats local skills, but for remote ones
- Include the install hint so users can immediately run `/skill install gh:owner/repo`

### What NOT to do

- Don't build a full marketplace or registry
- Don't add caching or persistent state
- Don't modify any other source files
- Don't add any new dependencies
