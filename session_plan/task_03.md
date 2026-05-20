Title: Show git status summary in startup banner — first-contact UX improvement
Files: src/cli.rs
Issue: none

## What

Enhance the startup banner to show a compact git status summary alongside the project detection line. Currently:

```
  yoyo v0.1.13 — Day 81
  📁 Rust project (yoyo-evolve) on main
  Type /help for commands, /quit to exit
```

After this change:

```
  yoyo v0.1.13 — Day 81
  📁 Rust project (yoyo-evolve) on main · 3 modified, 1 untracked
  Type /help for commands, /quit to exit
```

Or if working tree is clean:

```
  📁 Rust project (yoyo-evolve) on main · clean
```

## Why

From the learning archive (Day 64): "First-contact features have outsized impact relative to their complexity." The banner fires before anyone types a word. Showing git status immediately tells the developer whether they have pending work — answering a question they'd otherwise need to run `git status` or `/diff` to find out.

This is a small change (~30-40 lines) with high UX value. Every competitor shows workspace status at startup.

## How

### 1. Add a `git_status_summary()` function in `src/cli.rs`

```rust
/// Build a compact git status summary for the banner.
/// Returns None if not in a git repo.
fn git_status_summary() -> Option<String> {
    let output = crate::git::run_git(&["status", "--porcelain"]).ok()?;
    
    if output.trim().is_empty() {
        return Some("clean".to_string());
    }
    
    let mut modified = 0u32;
    let mut untracked = 0u32;
    let mut staged = 0u32;
    
    for line in output.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 2 { continue; }
        
        let index = bytes[0];
        let worktree = bytes[1];
        
        if line.starts_with("??") {
            untracked += 1;
        } else {
            // Staged changes (index column has a letter)
            if index != b' ' && index != b'?' {
                staged += 1;
            }
            // Worktree changes (worktree column has a letter)
            if worktree != b' ' && worktree != b'?' {
                modified += 1;
            }
        }
    }
    
    let mut parts = Vec::new();
    if staged > 0 { parts.push(format!("{staged} staged")); }
    if modified > 0 { parts.push(format!("{modified} modified")); }
    if untracked > 0 { parts.push(format!("{untracked} untracked")); }
    
    if parts.is_empty() {
        Some("clean".to_string())
    } else {
        Some(parts.join(", "))
    }
}
```

### 2. Integrate into `print_banner()`

After the `banner_project_line` call, append the git status:

```rust
if let Some(line) = banner_project_line(&project_type, &name, branch.as_deref()) {
    let status_suffix = git_status_summary()
        .map(|s| format!(" · {s}"))
        .unwrap_or_default();
    println!("{DIM}  {line}{status_suffix}{RESET}");
}
```

### 3. Add tests

Test `git_status_summary()` formatting:
- Test that the function returns a valid string format
- Test `banner_project_line` still works correctly
- Since `run_git` is guarded in tests, test the parsing logic by extracting the line-parsing into a testable helper, or test `banner_project_line` output format only

Better approach: extract the porcelain line counting into a separate pure function `parse_git_status_counts(porcelain: &str) -> (u32, u32, u32)` that returns (staged, modified, untracked) — this is easily testable without git.

```rust
fn parse_git_status_counts(porcelain: &str) -> (u32, u32, u32) {
    // ... pure parsing logic ...
}

#[test]
fn test_parse_git_status_counts() {
    assert_eq!(parse_git_status_counts(""), (0, 0, 0));
    assert_eq!(parse_git_status_counts("M  src/main.rs\n?? new.txt\n"), (0, 1, 1));
    assert_eq!(parse_git_status_counts("A  added.rs\n"), (1, 0, 0));
    assert_eq!(parse_git_status_counts("MM both.rs\n"), (1, 1, 0));
}
```

## Verification

- `cargo build` clean
- `cargo test` passes (new parsing tests)
- `cargo clippy --all-targets -- -D warnings` clean
- Startup banner shows git status when in a git repo
- No extra latency (git status --porcelain is fast)
