# Channel feedback-first stale-check design

**Date:** 2026-04-18
**Target repo:** `babata`
**Scope:** `src/channel/wechat.rs`, `src/channel/telegram.rs`

## Goal
在 channel 入站消息路由中，将判断顺序调整为：**先识别 feedback 回复，再判断普通消息是否过时**。这样即使 feedback 回复超过当前“1 小时”过时阈值，也仍然能够命中等待中的 feedback waiter；而非 feedback 的普通消息仍保持现有过时过滤规则。

## Current behavior
当前最新 `origin/main`（`c76e316`）中：
- `WechatChannel::route_incoming` 先检查 `message.timestamp < one_hour_ago`，过时则直接丢弃；之后才尝试通过 `extract_quote_hash` 命中 `feedback_waiters`。
- `TelegramChannel::route_incoming` 先检查 `message.timestamp < one_hour_ago`，过时则直接丢弃；之后才尝试通过 `reply_to_message_id` 命中 `feedback_waiters`。

这会导致一种错误行为：若用户对系统发出的 feedback 提问进行了延迟回复，只要该消息时间戳落在过时阈值之外，就会在命中 waiter 之前被丢弃，反馈交互被错误中断。

## Chosen approach
采用**最小改动顺序重排**：
1. 在 WeChat 和 Telegram 的 `route_incoming` 中，先识别 feedback 回复；
2. 若命中 waiter，立即将内容发送给 waiter，并 `continue`；
3. 若未命中 waiter，再对普通消息执行“是否过时”的过滤；
4. 其余普通消息拼装逻辑保持不变。

## Architecture and component impact
### WeChat
受影响函数：`WechatChannel::route_incoming`

调整后的处理流程：
1. 过滤非目标用户消息；
2. 先持久化 `latest_context_token`；
3. 先把消息转换为 `Content`；
4. 使用 `extract_quote_hash(&message)` 检查是否命中 `feedback_waiters`；
5. 如果命中，直接向 waiter 发送内容并结束当前消息处理；
6. 如果未命中，再检查是否过时；
7. 非过时普通消息继续进入返回内容集合。

说明：
- 保持 `latest_context_token` 的持久化时机不晚于 feedback 发送所需上下文；
- 保持现有 quoted text / image / voice / file / video 的内容转换逻辑不变；
- 仅改变路由顺序，不改变 message-to-content 的格式。

### Telegram
受影响函数：`TelegramChannel::route_incoming`

调整后的处理流程：
1. 过滤非目标用户消息；
2. 先将消息转换为 `Content`；
3. 如果没有可路由内容则跳过；
4. 使用 `reply_to_message_id` 检查是否命中 `feedback_waiters`；
5. 如果命中，直接向 waiter 发送内容并结束当前消息处理；
6. 如果未命中，再检查是否过时；
7. 非过时普通消息继续执行 quoted content 拼接，再写入返回内容集合。

说明：
- 保留当前 reply message 的 quoted content 拼接行为；
- 保留现有空内容提前跳过逻辑；
- 仅调整“waiter 命中”和“过时过滤”的先后顺序。

## Data flow
### Feedback reply
- WeChat：消息 -> `extract_quote_hash` -> `feedback_waiters.remove(hash)` -> waiter 收到 `Vec<Content>` -> 不进入普通任务创建链路。
- Telegram：消息 -> `reply_to_message_id` -> `feedback_waiters.remove(id)` -> waiter 收到 `Vec<Content>` -> 不进入普通任务创建链路。

### Non-feedback message
- 消息未命中 waiter -> 执行一小时过时过滤 -> 未过时则继续按原有格式进入上层 channel loop -> 创建普通任务。

## Error handling and compatibility
- 不改变现有错误类型和错误传播方式；
- 不放宽普通消息的过时过滤规则，仅放宽“已命中 feedback waiter 的回复”这一特例；
- `feedback_waiters.remove(...)` 保持原子消费，避免重复投递；
- 若 feedback waiter 不存在，消息按普通消息处理，并继续受过时规则约束；
- WeChat 与 Telegram 行为保持一致，减少跨 channel 语义差异。

## Testing strategy
新增或更新测试，覆盖以下行为：
1. **WeChat：过时但命中 feedback 的消息仍会被 waiter 接收**；
2. **WeChat：过时且未命中 feedback 的普通消息仍会被丢弃**；
3. **Telegram：过时但命中 feedback 的回复仍会被 waiter 接收**；
4. **Telegram：过时且未命中 feedback 的普通消息仍会被丢弃**。

测试实现原则：
- 优先在各自文件现有 `#[cfg(test)]` 模块中追加单元测试；
- 直接调用 `route_incoming`，避免依赖真实外部 channel 服务；
- 构造最小必要消息体，聚焦顺序变化而非附件下载路径；
- 对 waiter 场景同时验证：
  - 返回给普通消息链路的 `Vec<Content>` 为空；
  - waiter 确实收到预期内容；
  - 过时检查不会提前吞掉该 feedback 回复。

## Verification plan
实现完成后，验证至少包括：
- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

## Out of scope
- 不修改过时阈值（仍为 1 小时）；
- 不重构 channel 抽象层；
- 不引入跨 channel 共享辅助模块；
- 不改变 task 创建逻辑，只调整 channel 内部消息路由优先级。
