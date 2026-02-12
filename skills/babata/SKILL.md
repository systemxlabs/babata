---
name: babata
description: Manage and configure Babata agents, providers, models, and local runtime files (~/.babata), including onboarding, config edits, validation, and troubleshooting.
inline: true
---

# Babata Agent 管理与配置

用于管理 Babata 的本地配置与运行入口，目标是**安全修改配置并保证可运行**。

## 何时使用

当用户提出以下需求时使用本技能：

- 初始化 Babata（首次可用）
- 新增/修改 agent、provider、model
- 修复 `~/.babata/config.json` 配置错误
- 调整 system prompts / skills 的加载内容
- 排查 CLI 运行失败（配置缺失、provider 不存在、agent 不存在）

## 关键事实（基于当前代码）

- 配置文件路径：`~/.babata/config.json`
- 首次初始化命令：`babata onboard`（已实现）
- Provider 管理子命令 `babata provider add/delete/list` 当前**未实现**
- prompt 入口：`babata --agent <agent_name> "<prompt>"`
  - 仅支持 `--agent` 与位置参数 `prompt`
  - 不支持 `--provider` / `--model` 覆盖
- 当前支持 provider：`openai`、`moonshot`

## 配置结构与约束

`config.json` 必须满足：

```json
{
  "providers": {
    "openai": { "api_key": "..." }
  },
  "agents": {
    "main": {
      "provider": "openai",
      "model": "gpt-4.1"
    }
  }
}
```

硬性约束：

1. 必须存在 `agents.main`
2. 每个 `agents.<name>.provider` 必须在 `providers` 中存在同名项

## 标准工作流

1. **确认目标状态**
   - 明确用户想改哪些字段：agent 名、provider 名、model、API key。
2. **读取现状**
   - 读取 `~/.babata/config.json`；若不存在，优先建议执行 `babata onboard`。
3. **最小化修改**
   - 只改用户要求的字段；不要重排或改动无关配置。
4. **一致性校验**
   - 检查上述两条硬性约束是否满足。
5. **冒烟验证**
   - 运行一次最小调用验证，例如：
     - `babata --agent main "hello"`
6. **结果回传**
   - 明确说明改了哪些键、是否通过验证、下一步如何继续。

## 常见任务模板

### 1) 新增 provider

- 在 `providers` 下新增一项（key 建议使用小写，如 `openai`）。
- 再将目标 agent 的 `provider` 指向该 key。

### 2) 切换 main agent 的模型

- 修改 `agents.main.model`
- 不改 `providers` 时，确认 `agents.main.provider` 仍存在

### 3) 新增自定义 agent（如 `coder`）

- 在 `agents` 下新增：
  - `provider`：必须引用已存在 provider
  - `model`：目标模型名
- 不要删除 `agents.main`

## 故障排查速查

- `No 'main' agent defined in configuration`
  - 解决：补回 `agents.main`
- `Agent 'X' references unknown provider 'Y'`
  - 解决：新增 `providers.Y` 或修正 agent 的 provider 字段
- `Agent 'X' not found in config; run "babata onboard" first`
  - 解决：创建该 agent 或改用已存在 agent
- `Unsupported provider 'X'`
  - 解决：当前仅支持 `openai` / `moonshot`

## system_prompts 与 skills 目录

`babata onboard` 会确保存在：

- `~/.babata/system_prompts`
- `~/.babata/skills`

若项目根目录下存在同名目录，会在首次初始化时复制进去。

## 输出要求

完成配置任务后，输出应包含：

- 实际变更的文件与关键字段
- 约束校验结果
- 一条可直接执行的验证命令
