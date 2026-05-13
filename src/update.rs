/// Compare two version strings (e.g. "0.1.5" vs "0.2.0").
/// Returns true if `latest` is strictly newer than `current`.
pub fn version_is_newer(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let cur = parse(current);
    let lat = parse(latest);
    let len = cur.len().max(lat.len());
    for i in 0..len {
        let c = cur.get(i).copied().unwrap_or(0);
        let l = lat.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// Check GitHub for a newer release. Returns `Some("x.y.z")` if a newer version
/// exists, `None` if current or on any error. Uses a 3-second timeout to avoid
/// blocking startup.
///
/// `current_version` is the running binary's version (e.g. `cli::VERSION`).
pub fn check_for_update(current_version: &str) -> Option<String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sf",
            "--max-time",
            "3",
            "https://api.github.com/repos/yologdev/yoyo-evolve/releases/latest",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8(output.stdout).ok()?;

    // Simple JSON extraction: find "tag_name": "v0.1.5"
    let tag = body
        .split("\"tag_name\"")
        .nth(1)?
        .split('"')
        .find(|s| !s.is_empty() && *s != ":" && *s != ": ")?;

    let latest = tag.strip_prefix('v').unwrap_or(tag);

    if version_is_newer(current_version, latest) {
        Some(latest.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_newer_basic() {
        assert!(version_is_newer("0.1.5", "0.2.0"));
    }

    #[test]
    fn test_version_is_newer_same() {
        assert!(!version_is_newer("0.1.5", "0.1.5"));
    }

    #[test]
    fn test_version_is_newer_older() {
        assert!(!version_is_newer("0.2.0", "0.1.5"));
    }

    #[test]
    fn test_version_is_newer_numeric_comparison() {
        // Must compare numerically, not lexicographically
        assert!(version_is_newer("0.1.5", "0.1.10"));
    }

    #[test]
    fn test_version_is_newer_major_dominates() {
        assert!(!version_is_newer("1.0.0", "0.99.99"));
    }

    #[test]
    fn test_version_is_newer_different_lengths() {
        assert!(version_is_newer("0.1", "0.1.1"));
        assert!(!version_is_newer("0.1.1", "0.1"));
    }

    #[test]
    fn test_version_is_newer_0_1_8_to_0_1_11() {
        // The actual upgrade path for this release
        assert!(version_is_newer("0.1.8", "0.1.11"));
        assert!(!version_is_newer("0.1.11", "0.1.8"));
    }

    #[test]
    fn test_check_for_update_graceful_failure() {
        // When curl isn't available or network fails, should return None
        // We can't control the network in tests, but we can verify it doesn't panic
        let _result = check_for_update("0.1.0");
        // Just assert it doesn't panic — the result depends on network state
    }
}
