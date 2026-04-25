//! Permission config, directory restrictions, MCP server config, and TOML parsing helpers.
//!
//! Extracted from `cli.rs` to keep configuration parsing separate from CLI argument handling.

/// Permission configuration for bash command auto-approval.
/// Parsed from the `[permissions]` section in `.yoyo.toml`.
#[derive(Debug, Clone, Default)]
pub struct PermissionConfig {
    /// Patterns that auto-approve matching bash commands (no prompt needed).
    pub allow: Vec<String>,
    /// Patterns that auto-deny matching bash commands (rejected with message).
    pub deny: Vec<String>,
}

impl PermissionConfig {
    /// Check a command against deny patterns first, then allow patterns.
    /// Returns `Some(true)` if allowed, `Some(false)` if denied, `None` if no match (prompt user).
    pub fn check(&self, command: &str) -> Option<bool> {
        // Deny takes priority — check deny patterns first
        for pattern in &self.deny {
            if glob_match(pattern, command) {
                return Some(false);
            }
        }
        // Then check allow patterns
        for pattern in &self.allow {
            if glob_match(pattern, command) {
                return Some(true);
            }
        }
        // No match — prompt the user
        None
    }

    /// Returns true if no patterns are configured.
    pub fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }
}

/// Directory restriction configuration for file access security.
/// Controls which directories yoyo's file tools (read_file, write_file, edit_file,
/// list_files, search) can access. When configured, paths are canonicalized to prevent
/// `../` traversal escapes.
///
/// Rules:
/// - If `deny` is non-empty, any path under a denied directory is blocked.
/// - If `allow` is non-empty, only paths under an allowed directory are permitted.
/// - Deny overrides allow when both match.
/// - Paths are resolved to absolute paths before checking.
#[derive(Debug, Clone, Default)]
pub struct DirectoryRestrictions {
    /// Directories that are explicitly allowed. If non-empty, only these dirs are accessible.
    pub allow: Vec<String>,
    /// Directories that are explicitly denied. Always takes priority over allow.
    pub deny: Vec<String>,
}

impl DirectoryRestrictions {
    /// Returns true if no restrictions are configured.
    pub fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }

    /// Check whether a given file path is permitted under the current restrictions.
    /// Returns `Ok(())` if the path is allowed, or `Err(reason)` if blocked.
    ///
    /// Path resolution:
    /// - Absolute paths are used directly.
    /// - Relative paths are resolved against the current working directory.
    /// - Symlinks and `..` components are resolved via `std::fs::canonicalize`
    ///   when the path exists, or by manual normalization when it doesn't.
    pub fn check_path(&self, path: &str) -> Result<(), String> {
        if self.is_empty() {
            return Ok(());
        }

        let resolved = resolve_path(path);

        // Deny always takes priority
        for denied in &self.deny {
            let denied_resolved = resolve_path(denied);
            if path_is_under(&resolved, &denied_resolved) {
                return Err(format!(
                    "Access denied: '{}' is under restricted directory '{}'",
                    path, denied
                ));
            }
        }

        // If allow list is set, path must be under at least one allowed directory
        if !self.allow.is_empty() {
            let allowed = self.allow.iter().any(|a| {
                let a_resolved = resolve_path(a);
                path_is_under(&resolved, &a_resolved)
            });
            if !allowed {
                return Err(format!(
                    "Access denied: '{}' is not under any allowed directory",
                    path
                ));
            }
        }

        Ok(())
    }
}

/// Resolve a path to an absolute, normalized form.
/// Uses `canonicalize` for existing paths (resolves symlinks, `..`, etc.).
/// Falls back to manual normalization for paths that don't exist yet.
fn resolve_path(path: &str) -> String {
    // Try canonicalize first (works for existing paths)
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical.to_string_lossy().to_string();
    }

    // Manual normalization for non-existent paths
    let p = std::path::Path::new(path);
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .join(p)
    };

    // Normalize components: resolve `.` and `..`
    let mut components = Vec::new();
    for component in absolute.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    let normalized: std::path::PathBuf = components.iter().collect();
    normalized.to_string_lossy().to_string()
}

/// Check if `path` is under (or equal to) `dir`.
/// Both should be absolute, normalized paths.
fn path_is_under(path: &str, dir: &str) -> bool {
    // Ensure dir ends with separator for prefix matching
    let dir_with_sep = if dir.ends_with('/') {
        dir.to_string()
    } else {
        format!("{}/", dir)
    };
    path == dir || path.starts_with(&dir_with_sep)
}

/// Simple glob matching: `*` matches any sequence of characters (including empty).
/// Supports multiple `*` wildcards. No other special characters.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();

    // No wildcards — exact match
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut pos = 0;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // First segment must match at the start
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Last segment must match at the end
            if !text[pos..].ends_with(part) {
                return false;
            }
            pos = text.len();
        } else {
            // Middle segments must appear in order
            match text[pos..].find(part) {
                Some(idx) => pos += idx + part.len(),
                None => return false,
            }
        }
    }

    true
}

/// Parse a TOML-style array value like `["pattern1", "pattern2"]` into a Vec<String>.
pub fn parse_toml_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    inner
        .split(',')
        .map(|s| {
            let s = s.trim();
            // Strip quotes
            if (s.starts_with('"') && s.ends_with('"'))
                || (s.starts_with('\'') && s.ends_with('\''))
            {
                s[1..s.len() - 1].to_string()
            } else {
                s.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a `[permissions]` section from a TOML config file content.
/// Looks for `allow = [...]` and `deny = [...]` lines under `[permissions]`.
pub fn parse_permissions_from_config(content: &str) -> PermissionConfig {
    let mut config = PermissionConfig::default();
    let mut in_permissions = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Check for section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_permissions = trimmed == "[permissions]";
            continue;
        }
        if !in_permissions {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "allow" => config.allow = parse_toml_array(value),
                "deny" => config.deny = parse_toml_array(value),
                _ => {}
            }
        }
    }
    config
}

/// Parse a `[directories]` section from a TOML config file content.
/// Looks for `allow = [...]` and `deny = [...]` lines under `[directories]`.
pub fn parse_directories_from_config(content: &str) -> DirectoryRestrictions {
    let mut config = DirectoryRestrictions::default();
    let mut in_directories = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_directories = trimmed == "[directories]";
            continue;
        }
        if !in_directories {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "allow" => config.allow = parse_toml_array(value),
                "deny" => config.deny = parse_toml_array(value),
                _ => {}
            }
        }
    }
    config
}

/// Parse `[mcp_servers.<name>]` sections from raw config content.
///
/// Each section defines a named MCP server with a command, optional args, and optional env vars:
/// ```toml
/// [mcp_servers.filesystem]
/// command = "npx"
/// args = ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
///
/// [mcp_servers.postgres]
/// command = "npx"
/// args = ["-y", "@modelcontextprotocol/server-postgres"]
/// env = { DATABASE_URL = "postgresql://localhost/mydb" }
/// ```
pub fn parse_mcp_servers_from_config(content: &str) -> Vec<McpServerConfig> {
    let mut servers: Vec<McpServerConfig> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_command: Option<String> = None;
    let mut current_args: Vec<String> = Vec::new();
    let mut current_env: Vec<(String, String)> = Vec::new();

    // Helper: flush accumulated server data into the result vec
    let flush = |name: &mut Option<String>,
                 command: &mut Option<String>,
                 args: &mut Vec<String>,
                 env: &mut Vec<(String, String)>,
                 servers: &mut Vec<McpServerConfig>| {
        if let (Some(n), Some(c)) = (name.take(), command.take()) {
            servers.push(McpServerConfig {
                name: n,
                command: c,
                args: std::mem::take(args),
                env: std::mem::take(env),
            });
        } else {
            // Reset even if incomplete
            *name = None;
            *command = None;
            args.clear();
            env.clear();
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Detect section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Flush any previous MCP server
            flush(
                &mut current_name,
                &mut current_command,
                &mut current_args,
                &mut current_env,
                &mut servers,
            );

            let section = &trimmed[1..trimmed.len() - 1];
            if let Some(name) = section.strip_prefix("mcp_servers.") {
                let name = name.trim();
                if !name.is_empty() {
                    current_name = Some(name.to_string());
                }
            }
            continue;
        }

        // Only parse key=value lines inside an mcp_servers section
        if current_name.is_none() {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "command" => {
                    let v = strip_quotes(value);
                    if !v.is_empty() {
                        current_command = Some(v);
                    }
                }
                "args" => {
                    current_args = parse_toml_array(value);
                }
                "env" => {
                    current_env = parse_inline_table(value);
                }
                _ => {}
            }
        }
    }

    // Flush the last server
    flush(
        &mut current_name,
        &mut current_command,
        &mut current_args,
        &mut current_env,
        &mut servers,
    );

    servers
}

/// Strip surrounding quotes from a TOML string value.
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() >= 2 {
            s[1..s.len() - 1].to_string()
        } else {
            String::new()
        }
    } else {
        s.to_string()
    }
}

/// Parse a simple inline TOML table like `{ KEY = "value", KEY2 = "value2" }`.
/// Returns a list of (key, value) pairs.
fn parse_inline_table(s: &str) -> Vec<(String, String)> {
    let s = s.trim();
    // Strip surrounding braces
    let inner = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for pair in inner.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((k, v)) = pair.split_once('=') {
            let k = k.trim().to_string();
            let v = strip_quotes(v);
            if !k.is_empty() {
                result.push((k, v));
            }
        }
    }
    result
}

/// Configuration for an MCP (Model Context Protocol) server defined in config TOML sections.
///
/// Parsed from `[mcp_servers.<name>]` sections in `.yoyo.toml` or user config:
/// ```toml
/// [mcp_servers.filesystem]
/// command = "npx"
/// args = ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
/// env = { DATABASE_URL = "postgresql://localhost/mydb" }
/// ```
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Keys that `/config set` understands. Each entry is a key name and a
/// human-readable description used in error messages.
pub const SETTABLE_KEYS: &[(&str, &str)] = &[
    ("model", "AI model name"),
    ("provider", "AI provider"),
    ("thinking", "thinking level (none/low/medium/high)"),
    ("temperature", "sampling temperature (0.0–2.0)"),
    ("max_tokens", "maximum response tokens"),
    ("max_turns", "maximum agent turns per prompt"),
];

/// Validate a config value for a given key. Returns `Ok(canonical_value)`
/// on success or `Err(message)` on invalid input.
pub fn validate_config_value(key: &str, value: &str) -> Result<String, String> {
    match key {
        "model" | "provider" => {
            if value.is_empty() {
                return Err(format!("{key} cannot be empty"));
            }
            Ok(value.to_string())
        }
        "thinking" => {
            let lower = value.to_ascii_lowercase();
            match lower.as_str() {
                "none" | "off" | "disabled" => Ok("none".to_string()),
                "low" | "minimal" => Ok("low".to_string()),
                "medium" | "med" => Ok("medium".to_string()),
                "high" | "max" => Ok("high".to_string()),
                _ => Err(format!(
                    "invalid thinking level '{value}' — use none, low, medium, or high"
                )),
            }
        }
        "temperature" => match value.parse::<f32>() {
            Ok(t) if (0.0..=2.0).contains(&t) => Ok(format!("{t}")),
            Ok(t) => Err(format!("temperature {t} out of range (0.0–2.0)")),
            Err(_) => Err(format!("'{value}' is not a valid number")),
        },
        "max_tokens" => match value.parse::<u32>() {
            Ok(n) if n > 0 => Ok(n.to_string()),
            Ok(_) => Err("max_tokens must be positive".to_string()),
            Err(_) => Err(format!("'{value}' is not a valid integer")),
        },
        "max_turns" => match value.parse::<usize>() {
            Ok(n) if n > 0 => Ok(n.to_string()),
            Ok(_) => Err("max_turns must be positive".to_string()),
            Err(_) => Err(format!("'{value}' is not a valid integer")),
        },
        _ => Err(format!(
            "unknown config key '{key}' — settable keys: {}",
            SETTABLE_KEYS
                .iter()
                .map(|(k, _)| *k)
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Write a single key=value pair to a TOML config file.
///
/// If the file exists, the key is either replaced in-place (preserving
/// comments and surrounding lines) or appended. If the file doesn't exist,
/// it's created with a header comment. Values are always quoted.
///
/// When `project_local` is true, writes to `.yoyo.toml` in the current
/// directory. Otherwise writes to `~/.yoyo.toml`.
///
/// Returns the path that was written to on success.
pub fn write_config_value(
    key: &str,
    value: &str,
    project_local: bool,
) -> Result<std::path::PathBuf, String> {
    let path = if project_local {
        std::path::PathBuf::from(".yoyo.toml")
    } else {
        crate::cli::home_config_path()
            .ok_or_else(|| "could not determine home directory".to_string())?
    };

    write_config_value_to(key, value, &path)
}

/// Write a config value to a specific path. Factored out of
/// [`write_config_value`] so tests can target a temp file.
pub fn write_config_value_to(
    key: &str,
    value: &str,
    path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory {}: {e}", parent.display()))?;
        }
    }

    // Read existing content or start fresh
    let existing = std::fs::read_to_string(path).unwrap_or_default();

    let new_content = set_toml_key(&existing, key, value);

    std::fs::write(path, &new_content)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;

    Ok(path.to_path_buf())
}

/// Pure function: insert or replace `key = "value"` in a flat TOML string.
/// Preserves comments, blank lines, and other keys. If the key already
/// exists (matched by `^key\s*=`), replaces that line. Otherwise appends.
///
/// Values that look like numbers or booleans are written unquoted; everything
/// else is quoted.
pub fn set_toml_key(content: &str, key: &str, value: &str) -> String {
    let formatted_value = format_toml_value(value);
    let new_line = format!("{key} = {formatted_value}");

    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            // Match `key = ...` at the start of a non-comment line
            if !trimmed.starts_with('#') {
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == key {
                        found = true;
                        return new_line.clone();
                    }
                }
            }
            line.to_string()
        })
        .collect();

    if !found {
        // Ensure there's a trailing newline before appending
        if !lines.is_empty() {
            let last = lines.last().unwrap();
            if !last.is_empty() {
                // Only add a blank line if the file doesn't already end with one
            }
        }
        lines.push(new_line);
    }

    let mut result = lines.join("\n");
    // Ensure file ends with a newline
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Format a value for TOML: numbers and booleans go unquoted,
/// everything else gets double-quoted.
fn format_toml_value(value: &str) -> String {
    // Check if it's a number (integer or float)
    if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
        return value.to_string();
    }
    // Check for booleans
    if value == "true" || value == "false" {
        return value.to_string();
    }
    // Default: quote it
    format!("\"{value}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_module_glob_match() {
        assert!(glob_match("cargo *", "cargo test"));
        assert!(!glob_match("cargo *", "rustc build"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "other"));
    }

    #[test]
    fn test_config_module_permission_check() {
        let perms = PermissionConfig {
            allow: vec!["cargo *".to_string()],
            deny: vec!["rm *".to_string()],
        };
        assert_eq!(perms.check("cargo test"), Some(true));
        assert_eq!(perms.check("rm -rf /"), Some(false));
        assert_eq!(perms.check("python script.py"), None);
    }

    #[test]
    fn test_config_module_parse_toml_array() {
        let result = parse_toml_array(r#"["one", "two", "three"]"#);
        assert_eq!(result, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_config_module_parse_permissions() {
        let content = r#"
[permissions]
allow = ["cargo *", "git *"]
deny = ["rm *"]
"#;
        let config = parse_permissions_from_config(content);
        assert_eq!(config.allow, vec!["cargo *", "git *"]);
        assert_eq!(config.deny, vec!["rm *"]);
    }

    #[test]
    fn test_config_module_parse_directories() {
        let content = r#"
[directories]
allow = ["/home/user/project"]
deny = ["/etc"]
"#;
        let config = parse_directories_from_config(content);
        assert_eq!(config.allow, vec!["/home/user/project"]);
        assert_eq!(config.deny, vec!["/etc"]);
    }

    #[test]
    fn test_config_module_parse_mcp_servers() {
        let content = r#"
[mcp_servers.test]
command = "npx"
args = ["-y", "test-server"]
env = { API_KEY = "secret" }
"#;
        let servers = parse_mcp_servers_from_config(content);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test");
        assert_eq!(servers[0].command, "npx");
        assert_eq!(servers[0].args, vec!["-y", "test-server"]);
        assert_eq!(
            servers[0].env,
            vec![("API_KEY".to_string(), "secret".to_string())]
        );
    }

    #[test]
    fn test_config_module_strip_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
        assert_eq!(strip_quotes("'hello'"), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
        assert_eq!(strip_quotes("\"\""), "");
        assert_eq!(strip_quotes(""), "");
    }

    #[test]
    fn test_config_module_parse_inline_table() {
        let result = parse_inline_table(r#"{ KEY = "value", OTHER = "val2" }"#);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("KEY".to_string(), "value".to_string()));
        assert_eq!(result[1], ("OTHER".to_string(), "val2".to_string()));
    }

    #[test]
    fn test_config_module_parse_inline_table_empty() {
        let result = parse_inline_table("{}");
        assert!(result.is_empty());

        let result = parse_inline_table("not a table");
        assert!(result.is_empty());
    }

    #[test]
    fn test_config_module_resolve_path_normalizes_parent_dir() {
        let resolved = resolve_path("/tmp/a/../b");
        assert_eq!(resolved, "/tmp/b");
    }

    #[test]
    fn test_config_module_resolve_path_absolute() {
        let resolved = resolve_path("/usr/bin/env");
        assert!(resolved.starts_with('/'));
        assert!(resolved.contains("usr"));
    }

    #[test]
    fn test_config_module_path_is_under_basic() {
        assert!(path_is_under("/etc/passwd", "/etc"));
        assert!(path_is_under("/etc", "/etc"));
        assert!(!path_is_under("/etcetc", "/etc"));
        assert!(!path_is_under("/tmp/file", "/etc"));
    }

    // --- write_config_value / set_toml_key tests ---

    #[test]
    fn test_set_toml_key_creates_new_key() {
        let content = "# yoyo config\nprovider = \"anthropic\"\n";
        let result = set_toml_key(content, "model", "claude-sonnet-4-6");
        assert!(result.contains("model = \"claude-sonnet-4-6\""));
        // Original key should still be there
        assert!(result.contains("provider = \"anthropic\""));
        // Comment should be preserved
        assert!(result.contains("# yoyo config"));
    }

    #[test]
    fn test_set_toml_key_replaces_existing_key() {
        let content = "provider = \"anthropic\"\nmodel = \"old-model\"\n";
        let result = set_toml_key(content, "model", "new-model");
        assert!(result.contains("model = \"new-model\""));
        assert!(!result.contains("old-model"));
        assert!(result.contains("provider = \"anthropic\""));
    }

    #[test]
    fn test_set_toml_key_preserves_comments() {
        let content = "# My config\n# model choice\nmodel = \"old\"\n# end\n";
        let result = set_toml_key(content, "model", "new");
        assert!(result.contains("# My config"));
        assert!(result.contains("# model choice"));
        assert!(result.contains("# end"));
        assert!(result.contains("model = \"new\""));
    }

    #[test]
    fn test_set_toml_key_numeric_value_unquoted() {
        let result = set_toml_key("", "max_tokens", "8192");
        assert!(result.contains("max_tokens = 8192"));
        assert!(!result.contains("\"8192\""));
    }

    #[test]
    fn test_set_toml_key_string_value_quoted() {
        let result = set_toml_key("", "model", "claude-opus-4-6");
        assert!(result.contains("model = \"claude-opus-4-6\""));
    }

    #[test]
    fn test_set_toml_key_empty_content() {
        let result = set_toml_key("", "provider", "anthropic");
        assert!(result.contains("provider = \"anthropic\""));
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn test_validate_config_value_valid_keys() {
        assert!(validate_config_value("model", "claude-sonnet-4-6").is_ok());
        assert!(validate_config_value("provider", "anthropic").is_ok());
        assert!(validate_config_value("thinking", "high").is_ok());
        assert!(validate_config_value("thinking", "off").is_ok());
        assert!(validate_config_value("temperature", "0.7").is_ok());
        assert!(validate_config_value("max_tokens", "4096").is_ok());
        assert!(validate_config_value("max_turns", "50").is_ok());
    }

    #[test]
    fn test_validate_config_value_invalid() {
        assert!(validate_config_value("model", "").is_err());
        assert!(validate_config_value("thinking", "extreme").is_err());
        assert!(validate_config_value("temperature", "5.0").is_err());
        assert!(validate_config_value("temperature", "abc").is_err());
        assert!(validate_config_value("max_tokens", "0").is_err());
        assert!(validate_config_value("max_tokens", "-1").is_err());
        assert!(validate_config_value("unknown_key", "val").is_err());
    }

    #[test]
    fn test_validate_config_thinking_aliases() {
        assert_eq!(validate_config_value("thinking", "off").unwrap(), "none");
        assert_eq!(validate_config_value("thinking", "minimal").unwrap(), "low");
        assert_eq!(validate_config_value("thinking", "med").unwrap(), "medium");
        assert_eq!(validate_config_value("thinking", "max").unwrap(), "high");
    }

    #[test]
    fn test_write_config_value_to_creates_file() {
        let tmp = std::env::temp_dir().join("yoyo_test_write_config_create");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join(".yoyo.toml");
        let _ = std::fs::remove_file(&path);

        let result = write_config_value_to("model", "test-model", &path);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("model = \"test-model\""));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_write_config_value_to_updates_existing() {
        let tmp = std::env::temp_dir().join("yoyo_test_write_config_update");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join(".yoyo.toml");
        std::fs::write(
            &path,
            "# config\nprovider = \"anthropic\"\nmodel = \"old-model\"\n",
        )
        .unwrap();

        let result = write_config_value_to("model", "new-model", &path);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("model = \"new-model\""));
        assert!(!content.contains("old-model"));
        assert!(content.contains("provider = \"anthropic\""));
        assert!(content.contains("# config"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_write_config_value_to_preserves_other_keys() {
        let tmp = std::env::temp_dir().join("yoyo_test_write_config_preserve");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join(".yoyo.toml");
        std::fs::write(
            &path,
            "provider = \"anthropic\"\nthinking = \"high\"\ntemperature = 0.5\n",
        )
        .unwrap();

        let result = write_config_value_to("model", "new-model", &path);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("model = \"new-model\""));
        assert!(content.contains("provider = \"anthropic\""));
        assert!(content.contains("thinking = \"high\""));
        assert!(content.contains("temperature = 0.5"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_format_toml_value() {
        assert_eq!(format_toml_value("hello"), "\"hello\"");
        assert_eq!(format_toml_value("42"), "42");
        assert_eq!(format_toml_value("3.14"), "3.14");
        assert_eq!(format_toml_value("true"), "true");
        assert_eq!(format_toml_value("false"), "false");
        assert_eq!(
            format_toml_value("claude-sonnet-4-6"),
            "\"claude-sonnet-4-6\""
        );
    }
}
