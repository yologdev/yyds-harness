Title: Add remote skill install from GitHub repositories
Files: src/commands_skill.rs, src/help.rs
Issue: none

Add support for `yoyo skill install gh:user/repo` (and `gh:user/repo/path`) to install skills directly from GitHub repositories. This is the #1 strategic gap identified in competitive analysis — skills/plugin discoverability and remote installation.

## Current state
`/skill install <path>` only supports local directories. The function `skill_install()` validates a local path, checks for SKILL.md, and copies to `~/.config/yoyo/skills/`.

## What to build

Add a `skill_install_from_github()` function that:

1. **Parse the source string**: detect `gh:` prefix
   - `gh:user/repo` → clone full repo, look for `SKILL.md` at root or in a `skill/` or `skills/` subdirectory
   - `gh:user/repo/path/to/skill` → clone and extract just that subdirectory
   - `gh:user/repo@branch` → optionally support a branch specifier (nice to have)

2. **Download**: Use `git clone --depth 1` into a temp directory (don't require the user to have the repo locally)

3. **Validate**: Same validation as local install — the target directory must contain a SKILL.md with valid YAML frontmatter

4. **Install**: Copy to `~/.config/yoyo/skills/<skill-name>/` using the existing `skill_install_to()` infrastructure

5. **Cleanup**: Remove the temp clone directory

## Error handling
- Git not available → clear error message
- Repo doesn't exist → clear error message
- No SKILL.md found → list what was found, suggest the right path
- Network failure → clear error message

## Update help text
In `src/help.rs`, update the `/skill` help entry to document:
```
/skill install <path>           Install a skill from a local directory
/skill install gh:user/repo     Install a skill from a GitHub repository
```

## Implementation approach

In `skill_install()` (now in `commands_skill.rs` after task 02):
```rust
fn skill_install(source: &str) {
    if source.starts_with("gh:") {
        skill_install_from_github(source);
    } else {
        // existing local install logic
    }
}
```

New function `skill_install_from_github(source: &str)`:
1. Parse `gh:user/repo[/path][@branch]`
2. Create temp dir
3. `git clone --depth 1 [--branch branch] https://github.com/user/repo.git tempdir`
4. Find SKILL.md in the expected location
5. Call `skill_install_to(skill_dir_path, install_dir)` to reuse existing logic
6. Clean up temp dir

## Tests
Add tests for:
- `gh:` prefix parsing (unit test, no network)
- Invalid formats (missing user, missing repo)
- Integration with existing `skill_install_to()` using a temp directory with a mock skill

## Verification
```bash
cargo build && cargo test
cargo clippy --all-targets -- -D warnings
```
