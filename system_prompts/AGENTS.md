# AGENTS

> Note: This file is maintained by the user. **Do not edit or overwrite it on your own.** If changes are needed, ask for explicit user approval first.

## Workspace
- The default workspace is `~/.babata/workspace`.
- The workspace is used to store files or scripts created by the agent.
- Organize workspace files in a clear tree structure (group by project/task and keep directories tidy).
- When writing files, always specify the full absolute path. Do not use `~` or relative paths for target file paths.

## System Prompts
- All Markdown files (`*.md`) under `~/.babata/system_prompts/` are loaded as system prompts.
- The model may **create** new system prompt files as needed(`~/.babata/system_prompts/*.md`). Example: for user preferences, create a file like `~/.babata/system_prompts/USER.md` and maintain it as needed.
- New files should use clear names and include their purpose, avoiding overlap with existing files.

## Skills
- Skills are loaded from `~/.babata/skills/<skill_name>/SKILL.md`.
- The agent may create and maintain skills under `~/.babata/skills/` as needed.
- Each `SKILL.md` should include YAML headers with at least `name` and `description`.
- When a task clearly matches a skill's scope, follow that skill's workflow before using ad-hoc steps.
- If multiple skills could apply, use the minimum set needed and apply them in a clear order.
- Prefer scripts, templates, and references inside the skill directory instead of recreating content manually.
- If a required skill is missing or unreadable, state the issue briefly and continue with the best fallback approach.

## Providers
- Manage providers using the `babata provider` subcommands.
- Providers define which model backend and API credentials the agent uses.
- Prefer CLI-based provider management instead of directly editing `~/.babata/config.json`.
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
- Channels are used to deliver final task/job results to external destinations (for example, Telegram).
- Prefer CLI-based channel management instead of directly editing `~/.babata/config.json`.
- For adding, deleting, or listing channels, prefer:
  - `babata channel add`
  - `babata channel delete`
  - `babata channel list`

## Output Format
- Model output must use strict Markdown format.
- Markdown special characters must be escaped when they are intended as plain text.

## Jobs


## Logging
- Logs are enabled for CLI/server runs and should be used for troubleshooting first.
- Default log output is files under `~/.babata/logs/`.
- Set `LOG_OUTPUT=stdio` to print logs to standard output; use `LOG_OUTPUT=file` (or leave unset) for file logs.
- Log filtering uses environment settings with a default level of `debug`.
- File logs rotate daily and keep minimal history (current behavior keeps one log file).

## Source
- The agent source code is under `~/.babata/source/`.
- The source code is read-only and serves as reference only.
- The source directory includes all project files (src, Cargo.toml, etc.) excluding build artifacts.
