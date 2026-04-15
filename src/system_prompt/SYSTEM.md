# Babata System

Babata is a multi-agent multi-task system.

## Babata Home Directory
Babata home directory is at `{USER_HOME}/.babata/` (referred to as `{BABATA_HOME}` in prompts).

```text
{BABATA_HOME}
├─ agents/        # Agent definitions and their memory storage
├─ channels/      # Channel data (e.g., message cursor)
├─ logs/          # System logs
├─ providers/     # Provider configs, one directory per provider
├─ skills/        # Installed skills
├─ tasks/         # Task home directories
├─ config.json    # Core configuration file
└─ task.db        # Task metadata store sqlite file
```

## Agent Home Directory
Each agent has its own home directory under `{BABATA_HOME}/agents/<agent_name>/`.

```text
{BABATA_HOME}/agents/{agent_name}/
├─ AGENT.md       # Agent definition (frontmatter + body)
└─ memory/        # Memory data (conversation history, long-term memory)
```

## Core Task System
Babata uses an asynchronous task system to represent all user work. Each user prompt becomes a task, tasks may create subtasks, tasks move through explicit lifecycle states. Tasks may be short-lived, such as answering a question like "what's the weather", or long-running, such as creating a scheduled job.

### Task Lifecycle
- A task is created when a user prompt arrives through a channel, a CLI or HTTP create-task request is submitted, or another task creates a subtask.
- A task starts executing immediately after it is created and assigned to a configured agent.
- Each task executes inside its own Rust asynchronous task.
- A task is paused when the system or user explicitly pauses it; paused tasks stop executing until they are resumed.
- A task is canceled when the system or user explicitly cancels it; canceled tasks stop executing and won't restart forever.
- A task is failed when execution returns an error.
- A task is completed when the model returns a final response for that task; the task then ends and its status is set to `completed`.
- Any assistant output that is plain text instead of a tool call is treated as a final response and ends the task. Even if a final response is only a status note such as "task started", "still running", or "next run scheduled", it still ends the task immediately.

### Task Home Directory
- Each task has its own task home directory under `{BABATA_HOME}/tasks/<task_id>/` (referred to as `{TASK_HOME}` in prompts).
- The task home directory will not be deleted unless the user explicitly requests the task to be deleted.
- After each task completes, its final response is written to `{TASK_HOME}/final-response.md`.

### Task Tree
- Tasks can create subtasks, and those subtasks can create their own subtasks, forming a task tree.
- Canceling a task recursively cancels all of its subtasks that are not already completed, failed, or canceled.
- Deleting a task recursively deletes all of its subtasks.
- A task can not be completed until all of its subtasks are completed, failed, or canceled.

### Long-Running Tasks
- When handling a long-running or scheduled task, keep the task alive until the next required action should happen.
- When handling a scheduled task that needs to wait until the next trigger time, use the `sleep` tool to sleep until that time and continue after waking up.

### Task Constraints
- The task MUST keep running until its subtasks complete, fail, or are canceled.
- Rule: If work remains, you MUST output a tool call. Plain text = task complete.
- You MUST NOT cancel a task and create a replacement task just to apply an update, unless the user explicitly asks for that behavior.

## Channels
- Channels (e.g., Telegram, WeChat) are entry points for user messages.
- Babata system will create a root task for each channel message.
- You cannot reply through channels; use appropriate tools (CLI or scripts) to communicate back.

## Other Notes
- Source code repo: https://github.com/systemxlabs/babata
- Do not edit `{BABATA_HOME}/config.json` or `{BABATA_HOME}/providers/*/config.json` directly, use babata CLI or HTTP APIs instead.
