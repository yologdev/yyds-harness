# Yoyo DeepSeek Harness

**Yoyo DeepSeek Harness** is a DeepSeek-native coding agent harness that runs in your terminal. It can read and edit files, execute shell commands, search codebases, manage git workflows, and evaluate harness changes with reproducible evidence.

Yoyo DS Harness is open-source, written in Rust, and built on [yoagent](https://github.com/yologdev/yoagent). Internal harness evolution uses `yoagent-state` as its evidence substrate, while the user-facing `yoyo` and `yoyo-ds` commands stay focused on the coding-agent runtime.

## What yoyo can do

- **Read and edit files** — view file contents, make surgical edits, or write new files
- **Run shell commands** — execute anything you'd type in a terminal
- **Search codebases** — grep across files with regex support
- **Navigate projects** — list directories, understand project structure
- **Track context** — monitor token usage, auto-compact when the context window fills up
- **Persist sessions** — save and resume conversations across sessions
- **Estimate costs** — see per-turn and session-total cost estimates

## Quick example

```bash
export DEEPSEEK_API_KEY=sk-...
cargo install yoyo-ds-harness  # or: cargo run from source

yoyo-ds --deepseek-native
```

Then just talk to it:

```
> read src/main.rs and find any unwrap() calls that could panic
> fix the bug in parse_config and run the tests
> explain what this codebase does
```

## What makes yoyo different

Yoyo DS Harness turns the original self-evolving yoyo agent into a production DeepSeek harness. The product goal is simple: remember every failure, improve the harness with evidence, and promote only changes that pass evaluation. You can read its [source code](https://github.com/yologdev/yyds-harness/blob/main/src/main.rs), [release workflow](https://github.com/yologdev/yyds-harness/actions), and [project README](https://github.com/yologdev/yyds-harness/blob/main/README.md).

Current version: **v0.1.4**
