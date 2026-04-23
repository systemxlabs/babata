import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode, type UIEvent } from "react"
import { ChevronDown, ChevronUp, LoaderCircle, MessageSquare, RefreshCw, Wrench } from "lucide-react"

import { getTaskMessages } from "@/api"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import type { MessageContentPart, MessageRecord, MessageType, ToolCall } from "@/types"
import { MESSAGE_TYPE_OPTIONS } from "@/types"

const PAGE_SIZE = 50
const LOAD_MORE_THRESHOLD = 120

interface TaskMessagesTabProps {
  taskId: string
}

const MESSAGE_TYPE_LABELS: Record<MessageType, string> = {
  user_prompt: "用户输入",
  user_steering: "Steer 消息",
  assistant_response: "助手回复",
  assistant_tool_calls: "工具调用",
  assistant_thinking: "思考过程",
  tool_result: "工具结果",
}

const DEFAULT_MESSAGE_STYLE = "border-slate-500/20 bg-slate-500/10 text-slate-700 dark:text-slate-300"

const MESSAGE_TYPE_STYLES: Record<MessageType, string> = {
  user_prompt: "border-sky-500/20 bg-sky-500/10 text-sky-700 dark:text-sky-300",
  user_steering: "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  assistant_response: "border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  assistant_tool_calls: "border-violet-500/20 bg-violet-500/10 text-violet-700 dark:text-violet-300",
  assistant_thinking: "border-orange-500/20 bg-orange-500/10 text-orange-700 dark:text-orange-300",
  tool_result: DEFAULT_MESSAGE_STYLE,
}

function getMessageTypeLabel(type: MessageType): string {
  // Fallback for runtime type mismatch or future message types
  return MESSAGE_TYPE_LABELS[type] ?? type
}

function getMessageTypeStyle(type: MessageType): string {
  // Fallback for runtime type mismatch or future message types
  return MESSAGE_TYPE_STYLES[type] ?? DEFAULT_MESSAGE_STYLE
}

function formatTime(timestamp: string) {
  return new Date(timestamp).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

function TruncatedPre({ children, className = "" }: { children: ReactNode; className?: string }) {
  const contentRef = useRef<HTMLPreElement>(null)
  const [isExpanded, setIsExpanded] = useState(false)
  const [shouldShowButton, setShouldShowButton] = useState(false)

  useEffect(() => {
    if (contentRef.current) {
      const hasOverflow = contentRef.current.scrollHeight > 240
      setShouldShowButton(hasOverflow)
    }
  }, [children])

  return (
    <div className="relative">
      <pre
        ref={contentRef}
        className={`overflow-x-auto whitespace-pre-wrap break-words rounded-[1rem] bg-card/80 px-4 py-3 font-mono text-[13px] leading-6 text-foreground transition-all ${
          !isExpanded ? "max-h-[240px] overflow-hidden" : ""
        } ${className}`}
      >
        {children}
      </pre>

      {shouldShowButton && !isExpanded && (
        <div className="absolute bottom-0 left-0 right-0 flex items-end justify-center">
          <div className="h-12 w-full bg-gradient-to-t from-background/90 via-background/60 to-transparent rounded-b-[1rem]" />
          <Button
            variant="ghost"
            size="sm"
            className="absolute bottom-2 h-7 px-3 text-xs bg-background/80 backdrop-blur-sm border border-border/50 rounded-full hover:bg-background"
            onClick={() => setIsExpanded(true)}
          >
            <ChevronDown className="mr-1 size-3" />
            展开
          </Button>
        </div>
      )}

      {isExpanded && shouldShowButton && (
        <div className="flex justify-center pt-2">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-3 text-xs rounded-full hover:bg-background/50"
            onClick={() => setIsExpanded(false)}
          >
            <ChevronUp className="mr-1 size-3" />
            收起
          </Button>
        </div>
      )}
    </div>
  )
}

function renderContentPart(part: MessageContentPart, index: number) {
  switch (part.type) {
    case "text":
      return (
        <TruncatedPre
          key={`text-${index}`}
          className="overflow-x-auto whitespace-pre-wrap break-words rounded-[1rem] bg-card/80 px-4 py-3 font-mono text-[13px] leading-6 text-foreground"
        >
          <code>{part.text}</code>
        </TruncatedPre>
      )
    case "image_url":
      return (
        <div key={`image-url-${index}`} className="rounded-[1rem] bg-card/80 px-4 py-3 text-sm text-foreground">
          <span className="font-medium">Image URL:</span> {part.url}
        </div>
      )
    case "image_data":
      return (
        <div key={`image-data-${index}`} className="rounded-[1rem] bg-card/80 px-4 py-3 text-sm text-foreground">
          <span className="font-medium">Embedded Image:</span> {part.media_type}
        </div>
      )
    case "audio_data":
      return (
        <div key={`audio-data-${index}`} className="rounded-[1rem] bg-card/80 px-4 py-3 text-sm text-foreground">
          <span className="font-medium">Embedded Audio:</span> {part.media_type}
        </div>
      )
  }
}

function renderToolCall(call: ToolCall, index: number) {
  return (
    <div key={`${call.call_id}-${index}`} className="space-y-2 rounded-[1rem] bg-card/80 px-4 py-3">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="outline" className="rounded-full">
          <Wrench className="mr-1 size-3" />
          {call.tool_name}
        </Badge>
        <span className="text-xs text-muted-foreground">{call.call_id}</span>
      </div>
      <TruncatedPre className="overflow-x-auto whitespace-pre-wrap break-words font-mono text-[13px] leading-6 text-foreground">
        <code>{call.args}</code>
      </TruncatedPre>
    </div>
  )
}

function renderToolResult(message: MessageRecord) {
  const call = message.tool_calls?.[0] ?? null

  return (
    <div className="space-y-2">
      <div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
        Result
      </div>
      <div className="space-y-2 rounded-[1rem] bg-card/80 px-4 py-3">
        {call ? (
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="rounded-full">
              <Wrench className="mr-1 size-3" />
              {call.tool_name}
            </Badge>
            <span className="text-xs text-muted-foreground">{call.call_id}</span>
          </div>
        ) : null}
        <TruncatedPre className="overflow-x-auto whitespace-pre-wrap break-words font-mono text-[13px] leading-6 text-foreground">
          <code>{message.result}</code>
        </TruncatedPre>
      </div>
    </div>
  )
}

function MessageCard({ message, index }: { message: MessageRecord; index: number }) {
  return (
    <div className="space-y-4 rounded-[1.2rem] border border-border/70 bg-background/70 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          <div className="text-xs font-medium text-muted-foreground">#{index + 1}</div>
          <Badge
            variant="outline"
            className={`rounded-full ${getMessageTypeStyle(message.message_type)}`}
          >
            {getMessageTypeLabel(message.message_type)}
          </Badge>
        </div>
        <div className="text-xs text-muted-foreground">{formatTime(message.created_at)}</div>
      </div>

      {message.content && message.content.length > 0 ? (
        <div className="space-y-2">
          <div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
            Content
          </div>
          <div className="space-y-2">
            {message.content.map((part, contentIndex) => renderContentPart(part, contentIndex))}
          </div>
        </div>
      ) : null}

      {message.tool_calls && message.tool_calls.length > 0 ? (
        <div className="space-y-2">
          <div className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
            Tool Calls
          </div>
          <div className="space-y-2">
            {message.tool_calls.map((call, toolIndex) => renderToolCall(call, toolIndex))}
          </div>
        </div>
      ) : null}

      {message.result ? renderToolResult(message) : null}
    </div>
  )
}

export function TaskMessagesTab({ taskId }: TaskMessagesTabProps) {
  const [messages, setMessages] = useState<MessageRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [hasMore, setHasMore] = useState(true)
  const [messageTypeFilter, setMessageTypeFilter] = useState<MessageType | 'all'>('all')

  const loadMessages = useCallback(async (offset: number, reset: boolean) => {
    if (reset) {
      setLoading(true)
      setError(null)
    } else {
      setLoadingMore(true)
    }

    try {
      const nextMessages = await getTaskMessages(
        taskId,
        PAGE_SIZE,
        offset,
        messageTypeFilter
      )

      setMessages((current) => (reset ? nextMessages : [...current, ...nextMessages]))
      setHasMore(nextMessages.length === PAGE_SIZE)
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : "加载任务消息失败")
    } finally {
      setLoading(false)
      setLoadingMore(false)
    }
  }, [taskId, messageTypeFilter])

  useEffect(() => {
    setMessages([])
    setHasMore(true)
    setError(null)
    void loadMessages(0, true)
  }, [loadMessages, taskId, messageTypeFilter])

  const handleScroll = useCallback((event: UIEvent<HTMLDivElement>) => {
    const target = event.currentTarget
    const remaining = target.scrollHeight - target.scrollTop - target.clientHeight

    if (remaining < LOAD_MORE_THRESHOLD && hasMore && !loadingMore && !loading) {
      void loadMessages(messages.length, false)
    }
  }, [hasMore, loadMessages, loading, loadingMore, messages.length])

  const content = useMemo(() => {
    if (loading) {
      return (
        <div className="flex min-h-[320px] items-center justify-center">
          <LoaderCircle className="size-5 animate-spin text-muted-foreground" />
        </div>
      )
    }

    if (error && messages.length === 0) {
      return (
        <div className="flex min-h-[320px] flex-col items-center justify-center rounded-[1.4rem] border border-destructive/20 bg-destructive/5 px-4 text-center">
          <div className="text-base font-semibold tracking-tight text-destructive">消息加载失败</div>
          <p className="mt-2 text-sm leading-6 text-destructive/80">{error}</p>
        </div>
      )
    }

    if (messages.length === 0) {
      return (
        <div className="flex min-h-[320px] flex-col items-center justify-center rounded-[1.4rem] border border-dashed border-border/70 bg-card/50 px-4 text-center">
          <div className="text-base font-semibold tracking-tight text-foreground">暂无消息</div>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            当前任务还没有记录可展示的会话消息。
          </p>
        </div>
      )
    }

    return (
      <div
        className="h-[420px] overflow-y-auto rounded-[1.4rem] border border-border/70 bg-card/60"
        onScroll={handleScroll}
      >
        <div className="space-y-3 p-4">
          {messages.map((message, index) => (
            <MessageCard key={`${message.created_at}-${index}`} message={message} index={index} />
          ))}

          {loadingMore ? (
            <div className="flex items-center justify-center py-3 text-sm text-muted-foreground">
              <LoaderCircle className="mr-2 size-4 animate-spin" />
              加载更多消息...
            </div>
          ) : null}

          {!hasMore && messages.length > 0 ? (
            <div className="py-3 text-center text-sm text-muted-foreground">
              消息已全部加载
            </div>
          ) : null}
        </div>
      </div>
    )
  }, [error, handleScroll, hasMore, loading, loadingMore, messages])

  return (
    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
      <CardContent className="space-y-4 p-5">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
              <MessageSquare className="size-3.5" />
              Task Messages
            </div>
            <div className="flex items-center gap-2">
              <div className="text-lg font-semibold tracking-tight text-foreground">
                任务会话消息
              </div>
              <Badge variant="outline" className="rounded-full px-3 py-1">
                已加载 {messages.length} 条
              </Badge>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Select
              value={messageTypeFilter}
              onValueChange={(value) => setMessageTypeFilter(value as MessageType | 'all')}
            >
              <SelectTrigger className="h-8 w-[160px] rounded-full text-xs">
                <SelectValue placeholder="消息类型" />
              </SelectTrigger>
              <SelectContent>
                {MESSAGE_TYPE_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              variant="outline"
              size="sm"
              className="rounded-full"
              onClick={() => void loadMessages(0, true)}
              disabled={loading || loadingMore}
            >
              <RefreshCw className={`mr-1.5 size-3.5 ${loading ? "animate-spin" : ""}`} />
              刷新
            </Button>
          </div>
        </div>

        {error && messages.length > 0 ? (
          <div className="rounded-[1.2rem] border border-destructive/20 bg-destructive/5 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        ) : null}

        {content}
      </CardContent>
    </Card>
  )
}
