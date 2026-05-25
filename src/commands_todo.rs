//! Todo list command handler: /todo and its subcommands.

use crate::format::*;

use std::sync::RwLock;

/// Acquire a read-guard, recovering from a poisoned RwLock instead of panicking.
fn rw_read_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

/// Acquire a write-guard, recovering from a poisoned RwLock instead of panicking.
fn rw_write_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, Clone, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "[ ]"),
            TodoStatus::InProgress => write!(f, "[~]"),
            TodoStatus::Done => write!(f, "[✓]"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub id: usize,
    pub description: String,
    pub status: TodoStatus,
}

static TODO_LIST: RwLock<Vec<TodoItem>> = RwLock::new(Vec::new());
static TODO_NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

/// Add a todo item, return its ID.
pub fn todo_add(description: &str) -> usize {
    let id = TODO_NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let item = TodoItem {
        id,
        description: description.to_string(),
        status: TodoStatus::Pending,
    };
    rw_write_or_recover(&TODO_LIST).push(item);
    id
}

/// Update the status of a todo item by ID.
pub fn todo_update(id: usize, status: TodoStatus) -> Result<(), String> {
    let mut list = rw_write_or_recover(&TODO_LIST);
    match list.iter_mut().find(|item| item.id == id) {
        Some(item) => {
            item.status = status;
            Ok(())
        }
        None => Err(format!("No todo item with ID {id}")),
    }
}

/// Return a snapshot of all todo items.
pub fn todo_list() -> Vec<TodoItem> {
    rw_read_or_recover(&TODO_LIST).clone()
}

/// Clear all todo items and reset the ID counter.
pub fn todo_clear() {
    rw_write_or_recover(&TODO_LIST).clear();
    TODO_NEXT_ID.store(1, std::sync::atomic::Ordering::SeqCst);
}

/// Remove a single todo item by ID.
pub fn todo_remove(id: usize) -> Result<TodoItem, String> {
    let mut list = rw_write_or_recover(&TODO_LIST);
    let pos = list
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| format!("No todo item with ID {id}"))?;
    Ok(list.remove(pos))
}

/// Format the todo list with status checkboxes.
pub fn format_todo_list(items: &[TodoItem]) -> String {
    if items.is_empty() {
        return "  No tasks. Use /todo add <description> to add one.".to_string();
    }
    let mut out = String::new();
    for item in items {
        out.push_str(&format!(
            "  {} #{} {}\n",
            item.status, item.id, item.description
        ));
    }
    // Remove trailing newline
    if out.ends_with('\n') {
        out.truncate(out.len() - 1);
    }
    out
}

/// Handle the /todo command and its subcommands. Returns a string to print.
pub fn handle_todo(input: &str) -> String {
    let arg = input.strip_prefix("/todo").unwrap_or("").trim();

    // Route "board" subcommands to the board handler
    if arg == "board" || arg.starts_with("board ") {
        return handle_todo_board(arg.strip_prefix("board").unwrap_or("").trim());
    }

    if arg.is_empty() {
        // Show all tasks
        let items = todo_list();
        return format_todo_list(&items);
    }

    if arg == "clear" {
        todo_clear();
        return format!("{GREEN}  ✓ Cleared all tasks{RESET}");
    }

    if let Some(desc) = arg.strip_prefix("add ") {
        let desc = desc.trim();
        if desc.is_empty() {
            return "  Usage: /todo add <description>".to_string();
        }
        let id = todo_add(desc);
        return format!("{GREEN}  ✓ Added task #{id}: {desc}{RESET}");
    }
    if arg == "add" {
        return "  Usage: /todo add <description>".to_string();
    }

    if let Some(id_str) = arg.strip_prefix("done ") {
        let id_str = id_str.trim();
        match id_str.parse::<usize>() {
            Ok(id) => match todo_update(id, TodoStatus::Done) {
                Ok(()) => return format!("{GREEN}  ✓ Marked #{id} as done{RESET}"),
                Err(e) => return format!("{RED}  {e}{RESET}"),
            },
            Err(_) => return format!("{RED}  Invalid ID: {id_str}{RESET}"),
        }
    }

    if let Some(id_str) = arg.strip_prefix("wip ") {
        let id_str = id_str.trim();
        match id_str.parse::<usize>() {
            Ok(id) => match todo_update(id, TodoStatus::InProgress) {
                Ok(()) => return format!("{GREEN}  ✓ Marked #{id} as in-progress{RESET}"),
                Err(e) => return format!("{RED}  {e}{RESET}"),
            },
            Err(_) => return format!("{RED}  Invalid ID: {id_str}{RESET}"),
        }
    }

    if let Some(id_str) = arg.strip_prefix("remove ") {
        let id_str = id_str.trim();
        match id_str.parse::<usize>() {
            Ok(id) => match todo_remove(id) {
                Ok(item) => {
                    return format!("{GREEN}  ✓ Removed #{id}: {}{RESET}", item.description)
                }
                Err(e) => return format!("{RED}  {e}{RESET}"),
            },
            Err(_) => return format!("{RED}  Invalid ID: {id_str}{RESET}"),
        }
    }

    // Unknown subcommand — show usage
    "  Usage:\n\
     \x20 /todo                    Show all tasks\n\
     \x20 /todo add <description>  Add a new task\n\
     \x20 /todo done <id>          Mark task as done\n\
     \x20 /todo wip <id>           Mark as in-progress\n\
     \x20 /todo remove <id>        Remove a task\n\
     \x20 /todo clear              Clear all tasks"
        .to_string()
}

// === Board (TODO.md Kanban) ===

const TODO_MD_PATH: &str = "TODO.md";

/// Generate initial TODO.md content with Kanban sections.
pub fn board_template(goal: &str) -> String {
    format!(
        "# TODO\n\n\
         ## Current Goal\n\n\
         {goal}\n\n\
         ## Constraints\n\n\
         - \n\n\
         ## Backlog\n\n\
         ## Ready\n\n\
         ## In Progress\n\n\
         ## Blocked\n\n\
         ## Review\n\n\
         ## Done\n\n\
         ## Evidence Log\n"
    )
}

/// Extract items (lines starting with `- [ ]` or `- [x]`) from a named section.
pub fn parse_board_section(content: &str, section: &str) -> Vec<String> {
    let header = format!("## {section}");
    let mut in_section = false;
    let mut items = Vec::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            in_section = line.trim() == header;
            continue;
        }
        if in_section {
            let trimmed = line.trim();
            if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") {
                items.push(trimmed.to_string());
            }
        }
    }
    items
}

/// Check if a task (by description text) already exists in a section.
pub fn board_has_task(content: &str, section: &str, task: &str) -> bool {
    let items = parse_board_section(content, section);
    items
        .iter()
        .any(|item| item.ends_with(task) || item.contains(task))
}

/// Check if a task exists in ANY section of the board.
fn board_task_exists_anywhere(content: &str, task: &str) -> bool {
    let sections = [
        "Backlog",
        "Ready",
        "In Progress",
        "Blocked",
        "Review",
        "Done",
    ];
    sections.iter().any(|s| board_has_task(content, s, task))
}

/// Add a `- [ ] task` line to the end of the named section. Deduplicates across all sections.
pub fn board_add_task(content: &str, section: &str, task: &str) -> String {
    // Dedup: if task already exists in ANY section, return unchanged
    if board_task_exists_anywhere(content, task) {
        return content.to_string();
    }

    let header = format!("## {section}");
    let mut result = String::new();
    let mut in_target = false;
    let mut inserted = false;
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## ") {
            if in_target && !inserted {
                // Insert task before the next section header
                result.push_str(&format!("- [ ] {task}\n"));
                inserted = true;
            }
            in_target = line.trim() == header;
        }
        result.push_str(line);
        result.push('\n');

        // If this is the last line and we're in the target section
        if i == lines.len() - 1 && in_target && !inserted {
            result.push_str(&format!("- [ ] {task}\n"));
            inserted = true;
        }
    }

    // Handle edge case: section exists but is at the very end with no trailing newline
    if !inserted && in_target {
        result.push_str(&format!("- [ ] {task}\n"));
    }

    result
}

/// Move a task from one section to another. If moving to "Done", marks it `[x]`.
pub fn board_move_task(content: &str, from_section: &str, to_section: &str, task: &str) -> String {
    let from_header = format!("## {from_section}");
    let to_header = format!("## {to_section}");

    // First, remove from source section
    let mut without_task = String::new();
    let mut in_from = false;
    let mut found = false;

    for line in content.lines() {
        if line.starts_with("## ") {
            in_from = line.trim() == from_header;
        }
        if in_from && !found {
            let trimmed = line.trim();
            if (trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] "))
                && trimmed.contains(task)
            {
                found = true;
                continue; // Skip this line (remove from source)
            }
        }
        without_task.push_str(line);
        without_task.push('\n');
    }

    if !found {
        return content.to_string();
    }

    // Now add to destination section
    let mark = if to_section == "Done" {
        "- [x]"
    } else {
        "- [ ]"
    };
    let new_line = format!("{mark} {task}");

    let mut result = String::new();
    let mut in_to = false;
    let mut inserted = false;
    let lines: Vec<&str> = without_task.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## ") {
            if in_to && !inserted {
                result.push_str(&new_line);
                result.push('\n');
                inserted = true;
            }
            in_to = line.trim() == to_header;
        }
        result.push_str(line);
        result.push('\n');

        if i == lines.len() - 1 && in_to && !inserted {
            result.push_str(&new_line);
            result.push('\n');
            inserted = true;
        }
    }

    if !inserted && in_to {
        result.push_str(&new_line);
        result.push('\n');
    }

    result
}

/// Replace the content under `## Current Goal` with the new goal text.
pub fn board_set_goal(content: &str, goal: &str) -> String {
    let mut result = String::new();
    let mut in_goal = false;
    let mut replaced = false;

    for line in content.lines() {
        if line.starts_with("## ") {
            if in_goal && !replaced {
                result.push_str(goal);
                result.push_str("\n\n");
                replaced = true;
            }
            in_goal = line.trim() == "## Current Goal";
            result.push_str(line);
            result.push('\n');
            if in_goal {
                result.push('\n');
            }
            continue;
        }
        if in_goal {
            // Skip old content — we'll insert the new goal when we leave the section
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    // If goal section is the last section
    if in_goal && !replaced {
        result.push_str(goal);
        result.push('\n');
    }

    result
}

/// Append an evidence line to the Evidence Log section.
pub fn board_add_evidence(content: &str, evidence: &str) -> String {
    let header = "## Evidence Log";
    let mut result = String::new();
    let mut in_evidence = false;
    let mut inserted = false;
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## ") {
            if in_evidence && !inserted {
                result.push_str(&format!("- {evidence}\n"));
                inserted = true;
            }
            in_evidence = line.trim() == header;
        }
        result.push_str(line);
        result.push('\n');

        if i == lines.len() - 1 && in_evidence && !inserted {
            result.push_str(&format!("- {evidence}\n"));
            inserted = true;
        }
    }

    if !inserted && in_evidence {
        result.push_str(&format!("- {evidence}\n"));
    }

    result
}

/// Normalize section name from user input to canonical form.
fn normalize_section(input: &str) -> Option<&'static str> {
    match input.to_lowercase().as_str() {
        "backlog" => Some("Backlog"),
        "ready" => Some("Ready"),
        "inprogress" | "in-progress" | "in_progress" | "wip" => Some("In Progress"),
        "blocked" => Some("Blocked"),
        "review" => Some("Review"),
        "done" => Some("Done"),
        _ => None,
    }
}

/// Handle `/todo board` subcommands.
pub fn handle_todo_board(input: &str) -> String {
    let arg = input.trim();

    // /todo board or /todo board show
    if arg.is_empty() || arg == "show" {
        match std::fs::read_to_string(TODO_MD_PATH) {
            Ok(content) => return content,
            Err(_) => {
                return format!(
                    "{YELLOW}  TODO.md does not exist. Use /todo board init to create it.{RESET}"
                )
            }
        }
    }

    // /todo board init [goal]
    if arg == "init" || arg.starts_with("init ") {
        if std::path::Path::new(TODO_MD_PATH).exists() {
            return format!("{YELLOW}  TODO.md already exists. Not overwriting.{RESET}");
        }
        let goal = arg.strip_prefix("init").unwrap_or("").trim();
        let goal = if goal.is_empty() {
            "Planning board initialized."
        } else {
            goal
        };
        let content = board_template(goal);
        match std::fs::write(TODO_MD_PATH, &content) {
            Ok(()) => return format!("{GREEN}  ✓ Created TODO.md with goal: {goal}{RESET}"),
            Err(e) => return format!("{RED}  Failed to write TODO.md: {e}{RESET}"),
        }
    }

    // /todo board goal <text>
    if let Some(goal_text) = arg.strip_prefix("goal ") {
        let goal_text = goal_text.trim();
        if goal_text.is_empty() {
            return "  Usage: /todo board goal <text>".to_string();
        }
        match std::fs::read_to_string(TODO_MD_PATH) {
            Ok(content) => {
                let updated = board_set_goal(&content, goal_text);
                match std::fs::write(TODO_MD_PATH, &updated) {
                    Ok(()) => return format!("{GREEN}  ✓ Updated goal: {goal_text}{RESET}"),
                    Err(e) => return format!("{RED}  Failed to write TODO.md: {e}{RESET}"),
                }
            }
            Err(_) => {
                return format!(
                    "{YELLOW}  TODO.md does not exist. Use /todo board init first.{RESET}"
                )
            }
        }
    }
    if arg == "goal" {
        return "  Usage: /todo board goal <text>".to_string();
    }

    // /todo board evidence <text>
    if let Some(evidence_text) = arg.strip_prefix("evidence ") {
        let evidence_text = evidence_text.trim();
        if evidence_text.is_empty() {
            return "  Usage: /todo board evidence <text>".to_string();
        }
        match std::fs::read_to_string(TODO_MD_PATH) {
            Ok(content) => {
                let updated = board_add_evidence(&content, evidence_text);
                match std::fs::write(TODO_MD_PATH, &updated) {
                    Ok(()) => return format!("{GREEN}  ✓ Added evidence: {evidence_text}{RESET}"),
                    Err(e) => return format!("{RED}  Failed to write TODO.md: {e}{RESET}"),
                }
            }
            Err(_) => {
                return format!(
                    "{YELLOW}  TODO.md does not exist. Use /todo board init first.{RESET}"
                )
            }
        }
    }
    if arg == "evidence" {
        return "  Usage: /todo board evidence <text>".to_string();
    }

    // /todo board add <section> <task>
    if let Some(rest) = arg.strip_prefix("add ") {
        let rest = rest.trim();
        // First token is section, rest is task
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() < 2 || parts[1].trim().is_empty() {
            return "  Usage: /todo board add <section> <task>\n  \
                    Sections: backlog, ready, inprogress, blocked, review"
                .to_string();
        }
        let section_input = parts[0];
        let task = parts[1].trim();
        let section = match normalize_section(section_input) {
            Some(s) => s,
            None => {
                return format!(
                    "{RED}  Unknown section: {section_input}. Use: backlog, ready, inprogress, blocked, review{RESET}"
                )
            }
        };
        match std::fs::read_to_string(TODO_MD_PATH) {
            Ok(content) => {
                let updated = board_add_task(&content, section, task);
                if updated == content {
                    return format!("{YELLOW}  Task already exists: {task}{RESET}");
                }
                match std::fs::write(TODO_MD_PATH, &updated) {
                    Ok(()) => return format!("{GREEN}  ✓ Added to {section}: {task}{RESET}"),
                    Err(e) => return format!("{RED}  Failed to write TODO.md: {e}{RESET}"),
                }
            }
            Err(_) => {
                return format!(
                    "{YELLOW}  TODO.md does not exist. Use /todo board init first.{RESET}"
                )
            }
        }
    }

    // /todo board done <task_text> — shortcut to move to Done
    if let Some(task_text) = arg.strip_prefix("done ") {
        let task_text = task_text.trim();
        if task_text.is_empty() {
            return "  Usage: /todo board done <task_text>".to_string();
        }
        match std::fs::read_to_string(TODO_MD_PATH) {
            Ok(content) => {
                // Try to find the task in any section and move it to Done
                let sections = ["Backlog", "Ready", "In Progress", "Blocked", "Review"];
                for section in &sections {
                    if board_has_task(&content, section, task_text) {
                        let updated = board_move_task(&content, section, "Done", task_text);
                        match std::fs::write(TODO_MD_PATH, &updated) {
                            Ok(()) => {
                                return format!("{GREEN}  ✓ Moved to Done: {task_text}{RESET}")
                            }
                            Err(e) => return format!("{RED}  Failed to write TODO.md: {e}{RESET}"),
                        }
                    }
                }
                format!("{YELLOW}  Task not found: {task_text}{RESET}")
            }
            Err(_) => {
                format!("{YELLOW}  TODO.md does not exist. Use /todo board init first.{RESET}")
            }
        }
    }
    // /todo board move <task_text> <to_section>
    else if let Some(rest) = arg.strip_prefix("move ") {
        let rest = rest.trim();
        // Last token is destination section, everything before is task text
        let parts: Vec<&str> = rest.rsplitn(2, ' ').collect();
        if parts.len() < 2 || parts[1].trim().is_empty() {
            return "  Usage: /todo board move <task_text> <to_section>".to_string();
        }
        let to_section_input = parts[0];
        let task_text = parts[1].trim();
        let to_section = match normalize_section(to_section_input) {
            Some(s) => s,
            None => {
                return format!(
                    "{RED}  Unknown section: {to_section_input}. Use: backlog, ready, inprogress, blocked, review, done{RESET}"
                )
            }
        };
        match std::fs::read_to_string(TODO_MD_PATH) {
            Ok(content) => {
                // Find the task in any section
                let sections = [
                    "Backlog",
                    "Ready",
                    "In Progress",
                    "Blocked",
                    "Review",
                    "Done",
                ];
                for section in &sections {
                    if board_has_task(&content, section, task_text) {
                        let updated = board_move_task(&content, section, to_section, task_text);
                        match std::fs::write(TODO_MD_PATH, &updated) {
                            Ok(()) => {
                                return format!(
                                    "{GREEN}  ✓ Moved '{task_text}' to {to_section}{RESET}"
                                )
                            }
                            Err(e) => return format!("{RED}  Failed to write TODO.md: {e}{RESET}"),
                        }
                    }
                }
                format!("{YELLOW}  Task not found: {task_text}{RESET}")
            }
            Err(_) => {
                format!("{YELLOW}  TODO.md does not exist. Use /todo board init first.{RESET}")
            }
        }
    } else {
        // Unknown board subcommand
        "  Usage:\n\
         \x20 /todo board              Show TODO.md board\n\
         \x20 /todo board init [goal]  Create TODO.md\n\
         \x20 /todo board add <section> <task>  Add task\n\
         \x20 /todo board move <task> <section> Move task\n\
         \x20 /todo board done <task>  Mark task done\n\
         \x20 /todo board goal <text>  Set current goal\n\
         \x20 /todo board evidence <text>  Add evidence"
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_todo_add_returns_incrementing_ids() {
        todo_clear();
        let id1 = todo_add("first task");
        let id2 = todo_add("second task");
        assert!(id2 > id1, "IDs should increment: {id1} < {id2}");
        let items = todo_list();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].description, "first task");
        assert_eq!(items[1].description, "second task");
    }

    #[test]
    #[serial]
    fn test_todo_update_status() {
        todo_clear();
        let id = todo_add("update me");
        assert_eq!(todo_list()[0].status, TodoStatus::Pending);

        todo_update(id, TodoStatus::InProgress).unwrap();
        assert_eq!(todo_list()[0].status, TodoStatus::InProgress);

        todo_update(id, TodoStatus::Done).unwrap();
        assert_eq!(todo_list()[0].status, TodoStatus::Done);
    }

    #[test]
    #[serial]
    fn test_todo_update_invalid_id() {
        todo_clear();
        let result = todo_update(99999, TodoStatus::Done);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("99999"));
    }

    #[test]
    #[serial]
    fn test_todo_remove() {
        todo_clear();
        let id = todo_add("remove me");
        assert_eq!(todo_list().len(), 1);

        let removed = todo_remove(id).unwrap();
        assert_eq!(removed.description, "remove me");
        assert!(todo_list().is_empty());
    }

    #[test]
    #[serial]
    fn test_todo_remove_invalid_id() {
        todo_clear();
        let result = todo_remove(99998);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("99998"));
    }

    #[test]
    #[serial]
    fn test_todo_clear() {
        todo_clear();
        todo_add("one");
        todo_add("two");
        assert_eq!(todo_list().len(), 2);

        todo_clear();
        assert!(todo_list().is_empty());
    }

    #[test]
    #[serial]
    fn test_todo_list_empty() {
        todo_clear();
        assert!(todo_list().is_empty());
    }

    #[test]
    #[serial]
    fn test_format_todo_list() {
        todo_clear();
        let id1 = todo_add("pending task");
        let id2 = todo_add("wip task");
        let id3 = todo_add("done task");
        todo_update(id2, TodoStatus::InProgress).unwrap();
        todo_update(id3, TodoStatus::Done).unwrap();

        let items = todo_list();
        let formatted = format_todo_list(&items);
        assert!(formatted.contains("[ ]"), "Should contain pending checkbox");
        assert!(
            formatted.contains("[~]"),
            "Should contain in-progress checkbox"
        );
        assert!(formatted.contains("[✓]"), "Should contain done checkbox");
        assert!(formatted.contains(&format!("#{id1}")));
        assert!(formatted.contains("pending task"));
        assert!(formatted.contains("wip task"));
        assert!(formatted.contains("done task"));
    }

    #[test]
    fn test_format_todo_list_empty() {
        let formatted = format_todo_list(&[]);
        assert!(formatted.contains("No tasks"));
    }

    #[test]
    #[serial]
    fn test_handle_todo_add() {
        todo_clear();
        let result = handle_todo("/todo add write tests");
        assert!(result.contains("Added task"));
        assert!(result.contains("write tests"));
        assert_eq!(todo_list().len(), 1);
    }

    #[test]
    #[serial]
    fn test_handle_todo_show_empty() {
        todo_clear();
        let result = handle_todo("/todo");
        assert!(result.contains("No tasks"));
    }

    #[test]
    #[serial]
    fn test_handle_todo_done() {
        todo_clear();
        let id = todo_add("finish me");
        let result = handle_todo(&format!("/todo done {id}"));
        assert!(result.contains("done"));
        assert_eq!(todo_list()[0].status, TodoStatus::Done);
    }

    #[test]
    #[serial]
    fn test_handle_todo_wip() {
        todo_clear();
        let id = todo_add("start me");
        let result = handle_todo(&format!("/todo wip {id}"));
        assert!(result.contains("in-progress"));
        assert_eq!(todo_list()[0].status, TodoStatus::InProgress);
    }

    #[test]
    #[serial]
    fn test_handle_todo_remove_via_command() {
        todo_clear();
        let id = todo_add("delete me");
        let result = handle_todo(&format!("/todo remove {id}"));
        assert!(result.contains("Removed"));
        assert!(todo_list().is_empty());
    }

    #[test]
    #[serial]
    fn test_handle_todo_clear_via_command() {
        todo_clear();
        todo_add("one");
        todo_add("two");
        let result = handle_todo("/todo clear");
        assert!(result.contains("Cleared"));
        assert!(todo_list().is_empty());
    }

    #[test]
    fn test_handle_todo_unknown_subcommand() {
        let result = handle_todo("/todo badcmd");
        assert!(result.contains("Usage"));
    }

    #[test]
    #[serial]
    fn test_handle_todo_add_empty_description() {
        let result = handle_todo("/todo add");
        assert!(result.contains("Usage"));
        let result2 = handle_todo("/todo add   ");
        assert!(result2.contains("Usage"));
    }

    #[test]
    fn test_todo_in_known_commands() {
        assert!(
            crate::commands::KNOWN_COMMANDS.contains(&"/todo"),
            "/todo should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_todo_help_exists() {
        let help = crate::help::command_help("todo");
        assert!(help.is_some(), "todo should have help text");
        let text = help.unwrap();
        assert!(text.contains("/todo add"));
        assert!(text.contains("/todo done"));
        assert!(text.contains("/todo clear"));
    }

    #[test]
    fn test_todo_in_help_text() {
        let text = crate::help::help_text();
        assert!(text.contains("/todo"), "/todo should appear in help text");
    }

    // === Board tests ===

    #[test]
    fn test_board_template_has_all_sections() {
        let content = board_template("Build something great");
        assert!(content.contains("# TODO"));
        assert!(content.contains("## Current Goal"));
        assert!(content.contains("Build something great"));
        assert!(content.contains("## Constraints"));
        assert!(content.contains("## Backlog"));
        assert!(content.contains("## Ready"));
        assert!(content.contains("## In Progress"));
        assert!(content.contains("## Blocked"));
        assert!(content.contains("## Review"));
        assert!(content.contains("## Done"));
        assert!(content.contains("## Evidence Log"));
    }

    #[test]
    fn test_parse_board_section_extracts_items() {
        let content =
            "## Backlog\n\n- [ ] task one\n- [ ] task two\n\n## Ready\n\n- [ ] task three\n";
        let items = parse_board_section(content, "Backlog");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "- [ ] task one");
        assert_eq!(items[1], "- [ ] task two");

        let ready = parse_board_section(content, "Ready");
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "- [ ] task three");
    }

    #[test]
    fn test_parse_board_section_handles_done_checkboxes() {
        let content = "## Done\n\n- [x] finished task\n- [ ] not done yet\n";
        let items = parse_board_section(content, "Done");
        assert_eq!(items.len(), 2);
        assert!(items[0].contains("[x]"));
        assert!(items[1].contains("[ ]"));
    }

    #[test]
    fn test_board_has_task_finds_existing() {
        let content = "## Backlog\n\n- [ ] implement feature X\n\n## Ready\n";
        assert!(board_has_task(content, "Backlog", "implement feature X"));
        assert!(!board_has_task(content, "Backlog", "nonexistent task"));
        assert!(!board_has_task(content, "Ready", "implement feature X"));
    }

    #[test]
    fn test_board_add_task_adds_to_section() {
        let content = board_template("test");
        let updated = board_add_task(&content, "Backlog", "new task");
        assert!(updated.contains("- [ ] new task"));
        let items = parse_board_section(&updated, "Backlog");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "- [ ] new task");
    }

    #[test]
    fn test_board_add_task_deduplicates() {
        let content = board_template("test");
        let updated = board_add_task(&content, "Backlog", "task A");
        let updated2 = board_add_task(&updated, "Backlog", "task A");
        // Should not add duplicate
        assert_eq!(updated, updated2);

        // Also deduplicates across sections
        let updated3 = board_add_task(&updated, "Ready", "task A");
        assert_eq!(updated, updated3);
    }

    #[test]
    fn test_board_move_task_between_sections() {
        let content = board_template("test");
        let with_task = board_add_task(&content, "Backlog", "move me");
        assert!(board_has_task(&with_task, "Backlog", "move me"));

        let moved = board_move_task(&with_task, "Backlog", "Ready", "move me");
        assert!(!board_has_task(&moved, "Backlog", "move me"));
        assert!(board_has_task(&moved, "Ready", "move me"));

        // Item should still have [ ] checkbox
        let ready_items = parse_board_section(&moved, "Ready");
        assert_eq!(ready_items.len(), 1);
        assert!(ready_items[0].starts_with("- [ ]"));
    }

    #[test]
    fn test_board_move_task_to_done_marks_checked() {
        let content = board_template("test");
        let with_task = board_add_task(&content, "In Progress", "finish me");
        let moved = board_move_task(&with_task, "In Progress", "Done", "finish me");

        assert!(!board_has_task(&moved, "In Progress", "finish me"));
        let done_items = parse_board_section(&moved, "Done");
        assert_eq!(done_items.len(), 1);
        assert!(done_items[0].starts_with("- [x]"));
        assert!(done_items[0].contains("finish me"));
    }

    #[test]
    fn test_board_set_goal_replaces_content() {
        let content = board_template("old goal");
        assert!(content.contains("old goal"));

        let updated = board_set_goal(&content, "new goal");
        assert!(updated.contains("new goal"));
        assert!(!updated.contains("old goal"));
        // Sections should still exist
        assert!(updated.contains("## Backlog"));
    }

    #[test]
    fn test_board_add_evidence() {
        let content = board_template("test");
        let updated = board_add_evidence(&content, "Tests pass at 95% coverage");
        assert!(updated.contains("- Tests pass at 95% coverage"));

        // Add another
        let updated2 = board_add_evidence(&updated, "Lint clean");
        assert!(updated2.contains("- Tests pass at 95% coverage"));
        assert!(updated2.contains("- Lint clean"));
    }

    #[test]
    #[serial]
    fn test_handle_todo_board_show_no_file() {
        // Ensure TODO.md doesn't exist in test dir
        let _ = std::fs::remove_file(TODO_MD_PATH);
        let result = handle_todo_board("");
        assert!(result.contains("does not exist"));

        let result2 = handle_todo_board("show");
        assert!(result2.contains("does not exist"));
    }

    #[test]
    #[serial]
    fn test_handle_todo_board_init_creates_file() {
        let _ = std::fs::remove_file(TODO_MD_PATH);
        let result = handle_todo_board("init Build the thing");
        assert!(result.contains("Created TODO.md"));
        assert!(result.contains("Build the thing"));

        // File should exist now
        let content = std::fs::read_to_string(TODO_MD_PATH).unwrap();
        assert!(content.contains("## Current Goal"));
        assert!(content.contains("Build the thing"));

        // Init again should NOT overwrite
        let result2 = handle_todo_board("init Different goal");
        assert!(result2.contains("already exists"));

        // Content unchanged
        let content2 = std::fs::read_to_string(TODO_MD_PATH).unwrap();
        assert!(content2.contains("Build the thing"));
        assert!(!content2.contains("Different goal"));

        // Cleanup
        let _ = std::fs::remove_file(TODO_MD_PATH);
    }

    #[test]
    #[serial]
    fn test_handle_todo_routes_board() {
        // Ensure the routing from /todo board works
        let _ = std::fs::remove_file(TODO_MD_PATH);
        let result = handle_todo("/todo board");
        assert!(result.contains("does not exist"));
    }

    #[test]
    fn test_normalize_section() {
        assert_eq!(normalize_section("backlog"), Some("Backlog"));
        assert_eq!(normalize_section("ready"), Some("Ready"));
        assert_eq!(normalize_section("inprogress"), Some("In Progress"));
        assert_eq!(normalize_section("in-progress"), Some("In Progress"));
        assert_eq!(normalize_section("wip"), Some("In Progress"));
        assert_eq!(normalize_section("blocked"), Some("Blocked"));
        assert_eq!(normalize_section("review"), Some("Review"));
        assert_eq!(normalize_section("done"), Some("Done"));
        assert_eq!(normalize_section("invalid"), None);
    }

    #[test]
    fn test_board_move_task_not_found() {
        let content = board_template("test");
        let result = board_move_task(&content, "Backlog", "Ready", "nonexistent");
        // Should return content unchanged
        assert_eq!(result, content);
    }
}
