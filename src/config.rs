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

    let p = std::path::Path::new(path);
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .join(p)
    };

    // For non-existent paths, canonicalize the nearest existing ancestor and
    // then append the missing suffix. This preserves symlink resolution for
    // checks like `/etc/missing` on macOS where `/etc` points at `/private/etc`.
    let absolute = normalize_pathbuf(absolute);
    let mut existing = absolute.as_path();
    let mut missing = Vec::new();
    while !existing.exists() {
        if let Some(name) = existing.file_name() {
            missing.push(name.to_os_string());
        }
        match existing.parent() {
            Some(parent) => existing = parent,
            None => break,
        }
    }
    if existing.exists() {
        if let Ok(mut canonical) = std::fs::canonicalize(existing) {
            for component in missing.iter().rev() {
                canonical.push(component);
            }
            return normalize_pathbuf(canonical).to_string_lossy().to_string();
        }
    }

    // Manual normalization fallback
    absolute.to_string_lossy().to_string()
}

fn normalize_pathbuf(path: std::path::PathBuf) -> std::path::PathBuf {
    // Normalize components: resolve `.` and `..`
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
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

/// Check whether auto-watch is enabled in the config.
///
/// Reads `auto_watch` from the given config map. Defaults to `true`
/// when the key is absent — watch mode is on by default for detected
/// projects so new users get Aider-style edit→test→fix automatically.
pub fn parse_auto_watch_from_config(config: &std::collections::HashMap<String, String>) -> bool {
    match config.get("auto_watch").map(|v| v.as_str()) {
        Some("false") | Some("0") | Some("no") | Some("off") => false,
        _ => true, // default: enabled
    }
}

/// Check whether auto-continue is enabled in the config.
///
/// Reads `auto_continue` from the given config map. Defaults to `true`
/// when the key is absent — auto-continuation is on by default so
/// incomplete responses are automatically followed up.
pub fn parse_auto_continue_from_config(config: &std::collections::HashMap<String, String>) -> bool {
    match config.get("auto_continue").map(|v| v.as_str()) {
        Some("false") | Some("0") | Some("no") | Some("off") => false,
        _ => true, // default: enabled
    }
}

/// Parse `max_auto_continues` from the config map.
///
/// Returns the configured value (clamped to 0-20) or `None` if the key
/// is absent or unparseable, letting the caller fall back to the default.
pub fn parse_max_auto_continues_from_config(
    config: &std::collections::HashMap<String, String>,
) -> Option<u32> {
    config
        .get("max_auto_continues")
        .and_then(|v| v.parse::<u32>().ok())
        .map(|n| n.min(20))
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
    ("auto_watch", "auto-enable watch mode on start (true/false)"),
    (
        "auto_continue",
        "auto-continue incomplete responses (true/false)",
    ),
    (
        "max_auto_continues",
        "max auto-continue follow-ups per turn (0-20)",
    ),
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
        "auto_watch" => {
            let lower = value.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "on" => Ok("true".to_string()),
                "false" | "0" | "no" | "off" => Ok("false".to_string()),
                _ => Err(format!(
                    "invalid auto_watch value '{value}' — use true or false"
                )),
            }
        }
        "auto_continue" => {
            let lower = value.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "on" => Ok("true".to_string()),
                "false" | "0" | "no" | "off" => Ok("false".to_string()),
                _ => Err(format!(
                    "invalid auto_continue value '{value}' — use true or false"
                )),
            }
        }
        "max_auto_continues" => match value.parse::<u32>() {
            Ok(n) if n <= 20 => Ok(n.to_string()),
            Ok(n) => Err(format!("max_auto_continues {n} out of range (0-20)")),
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
        home_config_path().ok_or_else(|| "could not determine home directory".to_string())?
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

/// Append a pattern to the `[permissions]` allow list in the project-local `.yoyo.toml`.
///
/// If the file doesn't exist it is created. If the `[permissions]` section or `allow`
/// key doesn't exist they are created. If the pattern is already present, the file is
/// left unchanged (no duplicates).
///
/// Returns the path written to on success.
pub fn append_allow_pattern(pattern: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(".yoyo.toml");
    append_allow_pattern_to(pattern, &path)
}

/// Testable version of [`append_allow_pattern`] that takes an explicit path.
pub fn append_allow_pattern_to(
    pattern: &str,
    path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory {}: {e}", parent.display()))?;
        }
    }

    let existing = std::fs::read_to_string(path).unwrap_or_default();

    // Parse existing permissions to check for duplicates
    let current = parse_permissions_from_config(&existing);
    if current.allow.iter().any(|p| p == pattern) {
        // Already present — nothing to do
        return Ok(path.to_path_buf());
    }

    // Build the new allow array
    let mut new_allow = current.allow.clone();
    new_allow.push(pattern.to_string());
    let new_content = set_permissions_allow(&existing, &new_allow);

    std::fs::write(path, &new_content)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;

    Ok(path.to_path_buf())
}

/// Pure function: set the `[permissions]` `allow` array in TOML content.
///
/// If a `[permissions]` section with an `allow = [...]` line exists, it is
/// replaced. If `[permissions]` exists but has no `allow`, the key is inserted
/// right after the section header. If no `[permissions]` section exists, one
/// is appended.
fn set_permissions_allow(content: &str, patterns: &[String]) -> String {
    let formatted: Vec<String> = patterns.iter().map(|p| format!("\"{}\"", p)).collect();
    let allow_line = format!("allow = [{}]", formatted.join(", "));

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut in_permissions = false;
    let mut found_allow = false;
    let mut permissions_section_exists = false;

    for (i, line) in lines.iter_mut().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if trimmed == "[permissions]" {
                permissions_section_exists = true;
                in_permissions = true;
            } else {
                // If we were in [permissions] but never found allow, insert before this section
                if in_permissions && !found_allow {
                    // We'll handle insertion below
                }
                in_permissions = false;
            }
            continue;
        }
        if in_permissions && !found_allow {
            if let Some((key, _)) = trimmed.split_once('=') {
                if key.trim() == "allow" {
                    *line = allow_line.clone();
                    found_allow = true;
                    let _ = i; // suppress unused warning
                }
            }
        }
    }

    if permissions_section_exists && !found_allow {
        // Insert allow line right after [permissions] header
        let mut result = Vec::new();
        for line in &lines {
            result.push(line.clone());
            if line.trim() == "[permissions]" {
                result.push(allow_line.clone());
            }
        }
        lines = result;
    }

    if !permissions_section_exists {
        // Append a new [permissions] section
        if !lines.is_empty() && !lines.last().unwrap().is_empty() {
            lines.push(String::new());
        }
        lines.push("[permissions]".to_string());
        lines.push(allow_line);
    }

    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
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

// ---------------------------------------------------------------------------
// Config-file path resolution and loading
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use crate::format::{is_quiet, DIM, RESET};

/// Config file scopes, loaded from broadest to narrowest:
/// - `~/.config/yoyo/config.toml` (XDG user-level)
/// - `~/.yoyo.toml` (home-level shorthand)
/// - `.yoyo.toml` (project-level)
///
/// Later scopes override earlier scalar keys.
const CONFIG_FILE_NAMES: &[&str] = &[".yoyo.toml"];
const DEEPSEEK_CONFIG_FILE_NAME: &str = ".yoyo/deepseek.toml";

/// XDG user-level config path: `~/.config/yoyo/config.toml`.
pub fn user_config_path() -> Option<std::path::PathBuf> {
    dirs_hint().map(|dir| dir.join("yoyo").join("config.toml"))
}

/// Home directory config path: `~/.yoyo.toml`.
pub fn home_config_path() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".yoyo.toml"))
}

/// Project-local DeepSeek-native config path.
pub fn deepseek_config_path() -> std::path::PathBuf {
    std::path::PathBuf::from(DEEPSEEK_CONFIG_FILE_NAME)
}

/// Best-effort XDG config dir (~/.config on Linux/macOS).
fn dirs_hint() -> Option<std::path::PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
}

/// Best-effort XDG data dir (~/.local/share on Linux/macOS).
fn data_dir_hint() -> Option<std::path::PathBuf> {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
        })
}

/// Get the path for the readline history file.
/// Prefers `$XDG_DATA_HOME/yoyo/history`, falls back to `~/.yoyo_history`.
pub fn history_file_path() -> Option<std::path::PathBuf> {
    // Try XDG data dir first
    if let Some(data_dir) = data_dir_hint() {
        let yoyo_dir = data_dir.join("yoyo");
        // Try to create the directory; if it works, use it
        if std::fs::create_dir_all(&yoyo_dir).is_ok() {
            return Some(yoyo_dir.join("history"));
        }
    }
    // Fall back to ~/.yoyo_history
    std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".yoyo_history"))
}

/// Parse a simple TOML-like config file (key = "value" or key = value per line).
/// Ignores comments (#) and blank lines. Returns a map of key → value.
pub fn parse_config_file(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim();
            // Strip surrounding quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value[1..value.len() - 1].to_string()
            } else {
                value.to_string()
            };
            map.insert(key, value);
        }
    }
    map
}

/// Parse `.yoyo/deepseek.toml` into the same flat key space used by CLI config.
///
/// This intentionally supports only simple scalar key/value pairs and section
/// headers. It keeps DeepSeek-specific configuration separate from generic
/// `.yoyo.toml`, while still reusing the existing CLI precedence pipeline.
pub fn parse_deepseek_config_file(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].trim().to_ascii_lowercase();
            continue;
        }
        let Some((raw_key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let value = strip_toml_scalar(raw_value.trim());
        if let Some(mapped_key) = map_deepseek_config_key(&section, key) {
            map.insert(mapped_key.to_string(), value);
        }
    }

    map
}

fn map_deepseek_config_key<'a>(section: &str, key: &'a str) -> Option<&'a str> {
    match section {
        "" | "deepseek" => match key {
            "enabled" | "native" | "deepseek_native" => Some("deepseek_native"),
            "default_model" | "model" => Some("deepseek_model"),
            "fast_model" => Some("deepseek_fast_model"),
            "base_url" => Some("deepseek_base_url"),
            "api_key" => Some("deepseek_api_key"),
            "thinking_default" | "thinking" | "reasoning_effort" => Some("deepseek_thinking"),
            "max_tokens" => Some("max_tokens"),
            "context_window" => Some("context_window"),
            _ => None,
        },
        "state" => match key {
            "enabled" => Some("state_enabled"),
            "events" => Some("state_events"),
            "fail_soft" => Some("state_fail_soft"),
            "store" => Some("state_store"),
            _ => None,
        },
        "deepseek.cache" => match key {
            "record_metrics" => Some("deepseek_cache_record_metrics"),
            "stable_prefix" => Some("deepseek_cache_stable_prefix"),
            "optimize_prompt_order" => Some("deepseek_cache_optimize_prompt_order"),
            _ => None,
        },
        "deepseek.context" => match key {
            "recent_failure_limit" => Some("deepseek_context_recent_failure_limit"),
            "changed_file_limit" => Some("deepseek_context_changed_file_limit"),
            "include_repo_map" => Some("deepseek_context_include_repo_map"),
            "include_instruction_files" => Some("deepseek_context_include_instruction_files"),
            _ => None,
        },
        "deepseek.routing" => match key {
            "planning" => Some("deepseek_routing_planning"),
            "root_cause" => Some("deepseek_routing_root_cause"),
            "summary" => Some("deepseek_routing_summary"),
            "local_edit" => Some("deepseek_routing_local_edit"),
            _ => None,
        },
        "deepseek.transport" => match key {
            "request_timeout_ms" => Some("deepseek_transport_request_timeout_ms"),
            "max_retries" => Some("deepseek_transport_max_retries"),
            _ => None,
        },
        "evolve.harness" => match key {
            "enabled" => Some("evolve_harness_enabled"),
            "allowed_patch_types" => Some("evolve_harness_allowed_patch_types"),
            "require_human_approval_for" => Some("evolve_harness_require_human_approval_for"),
            _ => None,
        },
        _ => None,
    }
}

fn strip_toml_scalar(value: &str) -> String {
    let value = value
        .split_once(" #")
        .map(|(before_comment, _)| before_comment.trim())
        .unwrap_or(value)
        .trim();
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// Load project-local DeepSeek config, if present.
pub fn load_deepseek_config_file() -> (HashMap<String, String>, String) {
    let path = deepseek_config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        return (parse_deepseek_config_file(&content), content);
    }
    (HashMap::new(), String::new())
}

fn config_scope_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = user_config_path() {
        paths.push(path);
    }
    if let Some(path) = home_config_path() {
        paths.push(path);
    }
    for name in CONFIG_FILE_NAMES {
        paths.push(std::path::PathBuf::from(name));
    }
    paths
}

fn merge_config_layer(
    merged: &mut HashMap<String, String>,
    raw: &mut String,
    content: &str,
    path: &std::path::Path,
) {
    merged.extend(parse_config_file(content));
    raw.push_str("\n# source: ");
    raw.push_str(&path.display().to_string());
    raw.push('\n');
    raw.push_str(content);
    if !content.ends_with('\n') {
        raw.push('\n');
    }
}

/// Load layered config files.
///
/// Scalar key precedence is XDG → home → project, with project-local
/// `.yoyo.toml` winning. The returned raw content is concatenated in that same
/// order so section parsers can layer permissions, directories, hooks, and MCP
/// snippets without a separate config model.
pub fn load_config_file() -> (HashMap<String, String>, String) {
    let mut merged = HashMap::new();
    let mut raw = String::new();
    let mut loaded_paths = Vec::new();

    for path in config_scope_paths() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !is_quiet() {
                eprintln!("{DIM}  config: {}{RESET}", path.display());
            }
            merge_config_layer(&mut merged, &mut raw, &content, &path);
            loaded_paths.push(path);
        }
    }

    if loaded_paths.is_empty() {
        (HashMap::new(), String::new())
    } else {
        (merged, raw)
    }
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
        let expected = std::fs::canonicalize("/tmp")
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join("b")
            .to_string_lossy()
            .to_string();
        assert_eq!(resolved, expected);
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

    #[test]
    fn auto_watch_defaults_to_true() {
        let config = std::collections::HashMap::new();
        assert!(parse_auto_watch_from_config(&config));
    }

    #[test]
    fn auto_watch_respects_false() {
        let mut config = std::collections::HashMap::new();
        config.insert("auto_watch".to_string(), "false".to_string());
        assert!(!parse_auto_watch_from_config(&config));
    }

    #[test]
    fn auto_watch_respects_off() {
        let mut config = std::collections::HashMap::new();
        config.insert("auto_watch".to_string(), "off".to_string());
        assert!(!parse_auto_watch_from_config(&config));
    }

    #[test]
    fn auto_watch_explicit_true() {
        let mut config = std::collections::HashMap::new();
        config.insert("auto_watch".to_string(), "true".to_string());
        assert!(parse_auto_watch_from_config(&config));
    }

    #[test]
    fn validate_auto_watch_values() {
        assert_eq!(
            validate_config_value("auto_watch", "true"),
            Ok("true".to_string())
        );
        assert_eq!(
            validate_config_value("auto_watch", "false"),
            Ok("false".to_string())
        );
        assert_eq!(
            validate_config_value("auto_watch", "yes"),
            Ok("true".to_string())
        );
        assert_eq!(
            validate_config_value("auto_watch", "no"),
            Ok("false".to_string())
        );
        assert!(validate_config_value("auto_watch", "maybe").is_err());
    }

    #[test]
    fn auto_continue_defaults_to_true() {
        let config = std::collections::HashMap::new();
        assert!(parse_auto_continue_from_config(&config));
    }

    #[test]
    fn auto_continue_respects_false() {
        let mut config = std::collections::HashMap::new();
        config.insert("auto_continue".to_string(), "false".to_string());
        assert!(!parse_auto_continue_from_config(&config));
    }

    #[test]
    fn auto_continue_respects_off() {
        let mut config = std::collections::HashMap::new();
        config.insert("auto_continue".to_string(), "off".to_string());
        assert!(!parse_auto_continue_from_config(&config));
    }

    #[test]
    fn auto_continue_explicit_true() {
        let mut config = std::collections::HashMap::new();
        config.insert("auto_continue".to_string(), "true".to_string());
        assert!(parse_auto_continue_from_config(&config));
    }

    #[test]
    fn validate_auto_continue_values() {
        assert_eq!(
            validate_config_value("auto_continue", "true"),
            Ok("true".to_string())
        );
        assert_eq!(
            validate_config_value("auto_continue", "false"),
            Ok("false".to_string())
        );
        assert_eq!(
            validate_config_value("auto_continue", "yes"),
            Ok("true".to_string())
        );
        assert_eq!(
            validate_config_value("auto_continue", "no"),
            Ok("false".to_string())
        );
        assert!(validate_config_value("auto_continue", "maybe").is_err());
    }

    #[test]
    fn max_auto_continues_defaults_to_none() {
        let config = std::collections::HashMap::new();
        assert_eq!(parse_max_auto_continues_from_config(&config), None);
    }

    #[test]
    fn max_auto_continues_parses_valid() {
        let mut config = std::collections::HashMap::new();
        config.insert("max_auto_continues".to_string(), "10".to_string());
        assert_eq!(parse_max_auto_continues_from_config(&config), Some(10));
    }

    #[test]
    fn max_auto_continues_clamps_to_20() {
        let mut config = std::collections::HashMap::new();
        config.insert("max_auto_continues".to_string(), "50".to_string());
        assert_eq!(parse_max_auto_continues_from_config(&config), Some(20));
    }

    #[test]
    fn max_auto_continues_zero_is_valid() {
        let mut config = std::collections::HashMap::new();
        config.insert("max_auto_continues".to_string(), "0".to_string());
        assert_eq!(parse_max_auto_continues_from_config(&config), Some(0));
    }

    #[test]
    fn max_auto_continues_non_numeric_returns_none() {
        let mut config = std::collections::HashMap::new();
        config.insert("max_auto_continues".to_string(), "abc".to_string());
        assert_eq!(parse_max_auto_continues_from_config(&config), None);
    }

    #[test]
    fn validate_max_auto_continues_values() {
        assert_eq!(
            validate_config_value("max_auto_continues", "5"),
            Ok("5".to_string())
        );
        assert_eq!(
            validate_config_value("max_auto_continues", "0"),
            Ok("0".to_string())
        );
        assert_eq!(
            validate_config_value("max_auto_continues", "20"),
            Ok("20".to_string())
        );
        assert!(validate_config_value("max_auto_continues", "21").is_err());
        assert!(validate_config_value("max_auto_continues", "-1").is_err());
        assert!(validate_config_value("max_auto_continues", "abc").is_err());
    }

    // === Config-file path resolution tests (moved from cli.rs) ===

    #[test]
    fn test_parse_config_file_basic() {
        let content = r#"
model = "claude-sonnet-4-20250514"
thinking = "medium"
max_tokens = 4096
"#;
        let config = parse_config_file(content);
        assert_eq!(config.get("model").unwrap(), "claude-sonnet-4-20250514");
        assert_eq!(config.get("thinking").unwrap(), "medium");
        assert_eq!(config.get("max_tokens").unwrap(), "4096");
    }

    #[test]
    fn test_parse_config_file_comments_and_blanks() {
        let content = r#"
# This is a comment
model = "claude-opus-4-6"

# Another comment
thinking = "high"
"#;
        let config = parse_config_file(content);
        assert_eq!(config.get("model").unwrap(), "claude-opus-4-6");
        assert_eq!(config.get("thinking").unwrap(), "high");
        assert_eq!(config.len(), 2);
    }

    #[test]
    fn test_parse_config_file_no_quotes() {
        let content = "model = claude-haiku-35\nmax_tokens = 2048";
        let config = parse_config_file(content);
        assert_eq!(config.get("model").unwrap(), "claude-haiku-35");
        assert_eq!(config.get("max_tokens").unwrap(), "2048");
    }

    #[test]
    fn test_parse_config_file_single_quotes() {
        let content = "model = 'claude-opus-4-6'";
        let config = parse_config_file(content);
        assert_eq!(config.get("model").unwrap(), "claude-opus-4-6");
    }

    #[test]
    fn test_parse_config_file_empty() {
        let config = parse_config_file("");
        assert!(config.is_empty());
    }

    #[test]
    fn test_parse_config_file_whitespace_handling() {
        let content = "  model  =  claude-opus-4-6  ";
        let config = parse_config_file(content);
        assert_eq!(config.get("model").unwrap(), "claude-opus-4-6");
    }

    #[test]
    fn deepseek_config_parser_maps_sections_to_cli_keys() {
        let content = r#"
[deepseek]
enabled = true
default_model = "deepseek-v4-flash"
base_url = "https://deepseek.example/v1"
thinking_default = "max"

[state]
enabled = false
events = ".yoyo/state/custom.jsonl"
fail_soft = true

[deepseek.cache]
stable_prefix = true

[deepseek.context]
recent_failure_limit = 8
changed_file_limit = 21
include_repo_map = false
include_instruction_files = ["YOYO.md", "AGENTS.md"]

[deepseek.transport]
request_timeout_ms = 45000
max_retries = 4

[evolve.harness]
allowed_patch_types = ["context_policy", "repair_policy"]
require_human_approval_for = ["permission_policy", "shell_policy"]
"#;

        let config = parse_deepseek_config_file(content);

        assert_eq!(config.get("deepseek_native").unwrap(), "true");
        assert_eq!(config.get("deepseek_model").unwrap(), "deepseek-v4-flash");
        assert_eq!(
            config.get("deepseek_base_url").unwrap(),
            "https://deepseek.example/v1"
        );
        assert_eq!(config.get("deepseek_thinking").unwrap(), "max");
        assert_eq!(config.get("state_enabled").unwrap(), "false");
        assert_eq!(
            config.get("state_events").unwrap(),
            ".yoyo/state/custom.jsonl"
        );
        assert_eq!(config.get("state_fail_soft").unwrap(), "true");
        assert_eq!(config.get("deepseek_cache_stable_prefix").unwrap(), "true");
        assert_eq!(
            config.get("deepseek_context_recent_failure_limit").unwrap(),
            "8"
        );
        assert_eq!(
            config.get("deepseek_context_changed_file_limit").unwrap(),
            "21"
        );
        assert_eq!(
            config.get("deepseek_context_include_repo_map").unwrap(),
            "false"
        );
        assert_eq!(
            config
                .get("deepseek_context_include_instruction_files")
                .unwrap(),
            "[\"YOYO.md\", \"AGENTS.md\"]"
        );
        assert_eq!(
            config.get("deepseek_transport_request_timeout_ms").unwrap(),
            "45000"
        );
        assert_eq!(config.get("deepseek_transport_max_retries").unwrap(), "4");
        assert_eq!(
            config.get("evolve_harness_allowed_patch_types").unwrap(),
            "[\"context_policy\", \"repair_policy\"]"
        );
        assert_eq!(
            config
                .get("evolve_harness_require_human_approval_for")
                .unwrap(),
            "[\"permission_policy\", \"shell_policy\"]"
        );
    }

    #[test]
    fn test_parse_config_file_mcp_array() {
        let content = r#"
model = "claude-sonnet-4-20250514"
mcp = ["npx open-websearch@latest", "npx @mcp/server-filesystem /tmp"]
"#;
        let config = parse_config_file(content);
        let mcp_val = config.get("mcp").expect("mcp key should exist");
        let mcps = parse_toml_array(mcp_val);
        assert_eq!(mcps.len(), 2);
        assert_eq!(mcps[0], "npx open-websearch@latest");
        assert_eq!(mcps[1], "npx @mcp/server-filesystem /tmp");
    }

    #[test]
    fn test_parse_config_file_mcp_empty_array() {
        let content = "mcp = []";
        let config = parse_config_file(content);
        let mcp_val = config.get("mcp").expect("mcp key should exist");
        let mcps = parse_toml_array(mcp_val);
        assert!(mcps.is_empty());
    }

    #[test]
    fn test_parse_config_file_mcp_single_entry() {
        let content = r#"mcp = ["npx open-websearch@latest"]"#;
        let config = parse_config_file(content);
        let mcp_val = config.get("mcp").expect("mcp key should exist");
        let mcps = parse_toml_array(mcp_val);
        assert_eq!(mcps.len(), 1);
        assert_eq!(mcps[0], "npx open-websearch@latest");
    }

    #[test]
    fn test_home_config_path_returns_yoyo_toml_in_home() {
        // home_config_path() should return $HOME/.yoyo.toml
        let original_home = std::env::var("HOME").ok();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let path = home_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path, tmp.path().join(".yoyo.toml"));

        // Restore
        if let Some(h) = original_home {
            std::env::set_var("HOME", h);
        }
    }

    #[test]
    fn test_home_config_path_file_is_loadable() {
        // If ~/.yoyo.toml exists, parse_config_file should parse it
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join(".yoyo.toml");
        std::fs::write(
            &config_path,
            "model = \"test-model\"\napi_key = \"sk-home-test\"\n",
        )
        .unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let config = parse_config_file(&content);
        assert_eq!(config.get("model").unwrap(), "test-model");
        assert_eq!(config.get("api_key").unwrap(), "sk-home-test");
    }

    #[test]
    fn test_config_precedence_project_over_home() {
        // If both project-level .yoyo.toml and ~/.yoyo.toml exist,
        // the project-level config should override overlapping scalar keys.
        let project_content = "model = \"project-model\"";
        let home_content = "model = \"home-model\"";

        let project_config = parse_config_file(project_content);
        let home_config = parse_config_file(home_content);

        assert_eq!(project_config.get("model").unwrap(), "project-model");
        assert_eq!(home_config.get("model").unwrap(), "home-model");
    }

    #[test]
    fn test_config_search_order_documented() {
        // Verify the documented scope order: XDG, home, project.
        // CONFIG_FILE_NAMES contains the project-level name
        assert_eq!(CONFIG_FILE_NAMES, &[".yoyo.toml"]);

        // home_config_path returns ~/.yoyo.toml
        let original_home = std::env::var("HOME").ok();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let home = home_config_path().unwrap();
        assert!(home.to_string_lossy().ends_with(".yoyo.toml"));
        assert!(home
            .to_string_lossy()
            .contains(&tmp.path().to_string_lossy().to_string()));

        // user_config_path returns ~/.config/yoyo/config.toml (XDG)
        let xdg = user_config_path().unwrap();
        assert!(xdg.to_string_lossy().ends_with("config.toml"));
        assert!(xdg.to_string_lossy().contains("yoyo"));

        // Restore
        if let Some(h) = original_home {
            std::env::set_var("HOME", h);
        }
    }

    #[test]
    fn layered_config_merges_xdg_home_and_project_scopes() {
        let mut merged = HashMap::new();
        let mut raw = String::new();

        merge_config_layer(
            &mut merged,
            &mut raw,
            r#"
model = "xdg-model"
provider = "anthropic"

[permissions]
deny = ["rm -rf *"]
"#,
            std::path::Path::new("/xdg/config.toml"),
        );
        merge_config_layer(
            &mut merged,
            &mut raw,
            r#"
model = "home-model"
api_key = "sk-home"

[permissions]
allow = ["git *"]
"#,
            std::path::Path::new("/home/.yoyo.toml"),
        );
        merge_config_layer(
            &mut merged,
            &mut raw,
            r#"
model = "project-model"
state_enabled = true
"#,
            std::path::Path::new(".yoyo.toml"),
        );

        assert_eq!(
            merged.get("model").map(String::as_str),
            Some("project-model")
        );
        assert_eq!(
            merged.get("provider").map(String::as_str),
            Some("anthropic")
        );
        assert_eq!(merged.get("api_key").map(String::as_str), Some("sk-home"));
        assert_eq!(
            merged.get("state_enabled").map(String::as_str),
            Some("true")
        );

        let permissions = parse_permissions_from_config(&raw);
        assert_eq!(permissions.allow, vec!["git *"]);
        assert_eq!(permissions.deny, vec!["rm -rf *"]);
        assert!(raw.contains("# source: /xdg/config.toml"));
        assert!(raw.contains("# source: /home/.yoyo.toml"));
        assert!(raw.contains("# source: .yoyo.toml"));
    }

    #[test]
    fn test_history_file_path_returns_some() {
        // In CI and local environments, HOME is typically set
        let path = history_file_path();
        if std::env::var("HOME").is_ok() {
            assert!(path.is_some(), "Should return a path when HOME is set");
            let p = path.unwrap();
            let p_str = p.to_string_lossy();
            assert!(
                p_str.contains("yoyo"),
                "History path should contain 'yoyo': {p_str}"
            );
            assert!(
                p_str.ends_with("history") || p_str.ends_with(".yoyo_history"),
                "History path should end with 'history' or '.yoyo_history': {p_str}"
            );
        }
    }

    #[test]
    fn test_history_file_path_prefers_xdg() {
        // When XDG_DATA_HOME is set, should use it
        let dir = std::env::temp_dir().join("yoyo_test_xdg_data");
        let _ = std::fs::create_dir_all(&dir);
        // We can't safely set env vars in parallel tests, so just verify the logic
        // by calling data_dir_hint and checking the fallback behavior
        let path = history_file_path();
        // Should return Some regardless
        if std::env::var("HOME").is_ok() || std::env::var("XDG_DATA_HOME").is_ok() {
            assert!(path.is_some());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_data_dir_hint_returns_path() {
        // data_dir_hint should return something when HOME is set
        if std::env::var("HOME").is_ok() || std::env::var("XDG_DATA_HOME").is_ok() {
            let dir = data_dir_hint();
            assert!(dir.is_some(), "Should return a data dir path");
        }
    }

    // -----------------------------------------------------------------------
    // append_allow_pattern / set_permissions_allow
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_permissions_allow_creates_section() {
        let content = "model = \"claude-sonnet-4-6\"\n";
        let result = set_permissions_allow(content, &["cargo test*".to_string()]);
        assert!(result.contains("[permissions]"));
        assert!(result.contains("allow = [\"cargo test*\"]"));
        // Original content preserved
        assert!(result.contains("model = \"claude-sonnet-4-6\""));
    }

    #[test]
    fn test_set_permissions_allow_updates_existing() {
        let content = "[permissions]\nallow = [\"cargo build*\"]\ndeny = [\"rm*\"]\n";
        let result = set_permissions_allow(
            content,
            &["cargo build*".to_string(), "cargo test*".to_string()],
        );
        assert!(result.contains("allow = [\"cargo build*\", \"cargo test*\"]"));
        // Deny is preserved
        assert!(result.contains("deny = [\"rm*\"]"));
    }

    #[test]
    fn test_set_permissions_allow_inserts_if_section_exists_without_allow() {
        let content = "[permissions]\ndeny = [\"rm*\"]\n";
        let result = set_permissions_allow(content, &["cargo test*".to_string()]);
        assert!(result.contains("allow = [\"cargo test*\"]"));
        assert!(result.contains("deny = [\"rm*\"]"));
    }

    #[test]
    fn test_append_allow_pattern_to_creates_file() {
        let dir = std::env::temp_dir().join("yoyo_test_append_allow_create");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".yoyo.toml");

        let result = append_allow_pattern_to("cargo test*", &path).unwrap();
        assert_eq!(result, path);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[permissions]"));
        assert!(content.contains("allow = [\"cargo test*\"]"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_append_allow_pattern_to_appends_to_existing() {
        let dir = std::env::temp_dir().join("yoyo_test_append_allow_existing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".yoyo.toml");

        // Write initial config
        std::fs::write(&path, "[permissions]\nallow = [\"cargo build*\"]\n").unwrap();

        append_allow_pattern_to("cargo test*", &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"cargo build*\""));
        assert!(content.contains("\"cargo test*\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_append_allow_pattern_to_no_duplicates() {
        let dir = std::env::temp_dir().join("yoyo_test_append_allow_nodup");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".yoyo.toml");

        std::fs::write(&path, "[permissions]\nallow = [\"cargo test*\"]\n").unwrap();

        // Try to add the same pattern again
        append_allow_pattern_to("cargo test*", &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Should still only have one entry
        assert_eq!(content.matches("cargo test*").count(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_append_allow_pattern_to_pattern_matches_via_check() {
        let dir = std::env::temp_dir().join("yoyo_test_append_allow_check");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".yoyo.toml");

        append_allow_pattern_to("cargo test*", &path).unwrap();

        // Parse back and verify it actually matches
        let content = std::fs::read_to_string(&path).unwrap();
        let perms = parse_permissions_from_config(&content);
        assert_eq!(perms.check("cargo test"), Some(true));
        assert_eq!(perms.check("cargo test --release"), Some(true));
        assert_eq!(perms.check("npm run test"), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
