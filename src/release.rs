//! Release identity and asset naming for Yoyo DS Harness.

use serde::Serialize;
use std::path::{Path, PathBuf};

pub const RELEASE_REPO: &str = "yologdev/yyds-harness";
pub const RELEASE_ARCHIVE_PREFIX: &str = "yyds-harness";
pub const PRIMARY_BINARY: &str = "yyds";
pub const SOURCE_PROVENANCE_POLICY_VERSION: u32 = 1;

pub fn releases_api_url() -> String {
    format!("https://api.github.com/repos/{RELEASE_REPO}/releases/latest")
}

pub fn releases_page_url() -> String {
    format!("https://github.com/{RELEASE_REPO}/releases")
}

pub fn install_script_url(windows: bool) -> String {
    let script = if windows { "install.ps1" } else { "install.sh" };
    format!("https://raw.githubusercontent.com/{RELEASE_REPO}/main/{script}")
}

pub fn manual_install_command(windows: bool) -> String {
    let url = install_script_url(windows);
    if windows {
        format!("irm {url} | iex")
    } else {
        format!("curl -fsSL {url} | bash")
    }
}

pub fn platform_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

pub fn platform_asset_name(os: &str, arch: &str, tag_name: &str) -> Option<String> {
    let target = platform_target(os, arch)?;
    let extension = if os == "windows" { "zip" } else { "tar.gz" };
    Some(format!(
        "{RELEASE_ARCHIVE_PREFIX}-{tag_name}-{target}.{extension}"
    ))
}

pub fn packaged_binary_names(windows: bool) -> [&'static str; 1] {
    if windows {
        ["yyds.exe"]
    } else {
        [PRIMARY_BINARY]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceProvenanceAudit {
    pub policy_version: u32,
    pub allowed_reference_domains: Vec<&'static str>,
    pub forbidden_markers: Vec<String>,
    pub scan_source: &'static str,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub findings: Vec<SourceProvenanceFinding>,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceProvenanceFinding {
    pub path: String,
    pub marker: String,
}

pub fn allowed_public_reference_domains() -> Vec<&'static str> {
    vec![
        "github.com/yologdev/yoyo-evolve",
        "api-docs.deepseek.com",
        "code.claude.com/docs",
        "github.com/openai/codex",
    ]
}

pub fn forbidden_source_provenance_markers() -> Vec<String> {
    vec![
        provenance_marker(&["copied", " from ", "leaked"]),
        provenance_marker(&["derived", " from ", "leaked"]),
        provenance_marker(&["based", " on ", "leaked"]),
        provenance_marker(&["vendored", " claude ", "code source"]),
        provenance_marker(&["copied", " claude ", "code implementation"]),
        provenance_marker(&["copied", " codex ", "implementation"]),
    ]
}

fn provenance_marker(parts: &[&str]) -> String {
    parts.concat()
}

#[cfg(test)]
pub fn audit_source_provenance_texts<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> SourceProvenanceAudit {
    audit_source_provenance_owned_texts(
        files
            .into_iter()
            .map(|(path, text)| (path.to_string(), text.to_string())),
    )
}

pub fn audit_source_provenance_repository(root: &Path) -> SourceProvenanceAudit {
    let (paths, scan_source) = repository_source_provenance_paths(root);
    let canonical_root = std::fs::canonicalize(root).ok();
    let mut files = Vec::new();
    let mut skipped_paths = Vec::new();
    let mut escaped_paths = Vec::new();
    for path in paths {
        let full_path = root.join(&path);
        if let Some(canonical_root) = &canonical_root {
            match std::fs::canonicalize(&full_path) {
                Ok(canonical_path) if !canonical_path.starts_with(canonical_root) => {
                    escaped_paths.push(path.display().to_string());
                    continue;
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }
        match std::fs::read_to_string(&full_path) {
            Ok(text) => files.push((path.display().to_string(), text)),
            Err(_) => skipped_paths.push(path.display().to_string()),
        }
    }
    let mut audit = audit_source_provenance_owned_texts(files);
    audit.scan_source = scan_source;
    audit.skipped_files = skipped_paths.len() + escaped_paths.len();
    for path in skipped_paths {
        audit.findings.push(SourceProvenanceFinding {
            path,
            marker: "source file unreadable".to_string(),
        });
    }
    for path in escaped_paths {
        audit.findings.push(SourceProvenanceFinding {
            path,
            marker: "source path escapes repository".to_string(),
        });
    }
    audit.passed = audit.findings.is_empty();
    audit
}

fn audit_source_provenance_owned_texts(
    files: impl IntoIterator<Item = (String, String)>,
) -> SourceProvenanceAudit {
    let forbidden_markers = forbidden_source_provenance_markers();
    let mut findings = Vec::new();
    let mut scanned_files = 0usize;
    for (path, text) in files {
        scanned_files += 1;
        let lower = text.to_ascii_lowercase();
        for marker in &forbidden_markers {
            if lower.contains(marker) {
                findings.push(SourceProvenanceFinding {
                    path: path.clone(),
                    marker: marker.clone(),
                });
            }
        }
    }
    if scanned_files == 0 {
        findings.push(SourceProvenanceFinding {
            path: "<source-provenance-audit>".to_string(),
            marker: "no source files scanned".to_string(),
        });
    }

    SourceProvenanceAudit {
        policy_version: SOURCE_PROVENANCE_POLICY_VERSION,
        allowed_reference_domains: allowed_public_reference_domains(),
        forbidden_markers,
        scan_source: "provided",
        scanned_files,
        skipped_files: 0,
        passed: findings.is_empty(),
        findings,
    }
}

fn repository_source_provenance_paths(root: &Path) -> (Vec<PathBuf>, &'static str) {
    if let Some(paths) = git_source_files(root) {
        (paths, "git")
    } else {
        (recursive_source_files(root), "filesystem")
    }
}

fn git_source_files(root: &Path) -> Option<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg("--cached")
        .arg("--others")
        .arg("--exclude-standard")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .filter(|path| source_provenance_candidate(path))
        .collect::<Vec<_>>();
    Some(files)
}

fn recursive_source_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_recursive_source_files(root, root, &mut out);
    out.sort();
    out
}

fn collect_recursive_source_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git" || name == ".yoyo" || name == "target" {
            continue;
        }
        let Ok(metadata) = entry.file_type() else {
            continue;
        };
        if metadata.is_dir() {
            collect_recursive_source_files(root, &path, out);
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if source_provenance_candidate(relative) {
            out.push(relative.to_path_buf());
        }
    }
}

fn source_provenance_candidate(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("rs")
            | Some("md")
            | Some("toml")
            | Some("yml")
            | Some("yaml")
            | Some("json")
            | Some("sh")
            | Some("ps1")
            | Some("txt")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_asset_names_include_product_prefix_tag_and_target() {
        assert_eq!(
            platform_asset_name("linux", "x86_64", "v0.1.13").as_deref(),
            Some("yyds-harness-v0.1.13-x86_64-unknown-linux-gnu.tar.gz")
        );
        assert_eq!(
            platform_asset_name("windows", "x86_64", "v0.1.13").as_deref(),
            Some("yyds-harness-v0.1.13-x86_64-pc-windows-msvc.zip")
        );
    }

    #[test]
    fn release_urls_point_at_yoyo_ds_harness() {
        assert_eq!(RELEASE_REPO, "yologdev/yyds-harness");
        assert!(releases_api_url().contains("yyds-harness"));
        assert!(manual_install_command(false).contains("yyds-harness"));
        assert!(manual_install_command(true).contains("yyds-harness"));
    }

    #[test]
    fn public_readme_metadata_uses_yoyo_ds_harness_identity() {
        let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
        let readme = std::fs::read_to_string(readme_path).unwrap();

        assert!(readme.contains("@misc{yoyo2026yoyodsharness"));
        assert!(readme.contains("Yoyo DeepSeek Harness:"));
        assert!(readme.contains("github.com/yologdev/yyds-harness"));
        assert!(readme.contains("repos=yologdev/yyds-harness"));
        assert!(readme.contains("www.star-history.com/?type=date&repos=yologdev%2Fyyds-harness"));
        assert!(readme.contains("api.star-history.com/chart?repos=yologdev/yyds-harness&type=date"));
        assert!(!readme.contains("@misc{yoyo2026yoyoevolve"));
        assert!(!readme.contains("github.com/yologdev/yoyo-evolve},"));
        assert!(!readme.contains("repos=yologdev/yoyo-evolve&type=Date"));
    }

    #[test]
    fn public_docs_metadata_uses_yoyo_ds_harness_identity() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let book = std::fs::read_to_string(root.join("docs/book.toml")).unwrap();
        let introduction = std::fs::read_to_string(root.join("docs/src/introduction.md")).unwrap();
        let fork_guide = std::fs::read_to_string(root.join("docs/src/guides/fork.md")).unwrap();
        let readme = std::fs::read_to_string(root.join("README.md")).unwrap();

        assert!(book.contains("Yoyo DeepSeek Harness documentation"));
        assert!(book.contains("git-repository-url = \"https://github.com/yologdev/yyds-harness\""));
        assert!(!book.contains("yoyo-evolve"));

        assert!(introduction.contains("# Yoyo DeepSeek Harness"));
        assert!(introduction.contains("cargo install yoyo-ds-harness"));
        assert!(introduction.contains("yyds"));
        assert!(introduction.contains("github.com/yologdev/yyds-harness"));
        assert!(introduction.contains("gen0 is `yologdev/yoyo-evolve`"));
        assert!(!introduction.contains("github.com/yologdev/yoyo-evolve"));

        assert!(fork_guide.contains("Fork Yoyo DeepSeek Harness"));
        assert!(fork_guide.contains("github.com/yologdev/yyds-harness"));
        assert!(fork_guide.contains("LINEAGE.md"));
        assert!(!fork_guide.contains("github.com/yologdev/yoyo-evolve"));

        assert!(
            readme.contains("[yologdev/yyds-harness](https://github.com/yologdev/yyds-harness)")
        );
        assert!(readme.contains("docs/src/guides/fork.md"));
        assert!(!readme.contains("yologdev.github.io/yoyo-evolve/book/guides/fork.html"));
    }

    #[test]
    fn source_provenance_audit_allows_public_reference_policy_text() {
        let audit = audit_source_provenance_texts([(
            "docs/plan.md",
            "Use public docs from github.com/openai/codex and do not rely on leaked source.",
        )]);

        assert!(audit.passed);
        assert_eq!(audit.policy_version, SOURCE_PROVENANCE_POLICY_VERSION);
        assert_eq!(audit.scan_source, "provided");
        assert!(audit
            .allowed_reference_domains
            .contains(&"github.com/openai/codex"));
        assert_eq!(audit.scanned_files, 1);
        assert_eq!(audit.skipped_files, 0);
        assert!(audit.findings.is_empty());
    }

    #[test]
    fn source_provenance_audit_flags_forbidden_copy_claims() {
        let copied_from_leaked = provenance_marker(&["copied", " from ", "leaked"]);
        let claim = format!("This implementation was {copied_from_leaked} source.");
        let audit = audit_source_provenance_texts([("src/example.rs", claim.as_str())]);

        assert!(!audit.passed);
        assert_eq!(audit.findings.len(), 1);
        assert_eq!(audit.findings[0].path, "src/example.rs");
        assert_eq!(audit.findings[0].marker, copied_from_leaked);
    }

    #[test]
    fn source_provenance_audit_fails_closed_when_no_files_scanned() {
        let audit = audit_source_provenance_texts([]);

        assert!(!audit.passed);
        assert_eq!(audit.scan_source, "provided");
        assert_eq!(audit.scanned_files, 0);
        assert_eq!(audit.findings.len(), 1);
        assert_eq!(audit.findings[0].marker, "no source files scanned");
    }

    #[test]
    fn source_provenance_repository_audit_scans_source_files() {
        let copied_codex_implementation =
            provenance_marker(&["copied", " codex ", "implementation"]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src").join("lib.rs"),
            format!("pub fn ok() {{}}\n// {copied_codex_implementation}\n"),
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("target")).unwrap();
        std::fs::write(dir.path().join("target").join("ignored.rs"), "").unwrap();

        let audit = audit_source_provenance_repository(dir.path());

        assert!(!audit.passed);
        assert_eq!(audit.scan_source, "filesystem");
        assert_eq!(audit.scanned_files, 1);
        assert_eq!(audit.findings.len(), 1);
        assert_eq!(audit.findings[0].path, "src/lib.rs");
        assert_eq!(audit.findings[0].marker, copied_codex_implementation);
    }

    #[test]
    fn source_provenance_repository_audit_scans_untracked_source_files_when_git_available() {
        let copied_codex_implementation =
            provenance_marker(&["copied", " codex ", "implementation"]);
        let dir = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("init")
            .output()
            .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src").join("lib.rs"), "pub fn ok() {}\n").unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("add")
            .arg("src/lib.rs")
            .output()
            .unwrap();
        std::fs::write(
            dir.path().join("src").join("untracked.rs"),
            format!("// {copied_codex_implementation}\n"),
        )
        .unwrap();

        let audit = audit_source_provenance_repository(dir.path());

        assert!(!audit.passed);
        assert_eq!(audit.scan_source, "git");
        assert!(audit.scanned_files >= 2);
        assert!(audit.findings.iter().any(|finding| {
            finding.path == "src/untracked.rs" && finding.marker == copied_codex_implementation
        }));
    }

    #[test]
    fn source_provenance_repository_audit_fails_closed_for_unreadable_source_candidates() {
        let dir = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("init")
            .output()
            .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src").join("missing.rs"),
            "pub fn missing() {}\n",
        )
        .unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("add")
            .arg("src/missing.rs")
            .output()
            .unwrap();
        std::fs::remove_file(dir.path().join("src").join("missing.rs")).unwrap();

        let audit = audit_source_provenance_repository(dir.path());

        assert!(!audit.passed);
        assert_eq!(audit.scan_source, "git");
        assert_eq!(audit.skipped_files, 1);
        assert!(audit.findings.iter().any(|finding| {
            finding.path == "src/missing.rs" && finding.marker == "source file unreadable"
        }));
    }

    #[test]
    #[cfg(unix)]
    fn source_provenance_repository_audit_fails_closed_for_source_symlink_escape() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("init")
            .output()
            .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(outside.path().join("external.rs"), "pub fn external() {}\n").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("external.rs"),
            dir.path().join("src").join("external.rs"),
        )
        .unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("add")
            .arg("src/external.rs")
            .output()
            .unwrap();

        let audit = audit_source_provenance_repository(dir.path());

        assert!(!audit.passed);
        assert_eq!(audit.scan_source, "git");
        assert_eq!(audit.skipped_files, 1);
        assert!(audit.findings.iter().any(|finding| {
            finding.path == "src/external.rs" && finding.marker == "source path escapes repository"
        }));
    }

    #[test]
    #[cfg(unix)]
    fn source_provenance_filesystem_scan_does_not_descend_symlinked_directories() {
        let copied_codex_implementation =
            provenance_marker(&["copied", " codex ", "implementation"]);
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src").join("lib.rs"), "pub fn ok() {}\n").unwrap();
        std::fs::write(
            outside.path().join("external.rs"),
            format!("// {copied_codex_implementation}\n"),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("linked-src")).unwrap();

        let audit = audit_source_provenance_repository(dir.path());

        assert!(audit.passed);
        assert_eq!(audit.scan_source, "filesystem");
        assert_eq!(audit.scanned_files, 1);
        assert!(audit.findings.is_empty());
    }

    #[test]
    fn source_provenance_candidate_includes_release_policy_source() {
        assert!(source_provenance_candidate(Path::new("src/release.rs")));
    }

    #[test]
    fn source_provenance_policy_source_has_no_literal_forbidden_markers() {
        let source = include_str!("release.rs").to_ascii_lowercase();
        for marker in forbidden_source_provenance_markers() {
            assert!(
                !source.contains(&marker),
                "release policy source contains forbidden marker literal: {marker}"
            );
        }
    }
}
