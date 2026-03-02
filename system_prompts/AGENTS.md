# AGENTS

> Note: This file is maintained by the user. **Do not edit or overwrite it on your own.** If changes are needed, ask for explicit user approval first.

## Workspace
- The default workspace is `~/.babata/workspace`.
- The workspace is used to store files or scripts created by the agent.
- When writing files, always specify the full absolute path. Do not use `~` or relative paths for target file paths.

## System Prompts
- All Markdown files (`*.md`) under `~/.babata/system_prompts/` are loaded as system prompts.
- The model may **create** new system prompt files as needed(`~/.babata/system_prompts/*.md`). Example: for user preferences, create a file like `~/.babata/system_prompts/USER.md` and maintain it as needed.
- New files should use clear names and include their purpose, avoiding overlap with existing files.

## Skills
- Skills are loaded from `~/.babata/skills/<skill_name>/SKILL.md`.
- Each `SKILL.md` should include YAML headers with at least `name` and `description`.
- When a task clearly matches a skill's scope, follow that skill's workflow before using ad-hoc steps.
- If multiple skills could apply, use the minimum set needed and apply them in a clear order.
- Prefer scripts, templates, and references inside the skill directory instead of recreating content manually.
- If a required skill is missing or unreadable, state the issue briefly and continue with the best fallback approach.

## Channels
- Manage channels using the `babata channel` subcommands.
- Users communicate with the system through configured channels.
- Channels are used to deliver final task/job results to external destinations (for example, Telegram).
- Prefer CLI-based channel management instead of directly editing `~/.babata/config.json`.
- For adding, deleting, or listing channels, prefer:
  - `babata channel add`
  - `babata channel delete`
  - `babata channel list`

## Logging
- Logs are enabled for CLI/server runs and should be used for troubleshooting first.
- Default log output is files under `~/.babata/logs/`.
- Set `LOG_OUTPUT=stdio` to print logs to standard output; use `LOG_OUTPUT=file` (or leave unset) for file logs.
- Log filtering uses environment settings with a default level of `debug`.
- File logs rotate daily and keep minimal history (current behavior keeps one log file).

## Source
- The project source code is embedded in the binary and written to `~/.babata/source/` during `babata onboard`.
- The source code is read-only and serves as reference only.
- The source directory includes all project files (src, Cargo.toml, etc.) excluding build artifacts.

## Jobs
- Manage jobs using the `babata job` subcommands.
- Scheduled jobs are recurring tasks (for example, cron-based jobs) that run repeatedly according to their schedule.
- One-shot jobs are single-run tasks (for example, `at`-style jobs) that run once at a specified time and do not repeat.
- A job's `prompt` is the task instruction for the model, describing what to do and what to output.
- After a scheduled job runs, the final result of that run is automatically sent to all configured `channel`s.
- On success, send the final output; on failure, send the final error message.
- The model does not need to call tools to send messages during the task; `babata` automatically broadcasts the final result to all configured `channel`s.
- Do not directly modify system schedulers (such as `crontab`, `launchd`, or `systemd timer`) as a replacement for `babata job`.
- For adding, updating, deleting, or checking history, prefer:
  - `babata job add`
  - `babata job delete`
  - `babata job list`
  - `babata job history`
