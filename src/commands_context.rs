//! Shell subcommands for DeepSeek-native context preview and explanation.

use crate::format::*;

pub fn handle_context_subcommand(args: &[String]) {
    match args.get(2).map(|s| s.as_str()).unwrap_or("help") {
        "preview" => handle_preview(args),
        "explain" => handle_explain(args),
        "index" => handle_index(args),
        _ => print_usage(),
    }
}

/// Maximum seconds allowed for building the DeepSeek context preview.
/// Guards against hangs in external processes (ast-grep) or slow filesystem operations.
const CONTEXT_PREVIEW_TIMEOUT_SECS: u64 = 30;

fn build_preview_with_timeout() -> Option<crate::context::DeepSeekContextPreview> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(crate::context::build_deepseek_context_preview());
    });
    rx.recv_timeout(std::time::Duration::from_secs(CONTEXT_PREVIEW_TIMEOUT_SECS))
        .ok()
}

fn handle_preview(args: &[String]) {
    if !deepseek_context_enabled(args) {
        eprintln!(
            "{YELLOW}  context preview currently supports the yyds DeepSeek-native layout only{RESET}"
        );
        return;
    }
    match build_preview_with_timeout() {
        Some(preview) => println!("{}", preview.render_preview()),
        None => eprintln!(
            "{YELLOW}  context preview timed out after {CONTEXT_PREVIEW_TIMEOUT_SECS}s — the operation may be slow due to a large repository or missing indexes. Try running `yyds context index --write` first.{RESET}"
        ),
    }
}

fn handle_explain(args: &[String]) {
    if !deepseek_context_enabled(args) {
        eprintln!(
            "{YELLOW}  context explain currently supports the yyds DeepSeek-native layout only{RESET}"
        );
        return;
    }
    match build_preview_with_timeout() {
        Some(preview) => println!("{}", preview.render_explain()),
        None => eprintln!(
            "{YELLOW}  context explain timed out after {CONTEXT_PREVIEW_TIMEOUT_SECS}s — the operation may be slow due to a large repository or missing indexes. Try running `yyds context index --write` first.{RESET}"
        ),
    }
}

fn handle_index(args: &[String]) {
    if !deepseek_context_enabled(args) {
        eprintln!("{YELLOW}  context index currently supports the yyds DeepSeek-native layout only{RESET}");
        return;
    }
    let write = args.iter().any(|arg| arg == "--write");
    let json_output = args.iter().any(|arg| arg == "--json");
    let path = args
        .iter()
        .position(|arg| arg == "--path")
        .and_then(|idx| args.get(idx + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(crate::context::DEFAULT_SEMANTIC_INDEX_PATH));
    match crate::context::build_and_maybe_write_semantic_index(&path, write) {
        Ok(report) if json_output => println!(
            "{}",
            serde_json::to_string_pretty(&report.payload()).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => println!("{}", report.render()),
        Err(e) => eprintln!("{RED}  failed to build context semantic index: {e}{RESET}"),
    }
}

fn print_usage() {
    println!(
        "Usage: yyds context <command>\n\n  preview\n  explain\n  index [--write] [--path PATH] [--json]"
    );
}

fn deepseek_context_enabled(args: &[String]) -> bool {
    args.first()
        .and_then(|arg| std::path::Path::new(arg).file_stem())
        .and_then(|stem| stem.to_str())
        .map(|stem| stem == "yyds")
        .unwrap_or(false)
        || args.iter().any(|arg| arg == "--deepseek-native")
}
