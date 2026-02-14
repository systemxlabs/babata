---
name: babata
description: 管理和配置 Babata 智能体（providers/agents/channels/jobs/onboard/server），维护 ~/.babata 配置并排查运行问题。
inline: true
---

# Babata 智能体管理与配置

用于管理 Babata 的本地配置、运行入口与服务文件，目标是**最小改动且保证可运行**。

## 何时使用

当用户需要以下操作时使用：

- 初始化或重建本地运行环境（`babata onboard`）
- 管理 provider（新增、删除、列出）
- 管理 job（新增、删除、列出）
- 配置/排查 Telegram DM channel
- 调整 agent/provider/model 绑定关系
- 启动/重启 server 服务，排查启动失败

## 当前能力（基于代码）

- 默认 prompt 调用：
  - `babata --agent <agent_name> "<prompt>"`
  - 不支持 `--provider` / `--model` 临时覆盖
- Onboard：
  - `babata onboard`
  - 交互式配置 provider、`main` agent、channel
  - 生成服务文件（模板替换 `{{HOME_DIR}}`）
- Provider CLI：
  - `babata provider add <PROVIDER_CONFIG_JSON>`
  - `babata provider delete <PROVIDER_NAME>`
  - `babata provider list`
- Agent CLI：
  - `babata agent add <AGENT_CONFIG_JSON>`
  - `babata agent delete <AGENT_NAME>`
  - `babata agent list`
- Job CLI：
  - `babata job add <JOB_CONFIG_JSON>`
  - `babata job delete <JOB_NAME>`
  - `babata job list`
- Server CLI：
  - `babata server serve`
  - `babata server start`
  - `babata server restart`
- 支持 provider：`openai`、`moonshot`
- 支持 channel：`telegram`（仅私聊 DM，不支持群组）

## 配置文件与结构

- 配置路径：`~/.babata/config.json`
- 日志路径：`~/.babata/logs`
- 典型配置：

```json
{
  "providers": [
    { "name": "openai", "api_key": "sk-..." },
    { "name": "moonshot", "api_key": "sk-..." }
  ],
  "agents": [
    { "name": "main", "provider": "openai", "model": "gpt-4.1" }
  ],
  "channels": [
    {
      "type": "telegram",
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
      "cron": "0 9 * * *",
      "description": "每天 9 点产出日报",
      "prompt": "请总结今天进展"
    }
  ]
}
```

## 硬性约束

1. 必须存在 `name = "main"` 的 agent
2. 每个 agent 的 `provider` 必须在 `providers` 中存在
3. `providers` 不能出现重复 provider 类型
4. 每个 job 的 `name` 必须唯一
5. 每个 job 的 `agent_name` 必须指向已存在 agent
6. job 的 `cron` 必须是合法表达式（`croner` 校验）
7. Telegram channel:
   - `bot_token` 必填
   - `allowed_user_ids` 必填，且必须是正整数
   - `polling_timeout_secs` 必须大于 0（若设置）

## 推荐工作流

1. **确认目标状态**
   - 明确用户要改 provider、agent、channel、job 的哪些字段。
2. **读取现状**
   - 读取 `~/.babata/config.json`；不存在时优先执行 `babata onboard`。
3. **优先走 CLI 子命令**
   - provider/agent/job 修改优先使用对应子命令。
4. **最小化修改**
   - 只改用户要求字段；不要顺手重排无关内容。
5. **一致性校验**
   - 确保满足上面的硬性约束。
6. **冒烟验证**
   - 交互调用：`babata --agent main "hello"`
   - 服务命令：`babata server start` 或 `babata server restart`

## 常用命令模板

- 新增 OpenAI provider：
  - `babata provider add '{"name":"openai","api_key":"sk-..."}'`
- 新增 Moonshot provider：
  - `babata provider add '{"name":"moonshot","api_key":"sk-..."}'`
- 列出 provider：
  - `babata provider list`
- 删除 provider：
  - `babata provider delete "openai"`
- 新增 agent：
  - `babata agent add '{"name":"main","provider":"openai","model":"gpt-4.1"}'`
- 删除 agent：
  - `babata agent delete "main"`
- 列出 agent：
  - `babata agent list`
- 新增 job：
  - `babata job add '{"name":"daily","agent_name":"main","enabled":true,"cron":"0 9 * * *","description":"Daily summary","prompt":"..."}'`
- 列出 job：
  - `babata job list`
- 删除 job：
  - `babata job delete "daily"`

## 服务文件与平台行为

- 模板位置（仓库内）：
  - `services/babata.server.plist.template`
  - `services/babata.server.service.template`
- `babata onboard` 会渲染模板并替换 `{{HOME_DIR}}`：
  - macOS 输出到：`~/Library/LaunchAgents/babata.server.plist`
  - Linux 输出到：`~/.babata/services/babata.server.service`
- 默认平台选择：
  - macOS 用 `.plist`
  - Linux 用 `.service`

## 已知边界

- `jobs` 目前提供配置管理能力（add/list/delete + 校验）；是否执行由运行时实现决定。
- channel 目前仅实现 Telegram DM 流程；群聊消息会被忽略。

## 结果回传要求

完成任务后输出：

- 变更的文件和关键字段
- 校验结果（通过/失败）
- 可直接执行的一条验证命令
