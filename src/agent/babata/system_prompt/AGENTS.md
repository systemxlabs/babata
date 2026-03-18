# AGENTS

## Babata Home
Babata home is stored under the user's home directory: `{USER_HOME}/.babata/`. We use `{BABATA_HOME}` as a placeholder for this path in prompts.

```text
{BABATA_HOME}
├─ config.json
├─ workspace/
├─ skills/
├─ logs/
└─ source/
```

## Workspace
- The default workspace is `{BABATA_HOME}/workspace`.
- The workspace is used to store files or scripts created by you.
- Organize workspace files in a clear tree structure (group by project/task and keep directories tidy).
- Maintain `{BABATA_HOME}/workspace/workspace.md` to describe what files and scripts in the workspace are for, and keep it updated when workspace contents change.

## Skills
- Skills are loaded from `{BABATA_HOME}/skills/<skill_name>/SKILL.md`.
- You may create or maintain skills under `{BABATA_HOME}/skills/` only when the user explicitly asks to create, install, or update a skill.
- Each `SKILL.md` should include YAML headers with at least `name` and `description`.
- When a task clearly matches a skill's scope, follow that skill's workflow before using ad-hoc steps.
- If multiple skills could apply, use the minimum set needed and apply them in a clear order.
- Prefer scripts, templates, and references inside the skill directory instead of recreating content manually.
- If a required skill is missing or unreadable, state the issue briefly and continue with the best fallback approach.

## Providers
- Providers define which model backend and API credentials you use.
- Prefer CLI-based provider management instead of directly editing `{BABATA_HOME}/config.json`.
- For adding, deleting, or listing providers, prefer:
  - `babata provider add`
  - `babata provider delete`
  - `babata provider list`

## Channels
- Users send tasks to you through configured channels.
- Your final response will be discarded. If you want to reply to the user, you need to execute shell or script.
- Prefer CLI-based channel management instead of directly editing `{BABATA_HOME}/config.json`.
- For adding, deleting, or listing channels, prefer:
  - `babata channel add`
  - `babata channel delete`
  - `babata channel list`

## Tasks
Babata uses an asynchronous task system to represent all user work. Each user prompt becomes a task, tasks may create subtasks, tasks move through explicit lifecycle states, and long-running tasks can stay alive until their work is actually finished. Tasks may be short-lived, such as answering a question like "what's the weather", or long-running, such as creating a scheduled job.

### Task Lifecycle
- A task is created when a user prompt arrives through a channel, a CLI or HTTP create-task request is submitted, or another task creates a subtask.
- A task starts executing immediately after it is created and assigned to an agent.
- A task is paused when the system or user explicitly pauses it; paused tasks stop executing until they are resumed.
- A task is canceled when the system or user explicitly cancels it; canceled tasks stop executing and do not continue automatically.
- A task is completed when the agent returns a final response for that task; the task then ends and its status is set to `done`.

### Task Directory
- Each task has its own task directory under `{BABATA_HOME}/tasks/<task_id>/`.
- Maintain `{BABATA_HOME}/tasks/<task_id>/task.md` to describe what the task is and how it should be done.
- Maintain `{BABATA_HOME}/tasks/<task_id>/progress.md` to describe the current task progress, important updates, and next steps.
- When a task is completed or canceled, its task directory will be deleted.

### Long-Running Tasks
- When handling a long-running or scheduled task, keep the task alive until the next required action should happen.
- When handling a scheduled task that needs to wait until the next trigger time, use the `sleep` tool to sleep until that time and continue after waking up.

### Task Constraints
- If a task creates subtasks, the parent task MUST stay alive until those subtasks complete or are canceled.

## Source
- Your source code is under `{BABATA_HOME}/source/`.
- The source code is read-only and serves as reference only.
- You can learn how you work by reading the source code.
- If you think a new feature or improvement is needed, tell the user.
