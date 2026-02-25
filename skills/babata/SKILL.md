---
name: babata
description: Manage and configure Babata CLI (providers/agents/channels/jobs/onboard/server), maintain ~/.babata/config.json, and troubleshoot service startup, Windows Service, job scheduling (schedule.kind=cron/at), and job history issues. Use when users request these operations.
---

# Babata Agent Management and Troubleshooting

Keep changes minimal and ensure everything remains runnable. Prefer existing CLI subcommands, and do not edit configuration files directly unless the user explicitly requests it.

## Workflow

1. Confirm the target: identify which fields to change for provider/agent/channel/job/service.
2. Read the current state: start with `~/.babata/config.json`.
3. Apply changes: prefer `babata provider|agent|channel|job|server` subcommands; for service operations, only use `babata server start` or `babata server restart`, and do not use `babata server stop`.
4. Validate results: confirm configuration constraints and service status, then provide a reproducible verification command.

## Current CLI Capabilities

- Prompt:
  - `babata --agent <agent_name> "<prompt>"`
  - `prompt` is required; temporary provider/model override via CLI is not supported.
- Onboard:
  - `babata onboard`
  - Interactively configures provider, `main` agent, and Telegram channel.
  - Generates service files on macOS/Linux; creates a Windows Service on Windows.
  - Attempts to start the service after service configuration.
- Provider:
  - `babata provider add <PROVIDER_CONFIG_JSON>`
  - `babata provider delete <PROVIDER_NAME>`
  - `babata provider list`
- Agent:
  - `babata agent add <AGENT_CONFIG_JSON>`
  - `babata agent delete <AGENT_NAME>`
  - `babata agent list`
- Channel:
  - `babata channel add <CHANNEL_CONFIG_JSON>`
  - `babata channel delete <CHANNEL_NAME>`
  - `babata channel list`
- Job:
  - `babata job add <JOB_CONFIG_JSON>`
  - `babata job delete <JOB_NAME>`
  - `babata job list`
  - `babata job history [--name <JOB_NAME>] [--limit <N>]`
- Server:
  - `babata server serve`
  - `babata server start`
  - `babata server restart`
  - Do not use `babata server stop` when performing service operations in this skill.
  - Hidden subcommand: `babata server windows-service-host --home-dir <HOME_DIR>` (for internal Windows service host use only)

## Configuration Structure

- Config path: `~/.babata/config.json`
- Job history database: `~/.babata/job_history.db`
- Example configuration:

```json
{
  "providers": [
    { "name": "openai", "api_key": "sk-..." },
    { "name": "moonshot", "api_key": "sk-..." },
    { "name": "deepseek", "api_key": "sk-..." }
  ],
  "agents": [
    { "name": "main", "provider": "openai", "model": "gpt-4.1" }
  ],
  "channels": [
    {
      "name": "telegram",
      "bot_token": "123456:ABC",
      "allowed_user_ids": [123456789],
      "base_url": "https://api.telegram.org",
      "polling_timeout_secs": 30
    }
  ],
  "jobs": [
    {
      "name": "daily-summary",
      "agent_name": "main",
      "enabled": true,
      "schedule": { "kind": "cron", "expr": "0 9 * * *", "tz": null },
      "description": "Generate a daily summary at 9 AM",
      "prompt": "Please summarize today's progress"
    },
    {
      "name": "one-shot",
      "agent_name": "main",
      "enabled": true,
      "schedule": { "kind": "at", "at": "2026-02-26T09:00:00Z" },
      "description": "One-time task",
      "prompt": "Run once"
    }
  ]
}
```

## Constraints and Semantics

1. Ensure an agent with `name = "main"` exists.
2. Ensure each agent's `provider` exists in `providers`.
3. Ensure provider types are unique (`openai`/`moonshot`/`deepseek` cannot be duplicated).
4. Ensure job names are unique, and each `job.agent_name` points to an existing agent.
5. Validate `schedule`:
   - For `kind=cron`, `expr` must be a valid cron expression.
   - For `kind=at`, if current time is later than `at`, the task will not run (skipped).
6. Validate Telegram channel:
   - `bot_token` is required.
   - `allowed_user_ids` is required and must contain positive integers.
   - If set, `polling_timeout_secs` must be greater than 0.

## Scheduling and Service Behavior

- Job scheduling is active only while `babata server serve` is running.
- The scheduler reloads configuration every 10 seconds and compares against running jobs: new jobs are started, deleted/changed jobs are rebuilt.
- Every job execution is recorded in sqlite history (both success and failure are recorded).
- Windows service is implemented as an SCM Windows Service (`sc create/config/start/stop`), not Task Scheduler, and does not use `HKCU\\...\\Run`.
- `babata onboard` requires Administrator privileges on Windows to create the service; if insufficient, it warns and skips service creation.
- Telegram channel handles only direct messages (DM); group messages are ignored.
- Template files are only for macOS/Linux:
  - `services/babata.server.plist.template`
  - `services/babata.server.service.template`

## Common Command Templates

- Add provider:
  - `babata provider add '{"name":"openai","api_key":"sk-..."}'`
- Add agent:
  - `babata agent add '{"name":"main","provider":"openai","model":"gpt-4.1"}'`
- Add Telegram channel:
  - `babata channel add '{"name":"telegram","bot_token":"123:abc","allowed_user_ids":[123456789]}'`
- Add cron job:
  - `babata job add '{"name":"daily","agent_name":"main","enabled":true,"schedule":{"kind":"cron","expr":"0 9 * * *"},"description":"Daily summary","prompt":"..."}'`
- Add at job:
  - `babata job add '{"name":"once","agent_name":"main","enabled":true,"schedule":{"kind":"at","at":"2026-02-26T09:00:00Z"},"description":"One shot","prompt":"..."}'`
- Query job history:
  - `babata job history --name daily --limit 20`

## Result Reporting

- List changed files and key fields.
- Provide validation results (success/failure).
- Provide one directly executable verification command.
