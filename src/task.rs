// TASK.md parsing/rendering and loading of the tasks/ folder.
//
// Format:
//
//     # <title>
//
//     - STATUS: (OPEN|CLOSED)
//     - PRIORITY: <number>
//     - TAGS: <comma-and-whitespace-separated-list-of-tags>
//     [- ASSIGNEES: <comma-and-whitespace-separated-list-of-people>]
//     [other properties]
//
//     [description]

use std::fs;
use std::path::Path;

use crate::error::{Context, Error, Result};
use crate::huid::is_valid_huid;
use crate::text::{is_space, is_space_except_newline, split_list, trim, trim_end};

pub const DEFAULT_TASK_TITLE: &str = "New Task";
pub const DEFAULT_PRIORITY: u64 = 100;
/// Unset priority is super high so you don't forget to set it.
pub const UNSET_PRIORITY: i64 = 999999;
pub const INVALID_TITLE: &str = "!!! INVALID: TASK TITLE MUST START WITH # !!!";

const STATUS: &str = "STATUS";
const PRIORITY: &str = "PRIORITY";
const TAGS: &str = "TAGS";
const ASSIGNEES: &str = "ASSIGNEES";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub key: String,
    pub value: String,
}

/// Ordered list of `- KEY: VALUE` properties. Putting an existing key moves it to
/// the end, so for duplicated properties the last one wins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Properties(pub Vec<Property>);

impl Properties {
    pub fn put(&mut self, key: &str, value: &str) {
        self.0.retain(|p| p.key != key);
        self.0.push(Property { key: key.to_string(), value: value.to_string() });
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.iter().find(|p| p.key == key).map(|p| p.value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMd {
    /// `None` when the file doesn't start with `#`.
    pub title: Option<String>,
    pub properties: Properties,
    pub body: String,
}

fn skip_while(b: &[u8], mut i: usize, pred: impl Fn(u8) -> bool) -> usize {
    while i < b.len() && pred(b[i]) {
        i += 1;
    }
    i
}

fn line_end(b: &[u8], i: usize) -> usize {
    b[i..].iter().position(|&x| x == b'\n').map_or(b.len(), |n| i + n)
}

pub fn parse_task_md(src: &str) -> TaskMd {
    let b = src.as_bytes();
    let mut properties = Properties::default();

    let mut cur = skip_while(b, 0, is_space);
    if b.get(cur) != Some(&b'#') {
        return TaskMd { title: None, properties, body: String::new() };
    }
    cur = skip_while(b, cur + 1, is_space_except_newline);
    let end = line_end(b, cur);
    let title = trim_end(&src[cur..end]).to_string();

    cur = skip_while(b, end, is_space);
    loop {
        let mut c = skip_while(b, cur, is_space_except_newline);
        if b.get(c) != Some(&b'-') {
            break;
        }
        c = skip_while(b, c + 1, is_space_except_newline);
        let key_start = c;
        c = skip_while(b, c, |x| x.is_ascii_alphanumeric());
        let key = &src[key_start..c];
        c = skip_while(b, c, is_space_except_newline);
        if key.is_empty() || b.get(c) != Some(&b':') {
            break;
        }
        let end = line_end(b, c + 1);
        properties.put(key, trim(&src[c + 1..end]));
        cur = if end < b.len() { end + 1 } else { end };
    }

    TaskMd { title: Some(title), properties, body: src[cur..].to_string() }
}

pub fn parse_tags(unparsed: &str) -> Vec<String> {
    split_list(unparsed).map(|(_, tag)| tag.to_string()).collect()
}

/// Like tags, but a leading `@` is optional: `alice` and `@alice` are the same person.
pub fn parse_assignees(unparsed: &str) -> Vec<String> {
    split_list(unparsed)
        .map(|(_, name)| name.strip_prefix('@').unwrap_or(name))
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

/// C's `atoi`: optional whitespace and sign, then digits; garbage yields 0.
fn atoi(s: &str) -> i64 {
    let s = trim(s);
    let (neg, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let n = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |acc, d| acc.saturating_mul(10).saturating_add(i64::from(d - b'0')));
    if neg { -n } else { n }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    /// False when TASK.md has no `# title`. Such tasks are listed but never rewritten,
    /// because rendering them back would destroy their content.
    pub well_formed: bool,
    pub status: String,
    pub priority: i64,
    pub tags: Vec<String>,
    /// Unlike `tags`, the ASSIGNEES property is optional and only written when used.
    pub assignees: Vec<String>,
    pub properties: Properties,
    pub body: String,
    pub content: String,
}

impl Task {
    pub fn new(id: String, title: String, priority: i64, tags: Vec<String>) -> Self {
        let mut properties = Properties::default();
        for key in [STATUS, PRIORITY, TAGS] {
            properties.put(key, "");
        }
        Task {
            id,
            title,
            well_formed: true,
            status: "OPEN".to_string(),
            priority,
            tags,
            assignees: Vec::new(),
            properties,
            body: String::new(),
            content: String::new(),
        }
    }

    pub fn from_md(id: String, content: String) -> Self {
        let md = parse_task_md(&content);
        let well_formed = md.title.is_some();
        let mut properties = md.properties;

        let status = properties.get(STATUS).unwrap_or("OPEN").to_string();
        let priority = properties.get(PRIORITY).map_or(UNSET_PRIORITY, atoi);
        let tags = properties.get(TAGS).map(parse_tags).unwrap_or_default();
        let assignees = properties.get(ASSIGNEES).map(parse_assignees).unwrap_or_default();
        // The well-known properties must exist because their position defines the
        // order in the rendered TASK.md.
        for key in [STATUS, PRIORITY, TAGS] {
            if properties.get(key).is_none() {
                properties.put(key, "");
            }
        }

        Task {
            id,
            title: md.title.unwrap_or_else(|| INVALID_TITLE.to_string()),
            well_formed,
            status,
            priority,
            tags,
            assignees,
            properties,
            body: md.body,
            content,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.status == "CLOSED"
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    pub fn unparsed_tags(&self) -> String {
        self.tags.join(",")
    }

    pub fn is_assigned_to(&self, name: &str) -> bool {
        self.assignees.iter().any(|a| a == name)
    }

    /// `status`, `priority`, `tags` and `assignees` take precedence over whatever is in `properties`.
    pub fn render(&self) -> String {
        let mut out = format!("# {}\n\n", self.title);
        for p in &self.properties.0 {
            match p.key.as_str() {
                STATUS => out += &format!("- STATUS: {}\n", self.status),
                PRIORITY => out += &format!("- PRIORITY: {}\n", self.priority),
                TAGS if self.tags.is_empty() => out += "- TAGS:\n",
                TAGS => out += &format!("- TAGS: {}\n", self.unparsed_tags()),
                ASSIGNEES if self.assignees.is_empty() => out += "- ASSIGNEES:\n",
                ASSIGNEES => out += &format!("- ASSIGNEES: {}\n", self.assignees.join(",")),
                _ if p.value.is_empty() => out += &format!("- {}:\n", p.key),
                _ => out += &format!("- {}: {}\n", p.key, p.value),
            }
        }
        if !self.assignees.is_empty() && self.properties.get(ASSIGNEES).is_none() {
            out += &format!("- ASSIGNEES: {}\n", self.assignees.join(","));
        }
        if !self.body.is_empty() && !self.body.starts_with('\n') && !self.body.starts_with("\r\n") {
            out.push('\n');
        }
        out += &self.body;
        out
    }

    /// One line in the `path:line: ...` format understood by Emacs compilation mode,
    /// VS Code, and most other editors.
    pub fn report(&self, tasks_dir: &str, updated: Option<&str>) -> String {
        let mut line = format!("{tasks_dir}/{}/TASK.md:1:", self.id);
        if let Some(update) = updated {
            line += &format!(" <UPDATED: {update}>");
        }
        line += &format!(" {} [PRIORITY: {:3}]", self.status, self.priority);
        if !self.tags.is_empty() {
            line += &format!(" [{}]", self.unparsed_tags());
        }
        if !self.assignees.is_empty() {
            let names: Vec<String> = self.assignees.iter().map(|a| format!("@{a}")).collect();
            line += &format!(" [{}]", names.join(","));
        }
        line += &format!(" {}", self.title);
        line
    }
}

pub fn load_tasks(dir: &Path) -> Result<Vec<Task>> {
    let entries = fs::read_dir(dir).context(format_args!("could not read {}", dir.display()))?;
    let mut tasks = Vec::new();
    for entry in entries {
        let entry = entry.context(format_args!("could not read {}", dir.display()))?;
        let Ok(id) = entry.file_name().into_string() else { continue };
        if id.starts_with('.') || !is_valid_huid(&id) {
            continue;
        }
        let task_path = entry.path();
        if !task_path.is_dir() {
            return Err(Error::Msg(format!("{id} is not a directory")));
        }
        let task_md_path = task_path.join("TASK.md");
        if !task_md_path.exists() {
            continue;
        }
        let bytes = fs::read(&task_md_path).context(format_args!("could not read {}", task_md_path.display()))?;
        let content = String::from_utf8(bytes)
            .map_err(|_| Error::Msg(format!("{} is not valid UTF-8", task_md_path.display())))?;
        tasks.push(Task::from_md(id, content));
    }
    Ok(tasks)
}

/// Highest priority (or newest id) first. Ties are broken by id so the order is stable
/// across platforms and filesystems.
pub fn sort_tasks(tasks: &mut [Task], by_id: bool) {
    if by_id {
        tasks.sort_by(|a, b| b.id.cmp(&a.id));
    } else {
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| b.id.cmp(&a.id)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "# Fix the thing\n\n- STATUS: OPEN\n- PRIORITY: 42\n- TAGS: bug, ui\n- AUTHOR: rexim\n\nSome description\n";

    #[test]
    fn parses_task_md() {
        let md = parse_task_md(SAMPLE);
        assert_eq!(md.title.as_deref(), Some("Fix the thing"));
        assert_eq!(md.properties.get("PRIORITY"), Some("42"));
        assert_eq!(md.properties.get("AUTHOR"), Some("rexim"));
        assert_eq!(md.body, "\nSome description\n");
    }

    #[test]
    fn render_round_trips() {
        let task = Task::from_md("20260101-000000".into(), SAMPLE.into());
        assert_eq!(task.tags, ["bug", "ui"]);
        assert_eq!(task.render(), SAMPLE.replace("bug, ui", "bug,ui"));
    }

    #[test]
    fn duplicated_properties_keep_last_value() {
        let task = Task::from_md("x".into(), "# T\n\n- STATUS: OPEN\n- STATUS: CLOSED\n".into());
        assert!(task.is_closed());
        assert_eq!(task.render(), "# T\n\n- STATUS: CLOSED\n- PRIORITY: 999999\n- TAGS:\n");
    }

    #[test]
    fn missing_properties_are_added_when_rendering() {
        let task = Task::from_md("x".into(), "# T\n\n- PRIORITY: 69\n- STATUS: OPEN\n\nNo tags\n".into());
        assert_eq!(task.render(), "# T\n\n- PRIORITY: 69\n- STATUS: OPEN\n- TAGS:\n\nNo tags\n");
    }

    #[test]
    fn body_without_properties_gets_separated() {
        let task = Task::from_md("x".into(), "# T\n\nJust text\n".into());
        assert_eq!(task.body, "Just text\n");
        assert_eq!(task.render(), "# T\n\n- STATUS: OPEN\n- PRIORITY: 999999\n- TAGS:\n\nJust text\n");
    }

    #[test]
    fn handles_crlf() {
        let task = Task::from_md("x".into(), "# Title\r\n\r\n- STATUS: CLOSED\r\n- PRIORITY: 7\r\n\r\nBody\r\n".into());
        assert_eq!(task.title, "Title");
        assert!(task.is_closed());
        assert_eq!(task.priority, 7);
        assert_eq!(task.body, "\r\nBody\r\n");
    }

    #[test]
    fn invalid_title() {
        let task = Task::from_md("x".into(), "no hash here\n".into());
        assert!(!task.well_formed);
        assert_eq!(task.title, INVALID_TITLE);
    }

    #[test]
    fn assignees_are_optional_and_round_trip() {
        let md = "# T\n\n- STATUS: OPEN\n- PRIORITY: 1\n- TAGS: bug\n- ASSIGNEES: @alice, bob\n\nBody\n";
        let task = Task::from_md("x".into(), md.into());
        assert_eq!(task.assignees, ["alice", "bob"]);
        assert!(task.is_assigned_to("bob"));
        assert_eq!(task.render(), md.replace("@alice, bob", "alice,bob"));

        let mut task = Task::from_md("x".into(), "# T\n\n- STATUS: OPEN\n- PRIORITY: 1\n- TAGS:\n\nBody\n".into());
        assert!(task.assignees.is_empty());
        assert!(!task.render().contains("ASSIGNEES"), "not added to tasks that don't use it");
        task.assignees.push("carol".into());
        assert_eq!(task.render(), "# T\n\n- STATUS: OPEN\n- PRIORITY: 1\n- TAGS:\n- ASSIGNEES: carol\n\nBody\n");
        assert!(task.report(".", None).ends_with("[PRIORITY:   1] [@carol] T"));
    }

    #[test]
    fn atoi_like_c() {
        assert_eq!(atoi("42"), 42);
        assert_eq!(atoi(" -7abc"), -7);
        assert_eq!(atoi("abc"), 0);
    }
}
