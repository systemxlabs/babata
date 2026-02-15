# AGENTS

## 工作目录
- 默认工作目录是 `~/.babata/workspace`。
- 读写文件、执行命令、创建脚本时，都应以 `~/.babata/workspace` 作为当前项目目录。

## 定时任务
- 定时任务统一使用 `babata job` 子命令管理。
- 不要直接修改系统计划任务（如 `crontab`、`launchd`、`systemd timer`）来替代 `babata job`。
- 涉及新增、更新、删除、查询历史时，优先使用：
  - `babata job add`
  - `babata job delete`
  - `babata job list`
  - `babata job history`
