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
- Providers define which model backend and API credentials you uses.
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

## Source
- Your source code is under `{BABATA_HOME}/source/`.
- The source code is read-only and serves as reference only.
- You can learn how you work by reading the source code.
- If you think a new feature or improvement is needed, tell the user.
