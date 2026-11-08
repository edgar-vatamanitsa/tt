# tt — Task Tracker

A tiny plain-text task system that lives in the `tasks/` folder of your repo, plus a command line tool to navigate and manipulate it. Runs on **Linux, macOS and Windows**.

`tt` is a Rust rewrite of [tatr](https://github.com/tsoding/tatr) by [Alexey Kutepov (rexim)](https://github.com/rexim). Big shout-out to the author of tatr for this simple and brilliant idea!

## Install

From a local checkout (any platform with a [Rust toolchain](https://rustup.rs/)):

```console
$ cargo install --path .
```

```console
$ tt help
```

## Quick start

```console
$ tt init                                   # create ./tasks/ with a README
$ tt new -p 50 -t bug,ui Button is ugly     # create a task
$ tt ls                                     # list open tasks
$ tt ls :bug and priority lt 100            # query tasks
$ tt ls -update close,+done :ui             # mass update
$ tt todo src/main.rs                       # turn TODO comments into tasks
$ tt summary                                # count tasks per tag
```

Output lines look like `./tasks/<HUID>/TASK.md:1: ...`, which Emacs compilation mode, VS Code, and most other editors and terminals can jump to.

## Tutorial

This walks through a typical day with `tt` in a small project called `myapp`. All output below is real. Your task IDs will differ, because they are the UTC time at which each task was created.

### 1. Set up the tasks folder

Run `tt init` once at the root of your project:

```console
$ cd myapp
$ tt init
Created ./tasks/
Created ./tasks/README.md
```

The `README.md` explains the folder to anyone who stumbles upon it. Skip it with `tt init -no-readme`.

### 2. Create tasks

The title is everything after the flags. `-p` sets the priority (default 100) and `-t` adds tags:

```console
$ tt new -p 50 -t bug Crash when config file is missing
./tasks/20261108-125157/TASK.md:1: OPEN [PRIORITY:  50] [bug] Crash when config file is missing

$ tt new -t enhancement,ui Add dark mode
./tasks/20261108-125158/TASK.md:1: OPEN [PRIORITY: 100] [enhancement,ui] Add dark mode

$ tt new Write installation docs
./tasks/20261108-125159/TASK.md:1: OPEN [PRIORITY: 100] Write installation docs
```

If a task with the exact same title already exists, `tt new` asks before creating a duplicate. Pass `-y` to skip the question.

When you work in a team, give everyone their own suffix so IDs never collide, e.g. `tt new -s alice Fix login`, which creates `20261108-130000-alice`.

### 3. List tasks

```console
$ tt ls
./tasks/20261108-125159/TASK.md:1: OPEN [PRIORITY: 100] Write installation docs
./tasks/20261108-125158/TASK.md:1: OPEN [PRIORITY: 100] [enhancement,ui] Add dark mode
./tasks/20261108-125157/TASK.md:1: OPEN [PRIORITY:  50] [bug] Crash when config file is missing
```

Tasks are sorted by priority, highest first. Use `-a` to reverse the order, `-id` to sort by creation time, and `-c` to see closed tasks instead of open ones.

### 4. Describe the task

A task is just a markdown file, so open `TASK.md` in your editor and write down whatever you know. Everything below the properties is free-form:

```markdown
# Crash when config file is missing

- STATUS: OPEN
- PRIORITY: 50
- TAGS: bug

Happens on first launch when ~/.myapp/config.toml does not exist.
```

You can edit the title and properties by hand too. Put screenshots, logs or other attachments next to `TASK.md` in the same folder and link to them.

### 5. Turn TODO comments into tasks

Say `src/main.c` has a TODO comment:

```c
int main(void) {
    load_config();
    // TODO: handle errors from load_config
    return 0;
}
```

`tt todo` finds every TODO in the given files, shows it with some context, and asks whether to create a task from it:

```console
$ tt todo -t bug src/main.c
src/main.c:3: Found a TODO: handle errors from load_config
int main(void) {
    load_config();
    // TODO: handle errors from load_config
    return 0;
}
Create a task from it? [y,n,q,a,c,?] y
./tasks/20261108-125200/TASK.md:1: OPEN [PRIORITY: 100] [bug] handle errors from load_config
```

The comment in the source file now points to the task:

```c
    // TASK(20261108-125200): handle errors from load_config
```

Answer `?` to see all the options: `a` says yes to the rest of the file, `c` skips the rest of the file, and `q` quits.

### 6. Find tasks with queries

Everything after `tt ls` is a query (see [TQL](#tt-query-language-tql) for the full language):

```console
$ tt ls :bug
./tasks/20261108-125200/TASK.md:1: OPEN [PRIORITY: 100] [bug] handle errors from load_config
./tasks/20261108-125157/TASK.md:1: OPEN [PRIORITY:  50] [bug] Crash when config file is missing

$ tt ls :bug and priority lt 100
./tasks/20261108-125157/TASK.md:1: OPEN [PRIORITY:  50] [bug] Crash when config file is missing

$ tt ls not tagged
./tasks/20261108-125159/TASK.md:1: OPEN [PRIORITY: 100] Write installation docs
```

### 7. Update many tasks at once

Add `-update` to change every task the query selects. Here every untagged task gets the `docs` tag:

```console
$ tt ls -update +docs not tagged
./tasks/20261108-125159/TASK.md:1: <UPDATED: +docs> OPEN [PRIORITY: 100] [docs] Write installation docs
1 tasks were updated
```

Tip: run the query without `-update` first to see exactly which tasks it will touch.

### 8. Get an overview

```console
$ tt summary
STATUS:   OPEN
TOTAL:    4
TAGGED:
            bug =>   2
           docs =>   1
    enhancement =>   1
             ui =>   1
```

Describe your tags in `tasks/tags` (one `<tag> , <description>` per line) and `tt summary` prints the descriptions next to the counts.

### 9. See where a task is mentioned

`tt ref` searches the whole project for a task's ID, so you can find the code to change:

```console
$ tt ref 20261108-125200
./src/main.c:3:    // TASK(20261108-125200): handle errors from load_config
```

Mention task IDs anywhere (in code, commit messages, or other tasks) and `tt ref` will find them.

### 10. Close the task

When the fix is done, close the task. A HUID on its own is a query that selects exactly that task:

```console
$ tt ls -update close,+fixed 20261108-125157
./tasks/20261108-125157/TASK.md:1: <UPDATED: close,+fixed> CLOSED [PRIORITY:  50] [bug,fixed] Crash when config file is missing
1 tasks were updated

$ tt ls -c
./tasks/20261108-125157/TASK.md:1: CLOSED [PRIORITY:  50] [bug,fixed] Crash when config file is missing
```

Closed tasks drop out of `tt ls`, but stay on disk and in history. `tt find <HUID>` finds a task whether it's open or closed, and `-path-only` prints just its path, which is handy for opening it in an editor:

```console
$ tt find 20261108-125157 -path-only
./tasks/20261108-125157/TASK.md
```

### 11. Work from inside a task folder

Inside a task's folder, `tt id` prints its ID without a newline, ready for the clipboard. `tt ref` with no arguments searches for the current task:

```console
$ cd tasks/20261108-125200
$ tt id | pbcopy        # macOS; use `clip` on Windows, `xclip` on Linux
$ tt ref
../../src/main.c:3:    // TASK(20261108-125200): handle errors from load_config
```

### 12. Commit the tasks with your code

The `tasks/` folder is plain files, so commit it like any other code:

```console
$ git add tasks src/main.c
$ git commit -m "Track config crash, link TODO to TASK(20261108-125200)"
```

Because task IDs are timestamps, tasks created on different branches don't clash when you merge. Use `git log tasks/<HUID>` to see a task's history.

## The Spec

### Layout

Each project at the root has `tasks/` folder which contains sub-folders for each task.

```
project/
+-...
+-tasks/
| +-tags
| +-20260824-215300
| | +-TASK.md
| | +-...
| +-20260830-000403-rexim
| | +-TASK.md
| | +-screenshot.png
| | +-...
| +-...
+-...
```

### HUID

Each task sub-folder is named with a Task ID. The Task ID format is `[0-9]{8}-[0-9]{6}`. Just grab the current date and time and use it as the Task ID. Use UTC timezone so the current timezone is irrelevant. If your Task ID collides with an existing one, just wait one second and try again.

The full format is actually `[0-9]{8}-[0-9]{6}(-[a-zA-Z0-9\-]*)?`. So if you work in a team you can agree on unique suffixes per individual to slap at the end like `20260829-235855-rexim` or `20260829-235902-01` (`tt new -s rexim ...`). Those are valid Task IDs too.

This ID system is called HUID (Human-Unique IDentifier). It is fairly unique if you generate IDs at "Human-speed", that is never more than one per second.

Having a Unique Task ID is beneficial under version control systems like git, because you can generate tasks in parallel branches and then relatively easily merge them together.

### TASK.md

Inside of the task sub-folder there is one mandatory file `TASK.md` which is a markdown file describing the task. The folder may contain other files as attachments to the task. `TASK.md` should link to the attachments as necessary. Try to keep the size of the attachments small, since they are going to be committed to the git repo.

The format of `TASK.md`:

```markdown
# <title>

- STATUS: (OPEN|CLOSED)
- PRIORITY: <number>
- TAGS: <comma-and-whitespace-separated-list-of-tags>
[other properties]

[description]
```

As you work on the task feel free to append any discovered details about the task to the `[description]`.

Use [git-blame](https://git-scm.com/docs/git-blame) and [git-log](https://git-scm.com/docs/git-log) to learn about when, how and by whom any changes to the task were made.

#### Task Properties

The `STATUS` property defines whether the task is done or not. There could be only two statuses. If you need more it is generally recommended to use `TAGS` for whatever you are trying to do.

The `PRIORITY` is generally used for sorting the tasks. Don't think of priority as an absolute value. Think of it as the means of getting your list of tasks sorted in the way you want to see them. A task without a `PRIORITY` sorts first (999999), so you don't forget to set it.

The `TAGS` property contains the list of tags separated by commas and whitespaces. `TAGS: foo,bar,baz` defines 3 tags. `TAGS: foo,,, hello  world` also defines 3 tags. You can use these tags to group tasks into categories, like `bug` or `enhancement`.

You may specify `[other properties]` (in addition to `STATUS`, `PRIORITY`, and `TAGS`) in a similar format (that is `- [NAME]: [VALUE]`). `tt` ignores them but keeps them intact during mass update operations.

Duplicated properties are not allowed. If a task specifies duplicated properties only the last value is taken into account. Mass update operations remove any duplicated properties.

### Tags description file

There might be an optional `tasks/tags` file with the following format:

```
<tag-name> [,] <tag-description>
<tag-name> [,] <tag-description>
...
```

It serves as documentation for each existing tag. `tt summary` displays the descriptions.

## Commands

| Command | Description |
|-|-|
| `tt init` | Create `tasks/` in the current directory (with a `README.md` unless `-no-readme`). |
| `tt ls [QUERY...]` | List open tasks matching the query (`-c` closed, `-a` ascending, `-id` sort by id, `-update` apply updates). |
| `tt new [TITLE...]` | Create a task (`-p` priority, `-t` tags, `-s` id suffix, `-y` no prompts). |
| `tt todo <FILE...>` | Interactively turn TODO comments into tasks, replacing them with `TASK(<HUID>)`. |
| `tt summary` | Count open (or `-c` closed) tasks per tag. |
| `tt find <HUID>` | Find a task regardless of its status (`-path-only` prints just the path). |
| `tt id` | Print the HUID of the task folder you're in (pipe to `pbcopy`, `clip` or `xclip`). |
| `tt ref [HUID]` | Find all references to a task in the repo (skips `.git` and binary files). |
| `tt graph` | Write a `graph.dot` of tasks referring to each other and render it with Graphviz `neato`. |
| `tt help [COMMAND]` | Show help. `tt <COMMAND> -help` works too. |
| `tt version` | Show version, commit, build time and compiler. |

Flags use a single dash (`-c`, `-update`), and `--c` style is accepted too. Flags come before positional arguments; use `--` to pass arguments that start with a dash, e.g. `tt ls -- priority gt -1`.

### tt Query Language (TQL)

The query language used by `tt ls` to select a set of tasks.

```console
$ tt ls :bug                        # everything tagged bug
$ tt ls :bug and not :ui            # bugs that are not ui
$ tt ls not tagged                  # everything without tags
$ tt ls :bug and priority lt 50     # bugs with priority below 50
$ tt ls not [:bug or :ui]           # grouping uses square brackets
```

#### Syntax

```
<expr>            ::= <or>
<or>              ::= <and> *('or' <and>)
<and>             ::= <compare> *('and' <compare>)
<compare>         ::= <primary> *(<compare-op> <primary>)
<compare-op>      ::= 'lt' | 'le' | 'gt' | 'ge' | 'eq' | 'ne'
<primary>         ::= <tag>
                    | '[' <expr> ']'
                    | 'not' <primary>
                    | 'any'
                    | 'tagged'
                    | 'priority'
                    | <number>
                    | <huid>
<tag>             ::= ':' 1*<any-character-except-whitespaces-and-square-brackets>
<number>          ::= ['-'] 1*<digit>
<huid>            ::= 8<digit> '-' 6<digit> [ '-' [ <huid-suffix> ] ]
<huid-suffix>     ::= 'A'-'Z' | 'a'-'z' | '-' | <digit>
<digit>           ::= '0'-'9'
```

The syntax is designed to be used in shell environments without requiring any special escaping. Parentheses and `<`/`>` have special meanings in shells, so grouping uses square brackets and comparisons follow the [test(1)](https://www.man7.org/linux/man-pages/man1/test.1.html) convention: `lt`, `le`, `gt`, `ge`, `eq`, `ne`. Avoid square brackets in tag names if you want to query them.

#### Reference

| Expression | Description |
|-|-|
| `<a> or <b>` | True when `<a>` or `<b>` or both are true. |
| `<a> and <b>` | True when both `<a>` and `<b>` are true. |
| `:<tag>` | True when a task's `TAGS` property contains `<tag>`. |
| `not <expr>` | True when `<expr>` is false. |
| `tagged` | True when a task has at least one tag. |
| `<huid>` | True when a task has id equal to `<huid>`. |
| `any` | Always true. |
| `priority` | Priority of the task as an integer. |
| `<a> lt <b>` | True when `<a>` is less than `<b>`. |
| `<a> gt <b>` | True when `<a>` is greater than `<b>`. |
| `<a> le <b>` | True when `<a>` is less or equal to `<b>`. |
| `<a> ge <b>` | True when `<a>` is greater or equal to `<b>`. |
| `<a> eq <b>` | True when `<a>` is equal to `<b>`. |
| `<a> ne <b>` | True when `<a>` is not equal to `<b>`. |

Use `tt ls -debug <query>` to see how a query is tokenized and compiled.

### Update Language

A comma- or whitespace-separated list of actions applied to every task selected by `tt ls`:

```console
$ tt ls -update 'close,-scope' :bug
```

The command above closes all the tasks tagged `bug` and removes the `scope` tag from them. An update with an empty query is refused; use `any` to update everything.

| Action | Description |
|-|-|
| `+tag` | Add tag to the task |
| `-tag` | Remove tag from the task |
| `close` | Close the task |
| `open` | Open the task |

## Differences from tatr

- Works on Linux, macOS and Windows. Paths in the output always use `/`, which Windows accepts too.
- `tt ref` searches the repo itself, without calling `grep`.
- Type errors in queries (`:bug lt 5`) are reported once, before any task is loaded.
- `-update` keeps the order of the remaining tags when removing one, and refuses to rewrite a `TASK.md` that doesn't start with `# <title>`, so its content is never lost.
- `tt new -s` validates the suffix. `CRLF` files are handled.
- `--` ends the flags, so negative numbers can be used in queries.
- The deprecated `tag` and `untag` commands are gone. Use `tt ls -update +tag ...` and `tt ls -update -tag ...` instead.

## Development

```console
$ cargo test          # unit + end-to-end tests
$ cargo run -- ls     # run against this repo's own tasks/
```

## License

Like the original tatr, `tt` is released under the [GNU General Public License, version 2](./LICENSE) or (at your option) any later version.
