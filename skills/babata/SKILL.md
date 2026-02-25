---
name: babata
description: 管理和配置 Babata CLI（providers/agents/channels/jobs/onboard/server），维护 ~/.babata/config.json，并排查服务启动、Windows Service、任务调度（schedule.kind=cron/at）与 job history 问题。用户请求这些操作时使用。
---

# Babata 智能体管理与排障

保持最小改动并确保可运行。优先使用现有 CLI 子命令，不直接手改配置文件，除非用户明确要求。

## 执行步骤

1. 确认目标：明确要改 provider/agent/channel/job/service 的哪些字段。
2. 读取现状：优先读取 `~/.babata/config.json`。
3. 执行变更：优先调用 `babata provider|agent|channel|job|server` 子命令。
4. 校验结果：确认配置约束和服务状态，再给出可复现验证命令。

## 当前 CLI 能力

- Prompt:
  - `babata --agent <agent_name> "<prompt>"`
  - prompt 必填；不支持命令行临时覆盖 provider/model。
- Onboard:
  - `babata onboard`
  - 交互式配置 provider、`main` agent、Telegram channel。
  - 在 macOS/Linux 生成服务文件；在 Windows 创建 Windows Service。
  - 配置服务后会尝试启动服务。
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
  - `babata server stop`
  - `babata server restart`
  - 隐藏子命令：`babata server windows-service-host --home-dir <HOME_DIR>`（仅 Windows service host 内部使用）

## 配置结构

- 配置路径：`~/.babata/config.json`
- 任务历史库：`~/.babata/job_history.db`
- 典型配置：

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
      "description": "每天 9 点产出日报",
      "prompt": "请总结今天进展"
    },
    {
      "name": "one-shot",
      "agent_name": "main",
      "enabled": true,
      "schedule": { "kind": "at", "at": "2026-02-26T09:00:00Z" },
      "description": "单次任务",
      "prompt": "执行一次"
    }
  ]
}
```

## 约束与语义

1. 保证存在 `name = "main"` 的 agent。
2. 保证每个 agent 的 `provider` 在 `providers` 中存在。
3. 保证 provider 类型唯一（`openai`/`moonshot`/`deepseek` 不重复）。
4. 保证 job 名称唯一，且 `job.agent_name` 指向已存在 agent。
5. 校验 `schedule`：
   - `kind=cron` 时，`expr` 必须是合法 cron 表达式。
   - `kind=at` 时，若当前时间已晚于 `at`，该任务不会执行（跳过）。
6. 校验 Telegram channel：
   - `bot_token` 必填。
   - `allowed_user_ids` 必填且必须为正整数。
   - `polling_timeout_secs` 若设置必须大于 0。

## 调度与服务行为

- job 调度仅在 `babata server serve` 运行期间生效。
- 调度器每 10 秒重载一次配置，并与运行中任务集合对比：新增则启动，删除/变化则重建。
- 每次 job 执行都会写入 sqlite history（成功和失败都记录）。
- Windows 服务实现为 SCM Windows Service（`sc create/config/start/stop`），不是 Task Scheduler，也不使用 `HKCU\\...\\Run`。
- `babata onboard` 在 Windows 需要管理员权限才能创建服务；权限不足时给出 warning 并跳过服务创建。
- Telegram channel 仅处理私聊（DM）；群组消息会被忽略。
- 模板文件仅用于 macOS/Linux：
  - `services/babata.server.plist.template`
  - `services/babata.server.service.template`

## 常用命令模板

- 新增 provider：
  - `babata provider add '{"name":"openai","api_key":"sk-..."}'`
- 新增 agent：
  - `babata agent add '{"name":"main","provider":"openai","model":"gpt-4.1"}'`
- 新增 Telegram channel：
  - `babata channel add '{"name":"telegram","bot_token":"123:abc","allowed_user_ids":[123456789]}'`
- 新增 cron 任务：
  - `babata job add '{"name":"daily","agent_name":"main","enabled":true,"schedule":{"kind":"cron","expr":"0 9 * * *"},"description":"Daily summary","prompt":"..."}'`
- 新增 at 任务：
  - `babata job add '{"name":"once","agent_name":"main","enabled":true,"schedule":{"kind":"at","at":"2026-02-26T09:00:00Z"},"description":"One shot","prompt":"..."}'`
- 查询任务历史：
  - `babata job history --name daily --limit 20`

## 结果回传

- 列出改动文件和关键字段。
- 给出校验结果（成功/失败）。
- 给出一条可直接执行的验证命令。
