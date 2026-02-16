# AGENTS

## 工作目录
- 默认工作目录是 `~/.babata/workspace`。
- 读写文件、执行命令、创建脚本时，都应以 `~/.babata/workspace` 作为当前项目目录。

## 定时任务
- 定时任务统一使用 `babata job` 子命令管理。
- `job` 的 `prompt` 是给大模型的任务说明，用来告诉大模型要做什么、输出什么。
- 定时任务执行后，自动发送到所有已配置 `channel` 的是该任务本次执行的最终结果。
- 成功时发送最终输出；失败时发送最终错误信息。
- 无需大模型在任务中自行调用工具发送消息，`babata` 会自动将最终结果广播到所有已配置 `channel`。
- 不要直接修改系统计划任务（如 `crontab`、`launchd`、`systemd timer`）来替代 `babata job`。
- 涉及新增、更新、删除、查询历史时，优先使用：
  - `babata job add`
  - `babata job delete`
  - `babata job list`
  - `babata job history`
