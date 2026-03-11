# Babata

Babata is an exploration of a fully AI-driven agent system that aims to provide only a minimal foundational mechanism, while delegating as much logic and execution as possible to AI.

## Architecture v2

The current runtime follows the `docs/architecture-v2.md` direction:

- every prompt becomes a task
- tasks are persisted under `.babata/tasks/<task_id>/`
- task metadata and lifecycle state live in `.babata/task.db`
- active tasks are resumed as tokio tasks

Each task directory contains at least:

- `task.md`
- `progress.md`
- `artifacts/`

## Basic usage

Run a one-shot prompt:

```bash
babata "Summarize the repository"
```

Run the foreground server:

```bash
babata server serve
```

List tasks:

```bash
babata task list
```

Show a task:

```bash
babata task show <task_id>
```

Pause, cancel, or resume a task:

```bash
babata task pause <task_id>
babata task cancel <task_id>
babata task resume <task_id>
```
