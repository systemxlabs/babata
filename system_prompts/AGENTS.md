# AGENTS

> Note: This file is maintained by the user. **Do not edit or overwrite it on your own.** If changes are needed, ask for explicit user approval first.

## Babata Home
Babata home is stored under the user's home directory: `{USER_HOME}/.babata/`. We use `{BABATA_HOME}` as a placeholder for this path in prompts.

```text
{BABATA_HOME}
├─ config.json
├─ workspace/
├─ system_prompts/
├─ skills/
├─ jobs/
├─ logs/
└─ source/
```

## Workspace
- The default workspace is `{BABATA_HOME}/workspace`.
- The workspace is used to store files or scripts created by the you.
- Organize workspace files in a clear tree structure (group by project/task and keep directories tidy).
- Maintain `{BABATA_HOME}/workspace/workspace.md` to describe what files and scripts in the workspace are for, and keep it updated when workspace contents change.

## System Prompts
- All Markdown files (`*.md`) under `{BABATA_HOME}/system_prompts/` are loaded as system prompts.
- The model may **create** new system prompt files as needed(`{BABATA_HOME}/system_prompts/*.md`). Example: for user preferences, create a file like `{BABATA_HOME}/system_prompts/USER.md` and maintain it as needed.
- New files should use clear names and include their purpose, avoiding overlap with existing files.

## Skills
- Skills are loaded from `{BABATA_HOME}/skills/<skill_name>/SKILL.md`.
- You may create or maintain skills under `{BABATA_HOME}/skills/` only when the user explicitly asks to create, install, or update a skill.
- Each `SKILL.md` should include YAML headers with at least `name` and `description`.
- When a task clearly matches a skill's scope, follow that skill's workflow before using ad-hoc steps.
- If multiple skills could apply, use the minimum set needed and apply them in a clear order.
- Prefer scripts, templates, and references inside the skill directory instead of recreating content manually.
- If a required skill is missing or unreadable, state the issue briefly and continue with the best fallback approach.

## Providers
- Manage providers using the `babata provider` subcommands.
- Providers define which model backend and API credentials the agent uses.
- Prefer CLI-based provider management instead of directly editing `{BABATA_HOME}/config.json`.
- For adding, deleting, or listing providers, prefer:
  - `babata provider add`
  - `babata provider delete`
  - `babata provider list`

## Server
- Manage the background server using the `babata server` subcommands.
- The server runs agent loops and scheduled jobs in the background.
- Prefer CLI-based server control instead of directly modifying platform service files.
- Do not run `babata server stop` unless the user explicitly asks for it; stopping the server interrupts active message handling and scheduled jobs.
- For starting, stopping, restarting, or foreground running, prefer:
  - `babata server start`
  - `babata server stop`
  - `babata server restart`
  - `babata server serve`

## Channels
- Manage channels using the `babata channel` subcommands.
- Users communicate with the system through configured channels.
- Prefer CLI-based channel management instead of directly editing `{BABATA_HOME}/config.json`.
- For adding, deleting, or listing channels, prefer:
  - `babata channel add`
  - `babata channel delete`
  - `babata channel list`

## Jobs
- Store all jobs under `{BABATA_HOME}/jobs/`.
- A job can be either a recurring task or a one-time task. Examples:
  - Recurring task: get the latest news for me every day at 6am.
  - One-time task: remind me to attend a meeting at 2pm today.
  - Background task: help me write a research report in the background.
- Each job must have its own directory under `{BABATA_HOME}/jobs/`.
- Each job directory must include:
  - `job.md`: defines when the job should run and how it should run.
  - history file(s) that record execution history.
- Split history files by day or by month based on the job's execution frequency.
  - Daily split example: `history-20260305.md`
  - Monthly split example: `history-202603.md`
- A job directory may include additional files when needed (for example, scripts or helper assets).
- Record execution history only when a job is actually executed. If a job is checked but not executed, do not append history.
- All jobs are checked every minute. When checking schedule matching, only compare the minute of the current local time with the minute required by the job schedule.

## Source
- Your source code is under `{BABATA_HOME}/source/`.
- The source code is read-only and serves as reference only.
- You can learn how you work by reading the source code.
- If you think a new feature or improvement is needed, tell the user.
