//! Ast-grep structural search command handler: /ast.

use crate::format::*;

/// Subcommand completions for `/ast <Tab>`.
pub const AST_GREP_FLAGS: &[&str] = &["--lang", "--in"];

/// Check if ast-grep's `sg` binary is available on PATH.
pub fn is_ast_grep_available() -> bool {
    std::process::Command::new("sg")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run ast-grep structural search.
/// Returns Ok(output) or Err(error message).
pub fn run_ast_grep_search(
    pattern: &str,
    lang: Option<&str>,
    path: Option<&str>,
) -> Result<String, String> {
    if !is_ast_grep_available() {
        return Err(
            "ast-grep (sg) is not installed. Install from: https://ast-grep.github.io/".into(),
        );
    }
    let mut cmd = std::process::Command::new("sg");
    cmd.arg("run").arg("--pattern").arg(pattern);
    if let Some(l) = lang {
        cmd.arg("--lang").arg(l);
    }
    if let Some(p) = path {
        cmd.arg(p);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            if stdout.trim().is_empty() {
                Ok("No matches found.".into())
            } else {
                Ok(stdout)
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if stderr.trim().is_empty() {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                if stdout.trim().is_empty() {
                    Ok("No matches found.".into())
                } else {
                    Ok(stdout)
                }
            } else {
                Err(format!("ast-grep error: {}", stderr.trim()))
            }
        }
        Err(e) => Err(format!("Failed to run sg: {e}")),
    }
}

/// Parse `/ast` command arguments into (pattern, lang, path).
pub fn parse_ast_grep_args(
    input: &str,
) -> Result<(String, Option<String>, Option<String>), String> {
    let rest = input.strip_prefix("/ast").unwrap_or("").trim();

    if rest.is_empty() {
        return Err("Usage: /ast <pattern> [--lang <lang>] [--in <path>]".into());
    }

    let parts: Vec<&str> = rest.split_whitespace().collect();
    let mut pattern_parts: Vec<&str> = Vec::new();
    let mut lang: Option<String> = None;
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--lang" => {
                if i + 1 < parts.len() {
                    lang = Some(parts[i + 1].to_string());
                    i += 2;
                } else {
                    return Err("--lang requires a value (e.g. --lang rust)".into());
                }
            }
            "--in" => {
                if i + 1 < parts.len() {
                    path = Some(parts[i + 1].to_string());
                    i += 2;
                } else {
                    return Err("--in requires a value (e.g. --in src/)".into());
                }
            }
            other => {
                pattern_parts.push(other);
                i += 1;
            }
        }
    }

    if pattern_parts.is_empty() {
        return Err("Usage: /ast <pattern> [--lang <lang>] [--in <path>]".into());
    }

    Ok((pattern_parts.join(" "), lang, path))
}

/// Handle the `/ast` REPL command.
pub fn handle_ast_grep(input: &str) {
    match parse_ast_grep_args(input) {
        Err(msg) => {
            println!("{YELLOW}  {msg}{RESET}\n");
        }
        Ok((pattern, lang, path)) => {
            if !is_ast_grep_available() {
                println!("{YELLOW}  ast-grep (sg) is not installed.{RESET}");
                println!("{DIM}  Install from: https://ast-grep.github.io/{RESET}");
                println!("{DIM}  Example: npm i -g @ast-grep/cli{RESET}\n");
                return;
            }
            println!("{DIM}  Searching for pattern: {pattern}{RESET}");
            match run_ast_grep_search(&pattern, lang.as_deref(), path.as_deref()) {
                Ok(output) => {
                    println!("{output}");
                }
                Err(e) => {
                    println!("{YELLOW}  {e}{RESET}\n");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ast_grep_available_no_panic() {
        // Should not panic regardless of whether sg is installed
        let _ = is_ast_grep_available();
    }

    #[test]
    fn test_ast_grep_search_no_sg() {
        // When sg is not installed, should return a helpful error
        if !is_ast_grep_available() {
            let result = run_ast_grep_search("$X.unwrap()", None, None);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not installed"));
        }
    }

    #[test]
    fn test_ast_in_known_commands() {
        use crate::commands::KNOWN_COMMANDS;
        assert!(
            KNOWN_COMMANDS.contains(&"/ast"),
            "/ast should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_ast_in_help_text() {
        use crate::help::help_text;
        let help = help_text();
        assert!(help.contains("/ast"), "/ast should appear in help text");
    }

    #[test]
    fn test_parse_ast_grep_args_simple_pattern() {
        let result = parse_ast_grep_args("/ast $X.unwrap()");
        assert!(result.is_ok());
        let (pattern, lang, path) = result.unwrap();
        assert_eq!(pattern, "$X.unwrap()");
        assert!(lang.is_none());
        assert!(path.is_none());
    }

    #[test]
    fn test_parse_ast_grep_args_with_lang() {
        let result = parse_ast_grep_args("/ast $X.unwrap() --lang rust");
        assert!(result.is_ok());
        let (pattern, lang, path) = result.unwrap();
        assert_eq!(pattern, "$X.unwrap()");
        assert_eq!(lang.as_deref(), Some("rust"));
        assert!(path.is_none());
    }

    #[test]
    fn test_parse_ast_grep_args_with_lang_and_path() {
        let result = parse_ast_grep_args("/ast $X.unwrap() --lang rust --in src/");
        assert!(result.is_ok());
        let (pattern, lang, path) = result.unwrap();
        assert_eq!(pattern, "$X.unwrap()");
        assert_eq!(lang.as_deref(), Some("rust"));
        assert_eq!(path.as_deref(), Some("src/"));
    }

    #[test]
    fn test_parse_ast_grep_args_flags_before_pattern() {
        let result = parse_ast_grep_args("/ast --lang rust $X.unwrap()");
        assert!(result.is_ok());
        let (pattern, lang, _) = result.unwrap();
        assert_eq!(pattern, "$X.unwrap()");
        assert_eq!(lang.as_deref(), Some("rust"));
    }

    #[test]
    fn test_parse_ast_grep_args_empty() {
        let result = parse_ast_grep_args("/ast");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Usage"));
    }

    #[test]
    fn test_parse_ast_grep_args_missing_lang_value() {
        let result = parse_ast_grep_args("/ast $X --lang");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--lang requires"));
    }

    #[test]
    fn test_parse_ast_grep_args_missing_in_value() {
        let result = parse_ast_grep_args("/ast $X --in");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--in requires"));
    }

    #[test]
    fn test_ast_tab_completion() {
        use crate::commands::command_arg_completions;
        let candidates = command_arg_completions("/ast", "");
        assert!(
            candidates.contains(&"--lang".to_string()),
            "Should include '--lang'"
        );
        assert!(
            candidates.contains(&"--in".to_string()),
            "Should include '--in'"
        );
    }

    #[test]
    fn test_ast_tab_completion_filters() {
        use crate::commands::command_arg_completions;
        let candidates = command_arg_completions("/ast", "--l");
        assert!(
            candidates.contains(&"--lang".to_string()),
            "Should include '--lang' for prefix '--l'"
        );
        assert!(
            !candidates.contains(&"--in".to_string()),
            "Should not include '--in' for prefix '--l'"
        );
    }

    #[test]
    fn test_handle_ast_grep_no_panic_empty() {
        // Should not panic on empty input
        handle_ast_grep("/ast");
    }

    #[test]
    fn test_handle_ast_grep_no_panic_with_pattern() {
        // Should not panic even if sg is not installed
        handle_ast_grep("/ast $X.unwrap()");
    }
}
