import { useCallback, useEffect, useMemo, useState, type UIEvent } from "react"
import { Copy, FileText, LoaderCircle, RefreshCw } from "lucide-react"

import { getTaskLogs } from "@/api"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"

const PAGE_SIZE = 100
const LOAD_MORE_THRESHOLD = 120

interface TaskLogsTabProps {
  taskId: string
}

function getLogTone(log: string) {
  const upperLog = log.toUpperCase()

  if (upperLog.includes("[ERROR]") || upperLog.includes(" ERROR ")) {
    return {
      level: "ERROR",
      className: "border-rose-500/20 bg-rose-500/10 text-rose-700 dark:text-rose-300",
    }
  }

  if (upperLog.includes("[WARN]") || upperLog.includes(" WARNING ") || upperLog.includes(" WARN ")) {
    return {
      level: "WARN",
      className: "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
    }
  }

  if (upperLog.includes("[INFO]") || upperLog.includes(" INFO ")) {
    return {
      level: "INFO",
      className: "border-sky-500/20 bg-sky-500/10 text-sky-700 dark:text-sky-300",
    }
  }

  if (upperLog.includes("[DEBUG]") || upperLog.includes(" DEBUG ")) {
    return {
      level: "DEBUG",
      className: "border-violet-500/20 bg-violet-500/10 text-violet-700 dark:text-violet-300",
    }
  }

  return {
    level: "LOG",
    className: "border-slate-500/20 bg-slate-500/10 text-slate-700 dark:text-slate-300",
  }
}

export function TaskLogsTab({ taskId }: TaskLogsTabProps) {
  const [logs, setLogs] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [hasMore, setHasMore] = useState(true)
  const [copied, setCopied] = useState(false)

  const loadLogs = useCallback(async (offset: number, reset: boolean) => {
    if (reset) {
      setLoading(true)
      setError(null)
    } else {
      setLoadingMore(true)
    }

    try {
      const nextLogs = await getTaskLogs(taskId, PAGE_SIZE, offset)

      setLogs((current) => (reset ? nextLogs : [...current, ...nextLogs]))
      setHasMore(nextLogs.length === PAGE_SIZE)
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : "加载任务日志失败")
    } finally {
      setLoading(false)
      setLoadingMore(false)
    }
  }, [taskId])

  useEffect(() => {
    setLogs([])
    setHasMore(true)
    setError(null)
    void loadLogs(0, true)
  }, [loadLogs, taskId])

  const handleScroll = useCallback((event: UIEvent<HTMLDivElement>) => {
    const target = event.currentTarget
    const remaining = target.scrollHeight - target.scrollTop - target.clientHeight

    if (remaining < LOAD_MORE_THRESHOLD && hasMore && !loadingMore && !loading) {
      void loadLogs(logs.length, false)
    }
  }, [hasMore, loadLogs, loading, loadingMore, logs.length])

  const handleCopyLogs = async () => {
    try {
      await navigator.clipboard.writeText(logs.join("\n"))
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1600)
    } catch {
      setCopied(false)
    }
  }

  const content = useMemo(() => {
    if (loading) {
      return (
        <div className="flex min-h-[320px] items-center justify-center">
          <LoaderCircle className="size-5 animate-spin text-muted-foreground" />
        </div>
      )
    }

    if (error && logs.length === 0) {
      return (
        <div className="flex min-h-[320px] flex-col items-center justify-center rounded-[1.4rem] border border-destructive/20 bg-destructive/5 px-4 text-center">
          <div className="text-base font-semibold tracking-tight text-destructive">日志加载失败</div>
          <p className="mt-2 text-sm leading-6 text-destructive/80">{error}</p>
        </div>
      )
    }

    if (logs.length === 0) {
      return (
        <div className="flex min-h-[320px] flex-col items-center justify-center rounded-[1.4rem] border border-dashed border-border/70 bg-card/50 px-4 text-center">
          <div className="text-base font-semibold tracking-tight text-foreground">暂无日志</div>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            当前任务还没有产生可展示的执行日志。
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
          {logs.map((log, index) => {
            const tone = getLogTone(log)

            return (
              <div
                key={`${index}-${log}`}
                className="grid gap-3 rounded-[1.2rem] border border-border/70 bg-background/70 p-4 md:grid-cols-[56px_80px_minmax(0,1fr)]"
              >
                <div className="text-xs font-medium text-muted-foreground">#{index + 1}</div>
                <Badge variant="outline" className={`w-fit rounded-full ${tone.className}`}>
                  {tone.level}
                </Badge>
                <pre className="overflow-x-auto whitespace-pre-wrap break-words font-mono text-[13px] leading-6 text-foreground">
                  <code>{log}</code>
                </pre>
              </div>
            )
          })}

          {loadingMore ? (
            <div className="flex items-center justify-center py-3 text-sm text-muted-foreground">
              <LoaderCircle className="mr-2 size-4 animate-spin" />
              加载更多日志...
            </div>
          ) : null}

          {!hasMore && logs.length > 0 ? (
            <div className="py-3 text-center text-sm text-muted-foreground">
              日志已全部加载
            </div>
          ) : null}
        </div>
      </div>
    )
  }, [error, handleScroll, hasMore, loading, loadingMore, logs])

  return (
    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
      <CardContent className="space-y-4 p-5">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
              <FileText className="size-3.5" />
              Execution Logs
            </div>
            <div className="flex items-center gap-2">
              <div className="text-lg font-semibold tracking-tight text-foreground">
                任务执行日志
              </div>
              <Badge variant="outline" className="rounded-full px-3 py-1">
                已加载 {logs.length} 条
              </Badge>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="rounded-full"
              onClick={() => void loadLogs(0, true)}
              disabled={loading || loadingMore}
            >
              <RefreshCw className={`mr-1.5 size-3.5 ${loading ? "animate-spin" : ""}`} />
              刷新
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="rounded-full"
              onClick={() => void handleCopyLogs()}
              disabled={logs.length === 0}
            >
              <Copy className="mr-1.5 size-3.5" />
              {copied ? "已复制" : "复制已加载"}
            </Button>
          </div>
        </div>

        {error && logs.length > 0 ? (
          <div className="rounded-[1.2rem] border border-destructive/20 bg-destructive/5 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        ) : null}

        {content}
      </CardContent>
    </Card>
  )
}
