# Tasks

This folder contains tasks as defined by the specification of tt, a cross-platform rewrite of [tatr](https://github.com/tsoding/tatr).

Each sub-folder is a task named by its HUID (`YYYYMMDD-HHMMSS[-suffix]`, UTC) and contains a `TASK.md`:

```markdown
# <title>

- STATUS: (OPEN|CLOSED)
- PRIORITY: <number>
- TAGS: <comma-and-whitespace-separated-list-of-tags>

[description]
```

## Quick Start

```console
$ cargo install --path <path-to-tt-checkout>
```

After that run `tt ls` anywhere within this repo to get the list of currently opened tasks. Use `tt help` to list all the available subcommands of the tool.
