// End-to-end tests that run the real `tt` binary. The first few are ports of the
// tests from tatr's nob.c.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const TT: &str = env!("CARGO_BIN_EXE_tt");
const MARKER: &str = concat!("TO", "DO");

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("tt-test-{}-{n}-{name}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        Sandbox { root }
    }

    fn with_tasks(name: &str) -> Sandbox {
        let sandbox = Sandbox::new(name);
        sandbox.ok(&["init"]);
        sandbox
    }

    fn write(&self, rel: &str, content: &str) -> PathBuf {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.root.join(rel)).unwrap()
    }

    fn run_in(&self, dir: &Path, args: &[&str], stdin: &str) -> Output {
        let mut child = Command::new(TT)
            .args(args)
            .current_dir(dir)
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
        child.wait_with_output().unwrap()
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_in(&self.root, args, "")
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert_success(&output, args);
        String::from_utf8(output.stdout).unwrap()
    }

    fn task_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = fs::read_dir(self.root.join("tasks"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.as_bytes()[0].is_ascii_digit())
            .collect();
        ids.sort();
        ids
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn assert_success(output: &Output, args: &[&str]) {
    assert!(
        output.status.success(),
        "`tt {}` failed\nSTDOUT:\n{}\nSTDERR:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}

fn debug_query(query: &str) -> Output {
    Command::new(TT).args(["ls", "-debug", query]).output().unwrap()
}

#[test]
fn ls_query_negation_of_complex_expression_in_parens() {
    let output = debug_query("not [tagged or :bug and :test and :foo and :bar]");
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "QUERY TOKENS:\n    not\n    [\n    tagged\n    or\n    :bug\n    and\n    :test\n    and\n    :foo\n    and\n    :bar\n    ]\n\n\
         QUERY OPS:\n    QOP_TAGGED\n    QOP_TAG bug\n    QOP_TAG test\n    QOP_AND\n    QOP_TAG foo\n    QOP_AND\n    QOP_TAG bar\n    QOP_AND\n    QOP_OR\n    QOP_NOT\n"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn ls_query_not_stuck_to_open_paren() {
    let output = debug_query("not[:bug and :test] and :query");
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "QUERY TOKENS:\n    not\n    [\n    :bug\n    and\n    :test\n    ]\n    and\n    :query\n\n\
         QUERY OPS:\n    QOP_TAG bug\n    QOP_TAG test\n    QOP_AND\n    QOP_NOT\n    QOP_TAG query\n    QOP_AND\n"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn ls_query_report_error_utf8() {
    let output = debug_query(":привет hello");
    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Supported infix operators:\n\n    and  or                  - logical operators\n    lt  le  gt  ge  eq  ne   - comparison operators\n\n\
         :привет hello\n        ^\nERROR: Unexpected infix operator `hello`\n"
    );
}

#[test]
fn ls_query_priority_above_20() {
    let output = debug_query("priority gt 20");
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "QUERY TOKENS:\n    priority\n    gt\n    20\n\nQUERY OPS:\n    QOP_PRIORITY\n    QOP_INTEGER 20\n    QOP_GT\n"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn ls_query_type_error_points_at_operand() {
    let output = debug_query(":bug lt 5");
    assert!(!output.status.success());
    assert_eq!(stderr(&output), ":bug lt 5\n^\nERROR: Expected integer but got boolean\n");
}

#[test]
fn new_sanity() {
    let sb = Sandbox::with_tasks("new");
    let out = sb.ok(&["new", "-p", "69", "-t", "foo,bar", "-t", "baz", "Sanity check task"]);
    let ids = sb.task_ids();
    assert_eq!(ids.len(), 1);
    let id = &ids[0];
    assert_eq!(out, format!("./tasks/{id}/TASK.md:1: OPEN [PRIORITY:  69] [foo,bar,baz] Sanity check task\n"));
    assert_eq!(
        sb.read(&format!("tasks/{id}/TASK.md")),
        "# Sanity check task\n\n- STATUS: OPEN\n- PRIORITY: 69\n- TAGS: foo,bar,baz\n"
    );
}

#[test]
fn new_with_suffix_and_invalid_suffix() {
    let sb = Sandbox::with_tasks("suffix");
    sb.ok(&["new", "-s", "rexim", "Mine"]);
    assert!(sb.task_ids()[0].ends_with("-rexim"));
    let output = sb.run(&["new", "-s", "bad/suffix", "Nope"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("not a valid task id suffix"));
}

#[test]
fn new_asks_about_duplicate_titles() {
    let sb = Sandbox::with_tasks("dup");
    sb.ok(&["new", "Same"]);
    let output = sb.run_in(&sb.root, &["new", "Same"], "n\n");
    assert_success(&output, &["new", "Same"]);
    assert!(stdout(&output).contains("Task with this exact title already exists"));
    assert_eq!(sb.task_ids().len(), 1);
    // -y skips the question; the second task needs a fresh HUID, so it may wait a second.
    sb.ok(&["new", "-y", "Same"]);
    assert_eq!(sb.task_ids().len(), 2);
}

#[test]
fn todo_sanity() {
    let sb = Sandbox::with_tasks("todo");
    assert_eq!(sb.ok(&["ls", "-id"]).matches('\n').count(), 1);

    sb.write("todos", &format!("// {MARKER}: task 1\n/* {MARKER}: task 2\n*/\n# {MARKER}: task 3\n"));
    let output = sb.run_in(&sb.root, &["todo", "./todos"], "y\nn\ny\n");
    assert_success(&output, &["todo"]);

    let todos = sb.read("todos");
    assert_eq!(todos.matches('\n').count(), 4);
    assert_eq!(todos.matches(MARKER).count(), 1);
    assert_eq!(todos.matches("TASK").count(), 2);
    assert!(todos.contains(&format!("/* {MARKER}: task 2")));

    let ids = sb.task_ids();
    assert_eq!(ids.len(), 2);
    assert!(todos.starts_with(&format!("// TASK({}): task 1\n", ids[0])));
    assert_eq!(sb.ok(&["ls", "-id"]).matches('\n').count(), 2);
}

#[test]
fn todo_quits_on_eof_and_preserves_crlf() {
    let sb = Sandbox::with_tasks("todo-crlf");
    sb.write("src.c", &format!("int x;\r\n// {MARKER}: fix me\r\n"));
    let output = sb.run_in(&sb.root, &["todo", "src.c"], "y\n");
    assert_success(&output, &["todo"]);
    let id = &sb.task_ids()[0];
    assert_eq!(sb.read("src.c"), format!("int x;\r\n// TASK({id}): fix me\r\n"));
    assert!(sb.read(&format!("tasks/{id}/TASK.md")).starts_with("# fix me\n"));

    // Closed stdin means quit, not a crash.
    sb.write("more.c", &format!("// {MARKER}: later\n"));
    let output = sb.run_in(&sb.root, &["todo", "more.c"], "");
    assert_success(&output, &["todo"]);
    assert_eq!(sb.task_ids().len(), 1);
}

#[test]
fn tag_task_without_tags_property() {
    let sb = Sandbox::with_tasks("no-tags");
    let path = "tasks/20260101-000000/TASK.md";
    sb.write(path, "# Test\n\n- PRIORITY: 69\n- STATUS: OPEN\n\nA task without TAGS property\n");
    sb.ok(&["ls", "-update", "+aboba", "any"]);
    let content = sb.read(path);
    assert_eq!(content.matches("aboba").count(), 1);
    assert_eq!(content, "# Test\n\n- PRIORITY: 69\n- STATUS: OPEN\n- TAGS: aboba\n\nA task without TAGS property\n");
}

fn seed(sb: &Sandbox) {
    sb.write("tasks/20260101-000001/TASK.md", "# Crash on start\n\n- STATUS: OPEN\n- PRIORITY: 10\n- TAGS: bug\n");
    sb.write(
        "tasks/20260101-000002/TASK.md",
        "# Button is ugly\n\n- STATUS: OPEN\n- PRIORITY: 60\n- TAGS: bug, ui\n- OWNER: me\n\nDetails.\n",
    );
    sb.write(
        "tasks/20260101-000003/TASK.md",
        "# Write docs\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n\nSee 20260101-000001.\n",
    );
    sb.write("tasks/20260101-000004/TASK.md", "# Old thing\n\n- STATUS: CLOSED\n- PRIORITY: 5\n- TAGS: bug\n");
    sb.write("tasks/20260101-000005/notes.txt", "not a task: no TASK.md\n");
    sb.write("tasks/.hidden/TASK.md", "# Hidden\n");
}

#[test]
fn ls_filters_sorts_and_reports() {
    let sb = Sandbox::with_tasks("ls");
    seed(&sb);
    assert_eq!(
        sb.ok(&["ls"]),
        "./tasks/20260101-000003/TASK.md:1: OPEN [PRIORITY: 100] Write docs\n\
         ./tasks/20260101-000002/TASK.md:1: OPEN [PRIORITY:  60] [bug,ui] Button is ugly\n\
         ./tasks/20260101-000001/TASK.md:1: OPEN [PRIORITY:  10] [bug] Crash on start\n"
    );
    assert_eq!(sb.ok(&["ls", ":bug", "and", "not", ":ui"]).lines().count(), 1);
    assert_eq!(sb.ok(&["ls", "not tagged"]).lines().count(), 1);
    assert_eq!(sb.ok(&["ls", ":bug and priority lt 50"]).lines().count(), 1);
    assert_eq!(
        sb.ok(&["ls", "-c"]).lines().collect::<Vec<_>>(),
        ["./tasks/20260101-000004/TASK.md:1: CLOSED [PRIORITY:   5] [bug] Old thing"]
    );
    assert_eq!(
        sb.ok(&["ls", "-a"]).lines().next().unwrap(),
        "./tasks/20260101-000001/TASK.md:1: OPEN [PRIORITY:  10] [bug] Crash on start"
    );
    assert!(sb.ok(&["ls", "-id"]).starts_with("./tasks/20260101-000003/"));
    assert_eq!(sb.ok(&["ls", ":nothing"]), "No tasks were found\n");
    assert_eq!(sb.ok(&["ls", "20260101-000002"]).lines().count(), 1);
    // Negative numbers can be passed after `--`.
    assert_eq!(sb.ok(&["ls", "--", "priority", "gt", "-1"]).lines().count(), 3);
}

#[test]
fn ls_paths_are_relative_to_cwd() {
    let sb = Sandbox::with_tasks("rel");
    seed(&sb);
    fs::create_dir_all(sb.root.join("src/deep")).unwrap();
    let out = String::from_utf8(sb.run_in(&sb.root.join("src/deep"), &["ls", ":ui"], "").stdout).unwrap();
    assert!(out.starts_with("../../tasks/20260101-000002/TASK.md:1:"), "{out}");
    let out = String::from_utf8(sb.run_in(&sb.root.join("tasks"), &["ls", ":ui"], "").stdout).unwrap();
    assert!(out.starts_with("./20260101-000002/TASK.md:1:"), "{out}");
}

#[test]
fn ls_update_closes_and_retags() {
    let sb = Sandbox::with_tasks("update");
    seed(&sb);
    let out = sb.ok(&["ls", "-update", "close,-bug", "-update", "+done", ":ui"]);
    assert_eq!(
        out,
        "./tasks/20260101-000002/TASK.md:1: <UPDATED: close,-bug +done> CLOSED [PRIORITY:  60] [ui,done] Button is ugly\n1 tasks were updated\n"
    );
    assert_eq!(
        sb.read("tasks/20260101-000002/TASK.md"),
        "# Button is ugly\n\n- STATUS: CLOSED\n- PRIORITY: 60\n- TAGS: ui,done\n- OWNER: me\n\nDetails.\n"
    );
    // Reopen it from the closed list.
    sb.ok(&["ls", "-c", "-update", "open", ":done"]);
    assert!(sb.read("tasks/20260101-000002/TASK.md").contains("- STATUS: OPEN\n"));
}

#[test]
fn ls_update_requires_query_and_valid_actions() {
    let sb = Sandbox::with_tasks("update-errors");
    seed(&sb);
    let output = sb.run(&["ls", "-update", "close"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("-update with empty query is not allowed"));
    let output = sb.run(&["ls", "-update", "reopen", "any"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("ERROR: Unknown action `reopen`"));
}

#[test]
fn ls_never_rewrites_malformed_task() {
    let sb = Sandbox::with_tasks("malformed");
    let path = "tasks/20260101-000009/TASK.md";
    sb.write(path, "Oops, no title\n");
    let output = sb.run(&["ls", "-update", "+x", "any"]);
    assert!(stderr(&output).contains("not updating the task"));
    assert_eq!(sb.read(path), "Oops, no title\n");
}

#[test]
fn find_and_path_only() {
    let sb = Sandbox::with_tasks("find");
    seed(&sb);
    assert_eq!(
        sb.ok(&["find", "20260101-000004"]),
        "./tasks/20260101-000004/TASK.md:1: CLOSED [PRIORITY:   5] [bug] Old thing\n"
    );
    assert_eq!(sb.ok(&["find", "20260101-000004", "-path-only"]), "./tasks/20260101-000004/TASK.md\n");
    let output = sb.run(&["find", "nope"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("`nope` is not a valid HUID"));
    let output = sb.run(&["find", "20990101-000000"]);
    assert!(stderr(&output).contains("No task with HUID"));
}

#[test]
fn id_inside_task_folder() {
    let sb = Sandbox::with_tasks("id");
    seed(&sb);
    let output = sb.run_in(&sb.root.join("tasks/20260101-000002"), &["id"], "");
    assert_success(&output, &["id"]);
    assert_eq!(stdout(&output), "20260101-000002");
    let output = sb.run(&["id"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("You must be inside of a specific task folder like ./tasks/<HUID>/"));
}

#[test]
fn ref_searches_repo_natively() {
    let sb = Sandbox::with_tasks("ref");
    seed(&sb);
    sb.write("src/main.c", "int main() {\r\n    // TASK(20260101-000001): crash\r\n}\n");
    sb.write(".git/HEAD", "20260101-000001\n");
    fs::write(sb.root.join("src/blob.bin"), b"\x00\x0120260101-000001").unwrap();

    let out = sb.ok(&["ref", "20260101-000001"]);
    assert_eq!(
        out,
        "./src/main.c:2:    // TASK(20260101-000001): crash\n./tasks/20260101-000003/TASK.md:7:See 20260101-000001.\n"
    );

    // Inside a task folder the HUID is picked up automatically.
    let output = sb.run_in(&sb.root.join("tasks/20260101-000001"), &["ref"], "");
    assert_success(&output, &["ref"]);
    assert!(stdout(&output).starts_with("../../src/main.c:2:"), "{}", stdout(&output));

    let output = sb.run(&["ref", "20260101-000004"]);
    assert!(!output.status.success());
}

#[test]
fn summary_counts_tags_with_descriptions() {
    let sb = Sandbox::with_tasks("summary");
    seed(&sb);
    sb.write("tasks/tags", "bug , unintended behavior\nui,, user interface\n");
    assert_eq!(
        sb.ok(&["summary"]),
        "STATUS:   OPEN\nTOTAL:    3\nUNTAGGED: 1\nTAGGED:\n    bug =>   2 - unintended behavior\n     ui =>   1 - user interface\n"
    );
    assert_eq!(
        sb.ok(&["summary", "-c"]),
        "STATUS:   CLOSED\nTOTAL:    1\nTAGGED:\n    bug =>   1 - unintended behavior\n"
    );
}

#[test]
fn init_creates_readme_once() {
    let sb = Sandbox::new("init");
    assert_eq!(sb.ok(&["init"]), "Created ./tasks/\nCreated ./tasks/README.md\n");
    assert!(sb.read("tasks/README.md").contains("tt"));
    assert_eq!(sb.ok(&["init"]), "");

    let sb = Sandbox::new("init-no-readme");
    sb.ok(&["init", "-no-readme"]);
    assert!(!sb.root.join("tasks/README.md").exists());
}

#[test]
fn help_and_errors() {
    let sb = Sandbox::new("help");
    let out = sb.ok(&["help"]);
    assert!(out.starts_with("tt - Task Tracker\n"));
    assert!(out.contains("    ls      - List and update the tasks.\n"));
    let out = sb.ok(&["help", "new"]);
    assert!(out.contains("    -p <int>\n        Priority of the new task\n        Default: 100\n"));
    assert_eq!(sb.ok(&["new", "-help"]), out);
    assert!(sb.ok(&["version"]).starts_with("tt "));

    let output = sb.run(&["frobnicate"]);
    assert!(!output.status.success());
    assert!(stderr(&output).ends_with("ERROR: Unknown command `frobnicate`\n"));
    let output = sb.run(&["ls", "-x"]);
    assert!(!output.status.success());
    assert!(stderr(&output).ends_with("ERROR: -x: unknown flag\n"));
    let output = sb.run(&[]);
    assert!(stderr(&output).ends_with("ERROR: No command is provided\n"));
}
