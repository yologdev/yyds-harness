//! Skill-related command handlers: /skill list, show, path, install.

use crate::format::*;

/// Subcommand names for `/skill <Tab>` completion.
pub const SKILL_SUBCOMMANDS: &[&str] = &["list", "show", "path", "install"];

/// Handle the `/skill` command: list, show, and inspect loaded skills.
///
/// Accepts the raw input (with or without the `/skill` prefix) and a reference
/// to the loaded `SkillSet`. If no skills directory is configured, prints a
/// helpful message about the `--skills` flag.
pub fn handle_skill(input: &str, skills: &yoagent::skills::SkillSet) {
    let sub = input.strip_prefix("/skill").unwrap_or(input).trim();

    if sub.is_empty() || sub == "list" {
        skill_list(skills);
    } else if sub == "path" {
        skill_path(skills);
    } else if let Some(name) = sub.strip_prefix("show ") {
        skill_show(name.trim(), skills);
    } else if sub == "show" {
        eprintln!("{YELLOW}  usage: /skill show <name>{RESET}");
        eprintln!("{DIM}  try /skill list to see available skills{RESET}\n");
    } else if let Some(source) = sub.strip_prefix("install ") {
        let source = source.trim();
        if source.is_empty() {
            eprintln!("{YELLOW}  usage: /skill install <path>{RESET}");
            eprintln!("{DIM}  install a skill from a local directory containing SKILL.md{RESET}\n");
        } else {
            skill_install(source);
        }
    } else if sub == "install" {
        eprintln!("{YELLOW}  usage: /skill install <path>{RESET}");
        eprintln!("{DIM}  install a skill from a local directory containing SKILL.md{RESET}\n");
    } else {
        eprintln!("{RED}  unknown subcommand: {sub}{RESET}");
        eprintln!("{DIM}  try: /skill list, /skill show <name>, /skill path, /skill install <path>{RESET}\n");
    }
}

/// List all loaded skills with name and description.
fn skill_list(skills: &yoagent::skills::SkillSet) {
    if skills.is_empty() {
        println!("{DIM}  no skills loaded{RESET}");
        println!("{DIM}  use --skills <dir> to load skills from a directory{RESET}\n");
        return;
    }

    println!("{BOLD}  Loaded skills ({}):{RESET}\n", skills.len());

    // Find the longest skill name for alignment
    let max_name_len = skills
        .skills()
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(0);

    for skill in skills.skills() {
        let padding = " ".repeat(max_name_len.saturating_sub(skill.name.len()));
        println!(
            "    {GREEN}{}{RESET}{}  {DIM}{}{RESET}",
            skill.name, padding, skill.description
        );
    }
    println!();
}

/// Show the current skills directory paths (derived from loaded skill base_dirs).
fn skill_path(skills: &yoagent::skills::SkillSet) {
    if skills.is_empty() {
        println!("{DIM}  no skills directory configured{RESET}");
        println!("{DIM}  use --skills <dir> to load skills from a directory{RESET}\n");
        return;
    }

    // Collect unique parent directories from loaded skills
    let mut dirs: Vec<String> = skills
        .skills()
        .iter()
        .filter_map(|s| s.base_dir.parent().map(|p| p.display().to_string()))
        .collect();
    dirs.sort();
    dirs.dedup();

    if dirs.len() == 1 {
        println!("{DIM}  skills directory: {}{RESET}\n", dirs[0]);
    } else {
        println!("{DIM}  skills directories:{RESET}");
        for d in &dirs {
            println!("{DIM}    {d}{RESET}");
        }
        println!();
    }
}

/// Show the full content of a named skill's SKILL.md file.
fn skill_show(name: &str, skills: &yoagent::skills::SkillSet) {
    let skill = skills.skills().iter().find(|s| s.name == name);

    match skill {
        Some(s) => {
            match std::fs::read_to_string(&s.file_path) {
                Ok(content) => {
                    println!("{BOLD}  Skill: {}{RESET}", s.name);
                    println!("{DIM}  path: {}{RESET}\n", s.file_path.display());
                    // Print the skill content with light indentation
                    for line in content.lines() {
                        println!("  {line}");
                    }
                    println!();
                }
                Err(e) => {
                    eprintln!(
                        "{RED}  error reading {}: {e}{RESET}\n",
                        s.file_path.display()
                    );
                }
            }
        }
        None => {
            eprintln!("{RED}  skill not found: {name}{RESET}");
            if !skills.is_empty() {
                let names: Vec<&str> = skills.skills().iter().map(|s| s.name.as_str()).collect();
                eprintln!("{DIM}  available: {}{RESET}\n", names.join(", "));
            } else {
                eprintln!("{DIM}  no skills loaded — use --skills <dir>{RESET}\n");
            }
        }
    }
}

/// Extract the skill name from SKILL.md YAML frontmatter.
///
/// Looks for a `name:` field in the YAML frontmatter delimited by `---`.
/// Returns `None` if there's no valid frontmatter or no name field.
fn extract_skill_name_from_frontmatter(content: &str) -> Option<String> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    // Find the closing ---
    let after_first = &content[3..];
    let end_idx = after_first.find("\n---")?;
    let frontmatter = &after_first[..end_idx];

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            let name = value.trim().trim_matches('"').trim_matches('\'').trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Default skill install directory: `~/.config/yoyo/skills/`
///
/// Uses `XDG_CONFIG_HOME` if set, otherwise falls back to `~/.config`.
fn default_skill_install_dir() -> Option<std::path::PathBuf> {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })?;
    Some(config_dir.join("yoyo").join("skills"))
}

/// Install a skill from a local directory path.
///
/// Validates the source, extracts the skill name from SKILL.md frontmatter,
/// and copies the entire directory to `~/.config/yoyo/skills/<name>/`.
fn skill_install(source: &str) {
    let source_path = std::path::Path::new(source);

    // Validate source exists and is a directory
    if !source_path.exists() {
        eprintln!("{RED}  error: path does not exist: {source}{RESET}\n");
        return;
    }
    if !source_path.is_dir() {
        eprintln!("{RED}  error: not a directory: {source}{RESET}");
        eprintln!("{DIM}  /skill install expects a skill directory containing SKILL.md{RESET}\n");
        return;
    }

    // Validate SKILL.md exists
    let skill_md_path = source_path.join("SKILL.md");
    if !skill_md_path.exists() {
        eprintln!("{RED}  error: no SKILL.md found in {source}{RESET}");
        eprintln!("{DIM}  a skill directory must contain a SKILL.md file{RESET}\n");
        return;
    }

    // Read and parse frontmatter for skill name
    let content = match std::fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{RED}  error reading SKILL.md: {e}{RESET}\n");
            return;
        }
    };

    let skill_name = match extract_skill_name_from_frontmatter(&content) {
        Some(name) => name,
        None => {
            eprintln!(
                "{RED}  error: could not extract skill name from SKILL.md frontmatter{RESET}"
            );
            eprintln!("{DIM}  SKILL.md must have YAML frontmatter with a 'name:' field{RESET}\n");
            return;
        }
    };

    // Determine install destination
    let install_dir = match default_skill_install_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("{RED}  error: could not determine config directory{RESET}\n");
            return;
        }
    };

    let dest = install_dir.join(&skill_name);

    // Check if skill already exists
    if dest.exists() {
        eprintln!(
            "{YELLOW}  skill '{skill_name}' already installed at {}{RESET}",
            dest.display()
        );
        eprintln!("{DIM}  remove the existing directory first to reinstall{RESET}\n");
        return;
    }

    // Create destination and copy
    if let Err(e) = std::fs::create_dir_all(&dest) {
        eprintln!(
            "{RED}  error creating directory {}: {e}{RESET}\n",
            dest.display()
        );
        return;
    }

    match copy_dir_recursive(source_path, &dest) {
        Ok(count) => {
            println!("{GREEN}  ✓ installed skill '{skill_name}' ({count} files){RESET}");
            println!("{DIM}  location: {}{RESET}", dest.display());
            println!(
                "{DIM}  load with: --skills {}{RESET}\n",
                install_dir.display()
            );
        }
        Err(e) => {
            eprintln!("{RED}  error copying skill: {e}{RESET}");
            // Clean up partial install
            let _ = std::fs::remove_dir_all(&dest);
            eprintln!("{DIM}  installation rolled back{RESET}\n");
        }
    }
}

/// Recursively copy a directory's contents. Returns the number of files copied.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            std::fs::create_dir_all(&dest_path)?;
            count += copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Install a skill to a specific directory (used for testing).
/// Same logic as `skill_install` but allows specifying the destination base dir.
#[cfg(test)]
fn skill_install_to(source: &str, install_dir: &std::path::Path) -> Result<String, String> {
    let source_path = std::path::Path::new(source);

    if !source_path.exists() {
        return Err(format!("path does not exist: {source}"));
    }
    if !source_path.is_dir() {
        return Err(format!("not a directory: {source}"));
    }

    let skill_md_path = source_path.join("SKILL.md");
    if !skill_md_path.exists() {
        return Err(format!("no SKILL.md found in {source}"));
    }

    let content = std::fs::read_to_string(&skill_md_path)
        .map_err(|e| format!("error reading SKILL.md: {e}"))?;

    let skill_name = extract_skill_name_from_frontmatter(&content)
        .ok_or_else(|| "could not extract skill name from SKILL.md frontmatter".to_string())?;

    let dest = install_dir.join(&skill_name);

    if dest.exists() {
        return Err(format!(
            "skill '{skill_name}' already installed at {}",
            dest.display()
        ));
    }

    std::fs::create_dir_all(&dest).map_err(|e| format!("error creating directory: {e}"))?;

    match copy_dir_recursive(source_path, &dest) {
        Ok(_count) => Ok(skill_name),
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dest);
            Err(format!("error copying skill: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    use crate::commands::KNOWN_COMMANDS;
    use crate::help::help_text;

    #[test]
    fn test_skill_in_known_commands() {
        assert!(
            KNOWN_COMMANDS.contains(&"/skill"),
            "/skill should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_skill_in_help_text() {
        let help = help_text();
        assert!(help.contains("/skill"), "/skill should appear in help text");
        assert!(help.contains("skills"), "Help text should mention skills");
    }

    #[test]
    fn test_skill_list_with_real_skills() {
        // Load the real ./skills directory used by this project
        let skills = yoagent::skills::SkillSet::load(&["./skills"]).unwrap();
        assert!(
            skills.len() >= 4,
            "Expected at least 4 core skills, got {}",
            skills.len()
        );

        // Verify the evolve skill is present
        let names: Vec<&str> = skills.skills().iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"evolve"), "evolve skill should be loaded");
        assert!(
            names.contains(&"communicate"),
            "communicate skill should be loaded"
        );
    }

    #[test]
    fn test_skill_list_empty() {
        let skills = yoagent::skills::SkillSet::empty();
        // Should not panic — just print "no skills loaded"
        handle_skill("/skill list", &skills);
        handle_skill("/skill", &skills);
    }

    #[test]
    fn test_skill_show_existing() {
        let skills = yoagent::skills::SkillSet::load(&["./skills"]).unwrap();
        // Should not panic — prints the evolve skill content
        handle_skill("/skill show evolve", &skills);
    }

    #[test]
    fn test_skill_show_nonexistent() {
        let skills = yoagent::skills::SkillSet::load(&["./skills"]).unwrap();
        // Should not panic — prints error message
        handle_skill("/skill show nonexistent-skill", &skills);
    }

    #[test]
    fn test_skill_path() {
        let skills = yoagent::skills::SkillSet::load(&["./skills"]).unwrap();
        // Should not panic — prints the skills directory
        handle_skill("/skill path", &skills);
    }

    #[test]
    fn test_skill_path_empty() {
        let skills = yoagent::skills::SkillSet::empty();
        // Should not panic — prints "no skills directory configured"
        handle_skill("/skill path", &skills);
    }

    #[test]
    fn test_skill_unknown_subcommand() {
        let skills = yoagent::skills::SkillSet::empty();
        // Should not panic — prints error about unknown subcommand
        handle_skill("/skill foobar", &skills);
    }

    #[test]
    fn test_skill_show_bare() {
        let skills = yoagent::skills::SkillSet::empty();
        // Should not panic — prints usage hint
        handle_skill("/skill show", &skills);
    }

    #[test]
    fn test_skill_with_temp_dir() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A test skill\n---\n\n# My Skill\n\nDoes things.\n",
        )
        .unwrap();

        let skills = yoagent::skills::SkillSet::load(&[tmp.path()]).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills.skills()[0].name, "my-skill");
        assert_eq!(skills.skills()[0].description, "A test skill");

        // List should work
        handle_skill("/skill list", &skills);

        // Show should work
        handle_skill("/skill show my-skill", &skills);

        // Path should work
        handle_skill("/skill path", &skills);
    }

    #[test]
    fn test_extract_skill_name_valid() {
        let content = "---\nname: my-cool-skill\ndescription: Does things\n---\n\n# Content\n";
        assert_eq!(
            extract_skill_name_from_frontmatter(content),
            Some("my-cool-skill".to_string())
        );
    }

    #[test]
    fn test_extract_skill_name_quoted() {
        let content = "---\nname: \"quoted-name\"\ndescription: test\n---\n\nBody\n";
        assert_eq!(
            extract_skill_name_from_frontmatter(content),
            Some("quoted-name".to_string())
        );
    }

    #[test]
    fn test_extract_skill_name_single_quoted() {
        let content = "---\nname: 'single-quoted'\ndescription: test\n---\n\nBody\n";
        assert_eq!(
            extract_skill_name_from_frontmatter(content),
            Some("single-quoted".to_string())
        );
    }

    #[test]
    fn test_extract_skill_name_no_frontmatter() {
        let content = "# Just a heading\n\nNo frontmatter here.\n";
        assert_eq!(extract_skill_name_from_frontmatter(content), None);
    }

    #[test]
    fn test_extract_skill_name_no_name_field() {
        let content = "---\ndescription: has frontmatter but no name\n---\n\nBody\n";
        assert_eq!(extract_skill_name_from_frontmatter(content), None);
    }

    #[test]
    fn test_extract_skill_name_empty_name() {
        let content = "---\nname: \ndescription: empty name\n---\n\nBody\n";
        assert_eq!(extract_skill_name_from_frontmatter(content), None);
    }

    #[test]
    fn test_skill_install_nonexistent_path() {
        let tmp = TempDir::new().unwrap();
        let install_dir = tmp.path().join("installed");
        let result = skill_install_to("/nonexistent/path/to/skill", &install_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_skill_install_missing_skill_md() {
        let tmp = TempDir::new().unwrap();
        // Create a directory without SKILL.md
        let source = tmp.path().join("bad-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "# Not a skill").unwrap();

        let install_dir = tmp.path().join("installed");
        let result = skill_install_to(source.to_str().unwrap(), &install_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no SKILL.md"));
    }

    #[test]
    fn test_skill_install_not_a_directory() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not-a-dir.txt");
        fs::write(&file_path, "just a file").unwrap();

        let install_dir = tmp.path().join("installed");
        let result = skill_install_to(file_path.to_str().unwrap(), &install_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_skill_install_bad_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("no-name-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Just markdown, no frontmatter\n").unwrap();

        let install_dir = tmp.path().join("installed");
        let result = skill_install_to(source.to_str().unwrap(), &install_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not extract skill name"));
    }

    #[test]
    fn test_skill_install_success() {
        let tmp = TempDir::new().unwrap();

        // Create source skill with multiple files
        let source = tmp.path().join("my-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: test-skill\ndescription: A test\n---\n\n# Test Skill\n",
        )
        .unwrap();
        fs::write(source.join("extra.txt"), "extra content").unwrap();

        let install_dir = tmp.path().join("installed");
        let result = skill_install_to(source.to_str().unwrap(), &install_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-skill");

        // Verify files were copied
        let dest = install_dir.join("test-skill");
        assert!(dest.join("SKILL.md").exists());
        assert!(dest.join("extra.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("extra.txt")).unwrap(),
            "extra content"
        );
    }

    #[test]
    fn test_skill_install_already_exists() {
        let tmp = TempDir::new().unwrap();

        // Create source skill
        let source = tmp.path().join("my-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: dupe-skill\ndescription: A test\n---\n\n# Test\n",
        )
        .unwrap();

        let install_dir = tmp.path().join("installed");

        // First install succeeds
        let result = skill_install_to(source.to_str().unwrap(), &install_dir);
        assert!(result.is_ok());

        // Second install fails (already exists)
        let result2 = skill_install_to(source.to_str().unwrap(), &install_dir);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already installed"));
    }

    #[test]
    fn test_skill_install_with_subdirectory() {
        let tmp = TempDir::new().unwrap();

        // Create source skill with a subdirectory
        let source = tmp.path().join("complex-skill");
        let sub = source.join("templates");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: complex-skill\ndescription: Has subdirs\n---\n\n# Complex\n",
        )
        .unwrap();
        fs::write(sub.join("template.txt"), "template content").unwrap();

        let install_dir = tmp.path().join("installed");
        let result = skill_install_to(source.to_str().unwrap(), &install_dir);
        assert!(result.is_ok());

        // Verify subdirectory was copied
        let dest = install_dir.join("complex-skill");
        assert!(dest.join("templates").join("template.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("templates").join("template.txt")).unwrap(),
            "template content"
        );
    }

    #[test]
    fn test_skill_install_subcommand_routing() {
        let skills = yoagent::skills::SkillSet::empty();
        // These should not panic — they print usage/error messages
        handle_skill("/skill install", &skills);
        handle_skill("/skill install ", &skills);
    }
}
