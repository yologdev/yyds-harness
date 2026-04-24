//! Read-only "info" REPL command handlers.
//!
//! These handlers print state without mutating anything: `/version`, `/status`,
//! `/tokens`, `/cost`, `/profile`, `/model` (show), `/provider` (show),
//! `/think` (show), `/changelog`, `/evolution`.
//!
//! Extracted from `commands.rs` as the first slice of issue #260, which tracks
//! splitting the 3,500-line `commands.rs` into focused modules. Read-only
//! handlers are the safest possible first slice — no shared mutable state, no
//! session-changes plumbing, no provider rebuild paths.

use crate::cli::{KNOWN_PROVIDERS, VERSION};
use crate::commands::thinking_level_name;
use crate::format::*;
use crate::git::*;

use yoagent::agent::Agent;
use yoagent::context::total_tokens;
use yoagent::*;

// ── /version ─────────────────────────────────────────────────────────────

/// Build a compact version string: `yoyo v0.1.9 (abc1234 2026-04-23) linux-x86_64`
///
/// Uses compile-time env vars `GIT_HASH` and `BUILD_DATE` (set by `build.rs`
/// or overridden in CI/release builds).
pub fn version_line() -> String {
    let hash = option_env!("GIT_HASH").unwrap_or("dev");
    let date = option_env!("BUILD_DATE").unwrap_or("dev");
    let target = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);

    format!("yoyo v{VERSION} ({hash} {date}) {target}")
}

pub fn handle_version() {
    println!("{DIM}  {}{RESET}\n", version_line());
}

/// Print enriched version output. When verbose, also shows provider,
/// model, and yoagent version.
pub fn handle_version_verbose(provider: &str, model: &str) {
    println!("{DIM}  {}", version_line());
    println!("  provider: {provider}  model: {model}");
    let yoagent_ver = option_env!("YOAGENT_VERSION").unwrap_or("unknown");
    println!("  yoagent:  v{yoagent_ver}{RESET}\n");
}

// ── /status ──────────────────────────────────────────────────────────────

pub fn handle_status(
    model: &str,
    cwd: &str,
    session_total: &Usage,
    elapsed: std::time::Duration,
    turns: usize,
    context_used: u64,
    context_max: u64,
) {
    println!("{DIM}  model:   {model}");
    if let Some(branch) = git_branch() {
        println!("  git:     {branch}");
    }
    println!("  cwd:     {cwd}");
    println!(
        "  session: {} elapsed, {turns} turn{}",
        format_duration(elapsed),
        if turns == 1 { "" } else { "s" }
    );
    println!(
        "  tokens:  {} in / {} out (session total)",
        session_total.input, session_total.output
    );
    if context_max > 0 {
        let pct = ((context_used as f64 / context_max as f64) * 100.0) as u32;
        let color = context_usage_color(pct);
        println!(
            "  context: {} / {} tokens ({color}{pct}%{DIM})",
            format_token_count(context_used),
            format_token_count(context_max),
        );
    }
    println!("{RESET}");
}

// ── /tokens ──────────────────────────────────────────────────────────────

pub fn handle_tokens(agent: &Agent, session_total: &Usage, model: &str) {
    let max_context = crate::cli::effective_context_tokens();
    let messages = agent.messages().to_vec();
    let context_used = total_tokens(&messages) as u64;
    let bar = context_bar(context_used, max_context);

    println!("{DIM}  Active context:");
    println!("    messages:    {}", messages.len());
    println!(
        "    current:     {} / {} tokens",
        format_token_count(context_used),
        format_token_count(max_context)
    );
    println!("    {bar}");
    if session_total.input > context_used + 1000 {
        println!("    {DIM}(earlier messages were compacted to save space — session totals below show full usage){RESET}");
    }
    if context_used as f64 / max_context as f64 > 0.75 {
        println!("    {YELLOW}⚠ Context is getting full. Consider /clear or /compact.{RESET}");
    }
    println!();
    println!("  Session totals (all API calls):");
    println!(
        "    input:       {} tokens",
        format_token_count(session_total.input)
    );
    println!(
        "    output:      {} tokens",
        format_token_count(session_total.output)
    );
    println!(
        "    cache read:  {} tokens",
        format_token_count(session_total.cache_read)
    );
    println!(
        "    cache write: {} tokens",
        format_token_count(session_total.cache_write)
    );
    if let Some(cost) = estimate_cost(session_total, model) {
        println!("    est. cost:   {}", format_cost(cost));
    }
    println!("{RESET}");
}

// ── /cost ────────────────────────────────────────────────────────────────

pub fn handle_cost(session_total: &Usage, model: &str, messages: &[yoagent::AgentMessage]) {
    if let Some(cost) = estimate_cost(session_total, model) {
        println!("{DIM}  Session cost: {}", format_cost(cost));
        println!(
            "    {} in / {} out",
            format_token_count(session_total.input),
            format_token_count(session_total.output)
        );
        if session_total.cache_read > 0 || session_total.cache_write > 0 {
            println!(
                "    cache: {} read / {} write",
                format_token_count(session_total.cache_read),
                format_token_count(session_total.cache_write)
            );
        }
        if let Some((input_cost, cw_cost, cr_cost, output_cost)) =
            cost_breakdown(session_total, model)
        {
            println!();
            println!("    Breakdown:");
            println!("      input:       {}", format_cost(input_cost));
            println!("      output:      {}", format_cost(output_cost));
            if cw_cost > 0.0 {
                println!("      cache write: {}", format_cost(cw_cost));
            }
            if cr_cost > 0.0 {
                println!("      cache read:  {}", format_cost(cr_cost));
            }
        }

        // Per-turn breakdown
        let turn_costs = extract_turn_costs(messages, model);
        if !turn_costs.is_empty() {
            println!();
            println!("{}", format_turn_costs(&turn_costs));
        }

        println!("{RESET}");
    } else {
        println!("{DIM}  Cost estimation not available for model '{model}'.{RESET}\n");
    }
}

// ── /model ───────────────────────────────────────────────────────────────

pub fn handle_model_show(model: &str) {
    println!("{DIM}  current model: {model}");
    println!("  usage: /model <name>{RESET}\n");
}

// ── /provider ────────────────────────────────────────────────────────────

pub fn handle_provider_show(provider: &str) {
    println!("{DIM}  current provider: {provider}");
    println!("  usage: /provider <name>");
    println!("  available: {}{RESET}\n", KNOWN_PROVIDERS.join(", "));
}

// ── /think ───────────────────────────────────────────────────────────────

pub fn handle_think_show(thinking: ThinkingLevel) {
    let level_str = thinking_level_name(thinking);
    println!("{DIM}  thinking: {level_str}");
    println!("  usage: /think <off|minimal|low|medium|high>{RESET}\n");
}

// ── /changelog ──────────────────────────────────────────────────────────

pub fn handle_profile(
    agent: &Agent,
    model: &str,
    provider: &str,
    session_start: std::time::Instant,
    session_total: &Usage,
) {
    let max_context = crate::cli::effective_context_tokens();
    let messages = agent.messages();
    let context_used = total_tokens(messages) as u64;
    // Count assistant turns
    let turns = messages
        .iter()
        .filter(|m| {
            matches!(
                m,
                yoagent::AgentMessage::Llm(yoagent::Message::Assistant { .. })
            )
        })
        .count();
    let elapsed = session_start.elapsed();

    // Cost string
    let cost_str = estimate_cost(session_total, model)
        .map(|c| format!("~{}", format_cost(c)))
        .unwrap_or_else(|| "n/a".to_string());

    // Token strings
    let tokens_str = format!(
        "{} in / {} out",
        format_token_count(session_total.input),
        format_token_count(session_total.output)
    );

    // Context string (plain, for width calculation)
    let ctx_plain = if max_context > 0 {
        let pct = ((context_used as f64 / max_context as f64) * 100.0) as u32;
        format!(
            "{} / {} ({}%)",
            format_token_count(context_used),
            format_token_count(max_context),
            pct
        )
    } else {
        format_token_count(context_used)
    };

    // Context color for the display version
    let pct_val = if max_context > 0 {
        ((context_used as f64 / max_context as f64) * 100.0) as u32
    } else {
        0
    };
    let ctx_color = context_usage_color(pct_val);

    let label = "Session Profile";
    // Build content lines: (key, plain_value, display_value)
    // plain_value is for width calculation, display_value may contain ANSI
    let duration_str = format_duration(elapsed);
    let turns_str = format!("{turns}");
    let lines: Vec<(&str, &str, String)> = vec![
        ("Model", model, model.to_string()),
        ("Provider", provider, provider.to_string()),
        ("Duration", &duration_str, duration_str.clone()),
        ("Turns", &turns_str, turns_str.clone()),
        ("Tokens", &tokens_str, tokens_str.clone()),
        ("Cost", &cost_str, cost_str.clone()),
        (
            "Context",
            &ctx_plain,
            format!("{ctx_color}{ctx_plain}{DIM}"),
        ),
    ];

    // Use fixed label column of 10 chars (longest key is "Provider" = 8 + ":  " = 11)
    let label_col = 10;
    // Find the longest value for box width
    let max_val_width = lines.iter().map(|(_, pv, _)| pv.len()).max().unwrap_or(20);
    // inner_width = "│ " + label_col + value + " │"
    let inner_width = (label_col + max_val_width + 2).max(label.len() + 4);

    // Top border
    let top_pad = inner_width - label.len() - 2;
    println!("{DIM}  ╭─ {label} {}╮", "─".repeat(top_pad));

    // Content lines
    for (key, plain_val, display_val) in &lines {
        let key_pad = label_col - key.len() - 1; // -1 for the colon
        let val_pad = inner_width - label_col - plain_val.len() - 2;
        println!(
            "  │ {key}:{}{display_val}{} │",
            " ".repeat(key_pad),
            " ".repeat(val_pad)
        );
    }

    // Bottom border
    println!("  ╰{}╯{RESET}", "─".repeat(inner_width));
    println!();
}

/// Parse the optional count argument from `/changelog [N]` input.
/// Returns a count clamped to 1..=100, defaulting to 15.
pub fn parse_changelog_count(input: &str) -> usize {
    let arg = input.strip_prefix("/changelog").unwrap_or("").trim();
    if arg.is_empty() {
        return 15;
    }
    arg.parse::<usize>().unwrap_or(15).clamp(1, 100)
}

pub fn handle_changelog(input: &str) {
    let count = parse_changelog_count(input);

    let count_arg = format!("-{count}");
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "--format=%h %s (%ar)", &count_arg])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let text = String::from_utf8_lossy(&result.stdout);
            let text = text.trim();
            if text.is_empty() {
                println!("{DIM}  (no commits found){RESET}\n");
            } else {
                println!("{DIM}  Recent commits ({count} max):\n");
                for line in text.lines() {
                    println!("    {line}");
                }
                println!("{RESET}");
            }
        }
        Ok(_) => {
            println!("{DIM}  (not in a git repository){RESET}\n");
        }
        Err(_) => {
            println!("{DIM}  (git not available){RESET}\n");
        }
    }
}

/// A parsed evolution session from a git tag like `day54-15-04`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionSession {
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub title: Option<String>,
}

/// Parse a git tag like `day54-15-04` into an `EvolutionSession`.
pub fn parse_evolution_tag(tag: &str) -> Option<EvolutionSession> {
    let rest = tag.strip_prefix("day")?;
    let parts: Vec<&str> = rest.splitn(3, '-').collect();
    if parts.len() != 3 {
        return None;
    }
    let day = parts[0].parse::<u32>().ok()?;
    let hour = parts[1].parse::<u32>().ok()?;
    let minute = parts[2].parse::<u32>().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some(EvolutionSession {
        day,
        hour,
        minute,
        title: None,
    })
}

/// Parse journal titles from JOURNAL.md content.
/// Returns a map of (day, hour, minute) → title.
pub fn parse_journal_titles(content: &str) -> std::collections::HashMap<(u32, u32, u32), String> {
    let mut titles = std::collections::HashMap::new();
    for line in content.lines() {
        // Format: ## Day NN — HH:MM — Title text
        if let Some(rest) = line.strip_prefix("## Day ") {
            let parts: Vec<&str> = rest.splitn(3, " — ").collect();
            if parts.len() == 3 {
                if let Ok(day) = parts[0].parse::<u32>() {
                    let time_parts: Vec<&str> = parts[1].splitn(2, ':').collect();
                    if time_parts.len() == 2 {
                        if let (Ok(hour), Ok(minute)) =
                            (time_parts[0].parse::<u32>(), time_parts[1].parse::<u32>())
                        {
                            titles.insert((day, hour, minute), parts[2].to_string());
                        }
                    }
                }
            }
        }
    }
    titles
}

/// Parse optional count from `/evolution [N]`.
pub fn parse_evolution_count(input: &str) -> usize {
    let arg = input.strip_prefix("/evolution").unwrap_or("").trim();
    if arg.is_empty() {
        return 10;
    }
    arg.parse::<usize>().unwrap_or(10).clamp(1, 100)
}

/// Compute sessions-per-day stats: (avg, max_day, max_count, current_streak).
/// current_streak = consecutive days with at least one session ending at current_day.
pub fn session_stats(sessions: &[EvolutionSession], current_day: u32) -> (f64, u32, u32, u32) {
    if sessions.is_empty() {
        return (0.0, 0, 0, 0);
    }

    // Count sessions per day
    let mut day_counts: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for s in sessions {
        *day_counts.entry(s.day).or_insert(0) += 1;
    }

    let total_days = day_counts.len() as f64;
    let total_sessions = sessions.len() as f64;
    let avg = total_sessions / total_days;

    let (max_day, max_count) = day_counts
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(&day, &count)| (day, count))
        .unwrap_or((0, 0));

    // Compute current streak (consecutive days ending at current_day)
    let mut streak = 0u32;
    let mut check_day = current_day;
    loop {
        if day_counts.contains_key(&check_day) {
            streak += 1;
            if check_day == 0 {
                break;
            }
            check_day -= 1;
        } else {
            break;
        }
    }

    (avg, max_day, max_count, streak)
}

/// Handle the `/evolution` command — show evolution history and stats.
pub fn handle_evolution(input: &str) {
    let count = parse_evolution_count(input);

    // Read DAY_COUNT
    let current_day = std::fs::read_to_string("DAY_COUNT")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);

    // Fetch git tags
    let tag_output = std::process::Command::new("git")
        .args(["tag", "--sort=-creatordate"])
        .output();

    let tags_text = match tag_output {
        Ok(result) if result.status.success() => {
            String::from_utf8_lossy(&result.stdout).to_string()
        }
        Ok(_) => {
            println!("{DIM}  (not in a git repository){RESET}\n");
            return;
        }
        Err(_) => {
            println!("{DIM}  (git not available){RESET}\n");
            return;
        }
    };

    // Parse tags into sessions
    let mut sessions: Vec<EvolutionSession> =
        tags_text.lines().filter_map(parse_evolution_tag).collect();

    // Try to load journal titles
    let journal_titles = std::fs::read_to_string("journals/JOURNAL.md")
        .map(|content| parse_journal_titles(&content))
        .unwrap_or_default();

    // Attach titles to sessions
    for session in &mut sessions {
        if let Some(title) = journal_titles.get(&(session.day, session.hour, session.minute)) {
            session.title = Some(title.clone());
        }
    }

    // Get test count
    let test_count = std::process::Command::new("cargo")
        .args(["test", "--", "--list"])
        .output()
        .ok()
        .and_then(|r| {
            if r.status.success() {
                let text = String::from_utf8_lossy(&r.stdout).to_string();
                Some(text.lines().filter(|l| l.ends_with(": test")).count())
            } else {
                None
            }
        })
        .unwrap_or(0);

    let total_sessions = sessions.len();

    // Header
    println!("\n  {BOLD}🐙 Evolution History — Day {current_day}{RESET}");
    println!();

    // Summary line
    let test_str = if test_count > 0 {
        format!(" | {CYAN}{test_count}{RESET} tests")
    } else {
        String::new()
    };
    println!(
        "  {DIM}{current_day} days{RESET} | {GREEN}{total_sessions}{RESET} sessions{test_str}"
    );

    // Stats
    let (avg, max_day, max_count, streak) = session_stats(&sessions, current_day);
    if total_sessions > 0 {
        println!(
            "  {DIM}avg {avg:.1}/day | peak {max_count} sessions (day {max_day}) | streak {streak} days{RESET}"
        );
    }
    println!();

    // Recent sessions
    if sessions.is_empty() {
        println!("{DIM}  (no evolution sessions found){RESET}\n");
        return;
    }

    let show_count = count.min(sessions.len());
    println!("  {BOLD}Recent sessions:{RESET}");
    for session in sessions.iter().take(show_count) {
        let today_marker = if session.day == current_day {
            format!(" {GREEN}(today){RESET}")
        } else {
            String::new()
        };

        let title_str = session
            .title
            .as_deref()
            .map(|t| format!("  {DIM}{t}{RESET}"))
            .unwrap_or_default();

        println!(
            "    {CYAN}Day {:>3}{RESET}  {:02}:{:02}{today_marker}{title_str}",
            session.day, session.hour, session.minute
        );
    }

    if total_sessions > show_count {
        let remaining = total_sessions - show_count;
        println!(
            "    {DIM}... and {remaining} more (use /evolution {total_sessions} to see all){RESET}"
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoagent::provider::AnthropicProvider;
    use yoagent::{Agent, Usage};

    #[test]
    fn test_tokens_display_labels() {
        // Verify no panic with zero usage and empty conversation
        let agent = Agent::new(AnthropicProvider)
            .with_system_prompt("test")
            .with_model("test-model")
            .with_api_key("test-key");

        let usage = Usage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };

        // Should not panic with zero usage and empty conversation
        handle_tokens(&agent, &usage, "test-model");
    }

    #[test]
    fn test_tokens_display_with_large_values() {
        // Verify no panic with very large token counts
        let agent = Agent::new(AnthropicProvider)
            .with_system_prompt("test")
            .with_model("test-model")
            .with_api_key("test-key");

        let usage = Usage {
            input: 10_000_000,
            output: 5_000_000,
            cache_read: 3_000_000,
            cache_write: 1_000_000,
            total_tokens: 19_000_000,
        };

        // Should not panic with very large values
        handle_tokens(&agent, &usage, "test-model");
    }

    #[test]
    fn test_tokens_labels_are_clarified() {
        // Source-level check: the function body should use the clarified labels
        // from Issue #189, not the old confusing ones
        let source = include_str!("commands_info.rs");
        assert!(
            source.contains("Active context:"),
            "/tokens should use 'Active context:' header"
        );
        assert!(
            source.contains("Session totals (all API calls):"),
            "/tokens should use 'Session totals (all API calls):' header"
        );
        assert!(
            source.contains("session totals below show full usage"),
            "Compaction note should reference session totals"
        );
    }

    #[test]
    fn test_handle_status_with_timing() {
        use std::time::Duration;
        // Just verify it doesn't panic with various inputs
        handle_status(
            "test-model",
            "/tmp",
            &Usage::default(),
            Duration::from_secs(0),
            0,
            0,
            0,
        );
        handle_status(
            "test-model",
            "/tmp",
            &Usage::default(),
            Duration::from_secs(125),
            1,
            5000,
            200_000,
        );
        handle_status(
            "test-model",
            "/tmp",
            &Usage::default(),
            Duration::from_secs(7200),
            42,
            180_000,
            200_000,
        );
    }

    #[test]
    fn test_handle_status_context_line() {
        use std::time::Duration;
        // When context_max > 0, the context line should appear (no panic)
        handle_status(
            "test-model",
            "/tmp",
            &Usage::default(),
            Duration::from_secs(60),
            3,
            45_231,
            200_000,
        );
    }

    #[test]
    fn test_handle_status_skips_context_when_zero() {
        use std::time::Duration;
        // When context_max == 0, it should skip the context line (no panic)
        handle_status(
            "test-model",
            "/tmp",
            &Usage::default(),
            Duration::from_secs(60),
            3,
            0,
            0,
        );
    }

    #[test]
    fn test_parse_changelog_count_default() {
        assert_eq!(parse_changelog_count("/changelog"), 15);
    }

    #[test]
    fn test_parse_changelog_count_custom() {
        assert_eq!(parse_changelog_count("/changelog 30"), 30);
        assert_eq!(parse_changelog_count("/changelog 1"), 1);
        assert_eq!(parse_changelog_count("/changelog 100"), 100);
    }

    #[test]
    fn test_parse_changelog_count_clamped() {
        assert_eq!(parse_changelog_count("/changelog 0"), 1);
        assert_eq!(parse_changelog_count("/changelog 999"), 100);
    }

    #[test]
    fn test_parse_changelog_count_invalid() {
        // Non-numeric falls back to default 15
        assert_eq!(parse_changelog_count("/changelog abc"), 15);
        assert_eq!(parse_changelog_count("/changelog -5"), 15);
    }

    #[test]
    fn test_handle_changelog_no_panic() {
        // Should not panic regardless of git availability
        handle_changelog("/changelog");
        handle_changelog("/changelog 5");
    }

    #[test]
    fn test_handle_profile_no_panic() {
        use std::time::Instant;
        let agent = Agent::new(AnthropicProvider)
            .with_system_prompt("test")
            .with_model("test-model")
            .with_api_key("test-key");

        let usage = Usage::default();
        // Should not panic with empty agent and zero usage
        handle_profile(
            &agent,
            "claude-sonnet-4-20250514",
            "anthropic",
            Instant::now(),
            &usage,
        );
    }

    #[test]
    fn test_handle_profile_with_usage() {
        use std::time::Instant;
        let agent = Agent::new(AnthropicProvider)
            .with_system_prompt("test")
            .with_model("test-model")
            .with_api_key("test-key");

        let usage = Usage {
            input: 45_231,
            output: 12_890,
            cache_read: 5_000,
            cache_write: 2_000,
            total_tokens: 65_121,
        };
        // Should not panic with real-ish usage
        handle_profile(
            &agent,
            "claude-sonnet-4-20250514",
            "anthropic",
            Instant::now(),
            &usage,
        );
    }

    #[test]
    fn test_version_line_contains_version() {
        let line = version_line();
        assert!(
            line.contains(&format!("v{VERSION}")),
            "version_line should contain the version: {line}"
        );
    }

    #[test]
    fn test_version_line_contains_target() {
        let line = version_line();
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        assert!(
            line.contains(&format!("{os}-{arch}")),
            "version_line should contain target triple: {line}"
        );
    }

    #[test]
    fn test_version_line_format() {
        let line = version_line();
        // Should match: yoyo vX.Y.Z (HASH DATE) OS-ARCH
        assert!(
            line.starts_with("yoyo v"),
            "should start with 'yoyo v': {line}"
        );
        assert!(line.contains('('), "should contain '(': {line}");
        assert!(line.contains(')'), "should contain ')': {line}");
    }

    #[test]
    fn test_handle_version_no_panic() {
        // Basic version should not panic
        handle_version();
    }

    #[test]
    fn test_handle_version_verbose_no_panic() {
        // Verbose version with provider/model should not panic
        handle_version_verbose("anthropic", "claude-sonnet-4-20250514");
    }

    // === /evolution tests ===

    #[test]
    fn test_parse_evolution_tag_valid() {
        let s = parse_evolution_tag("day54-15-04").unwrap();
        assert_eq!(s.day, 54);
        assert_eq!(s.hour, 15);
        assert_eq!(s.minute, 4);
        assert!(s.title.is_none());
    }

    #[test]
    fn test_parse_evolution_tag_single_digits() {
        let s = parse_evolution_tag("day1-0-0").unwrap();
        assert_eq!(s.day, 1);
        assert_eq!(s.hour, 0);
        assert_eq!(s.minute, 0);
    }

    #[test]
    fn test_parse_evolution_tag_invalid_no_prefix() {
        assert!(parse_evolution_tag("v0.1.9").is_none());
    }

    #[test]
    fn test_parse_evolution_tag_invalid_bad_time() {
        assert!(parse_evolution_tag("day5-25-00").is_none()); // hour > 23
        assert!(parse_evolution_tag("day5-12-60").is_none()); // minute > 59
    }

    #[test]
    fn test_parse_evolution_tag_invalid_not_numbers() {
        assert!(parse_evolution_tag("dayX-12-30").is_none());
        assert!(parse_evolution_tag("day5-ab-30").is_none());
    }

    #[test]
    fn test_parse_evolution_tag_too_few_parts() {
        assert!(parse_evolution_tag("day5-12").is_none());
        assert!(parse_evolution_tag("day5").is_none());
    }

    #[test]
    fn test_parse_journal_titles() {
        let content = "\
# Journal

## Day 54 — 15:04 — Five sessions of standing still

Some text here.

## Day 54 — 04:40 — Knowing where you were built

More text.

## Day 53 — 19:11 — The file that was three things pretending to be one
";
        let titles = parse_journal_titles(content);
        assert_eq!(titles.len(), 3);
        assert_eq!(
            titles.get(&(54, 15, 4)),
            Some(&"Five sessions of standing still".to_string())
        );
        assert_eq!(
            titles.get(&(54, 4, 40)),
            Some(&"Knowing where you were built".to_string())
        );
        assert_eq!(
            titles.get(&(53, 19, 11)),
            Some(&"The file that was three things pretending to be one".to_string())
        );
    }

    #[test]
    fn test_parse_journal_titles_empty() {
        let titles = parse_journal_titles("");
        assert!(titles.is_empty());
    }

    #[test]
    fn test_parse_journal_titles_no_entries() {
        let titles = parse_journal_titles("# Journal\n\nSome other content.\n");
        assert!(titles.is_empty());
    }

    #[test]
    fn test_parse_evolution_count_default() {
        assert_eq!(parse_evolution_count("/evolution"), 10);
    }

    #[test]
    fn test_parse_evolution_count_custom() {
        assert_eq!(parse_evolution_count("/evolution 20"), 20);
        assert_eq!(parse_evolution_count("/evolution 1"), 1);
    }

    #[test]
    fn test_parse_evolution_count_clamped() {
        assert_eq!(parse_evolution_count("/evolution 0"), 1);
        assert_eq!(parse_evolution_count("/evolution 999"), 100);
    }

    #[test]
    fn test_parse_evolution_count_invalid() {
        assert_eq!(parse_evolution_count("/evolution abc"), 10);
    }

    #[test]
    fn test_session_stats_empty() {
        let (avg, max_day, max_count, streak) = session_stats(&[], 55);
        assert_eq!(avg, 0.0);
        assert_eq!(max_day, 0);
        assert_eq!(max_count, 0);
        assert_eq!(streak, 0);
    }

    #[test]
    fn test_session_stats_basic() {
        let sessions = vec![
            EvolutionSession {
                day: 54,
                hour: 4,
                minute: 40,
                title: None,
            },
            EvolutionSession {
                day: 54,
                hour: 15,
                minute: 4,
                title: None,
            },
            EvolutionSession {
                day: 53,
                hour: 19,
                minute: 11,
                title: None,
            },
        ];
        let (avg, max_day, max_count, streak) = session_stats(&sessions, 54);
        assert!((avg - 1.5).abs() < 0.01); // 3 sessions / 2 days
        assert_eq!(max_day, 54);
        assert_eq!(max_count, 2);
        assert_eq!(streak, 2); // days 54 and 53 are consecutive
    }

    #[test]
    fn test_session_stats_streak_with_gap() {
        let sessions = vec![
            EvolutionSession {
                day: 55,
                hour: 1,
                minute: 0,
                title: None,
            },
            // gap: no day 54
            EvolutionSession {
                day: 53,
                hour: 10,
                minute: 0,
                title: None,
            },
        ];
        let (_avg, _max_day, _max_count, streak) = session_stats(&sessions, 55);
        assert_eq!(streak, 1); // only day 55, gap before 53
    }

    #[test]
    fn test_handle_evolution_no_panic() {
        // Should not panic regardless of environment
        handle_evolution("/evolution");
        handle_evolution("/evolution 5");
    }
}
