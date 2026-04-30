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
}
