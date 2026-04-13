import { useEffect, useRef, useState } from "react"
import { Copy, FileText, RefreshCw } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"

interface TaskLogsTabProps {
  logs: string[]
  onRefresh?: () => void
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

export function TaskLogsTab({ logs, onRefresh }: TaskLogsTabProps) {
  const logsEndRef = useRef<HTMLDivElement>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [logs])

  const handleCopyLogs = async () => {
    try {
      await navigator.clipboard.writeText(logs.join("\n"))
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1600)
    } catch {
      setCopied(false)
    }
  }

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
                {logs.length} 条
              </Badge>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {onRefresh ? (
              <Button variant="outline" size="sm" className="rounded-full" onClick={onRefresh}>
                <RefreshCw className="mr-1.5 size-3.5" />
                刷新
              </Button>
            ) : null}
            <Button
              variant="outline"
              size="sm"
              className="rounded-full"
              onClick={() => void handleCopyLogs()}
              disabled={logs.length === 0}
            >
              <Copy className="mr-1.5 size-3.5" />
              {copied ? "已复制" : "复制全部"}
            </Button>
          </div>
        </div>

        {logs.length === 0 ? (
          <div className="flex min-h-[320px] flex-col items-center justify-center rounded-[1.4rem] border border-dashed border-border/70 bg-card/50 px-4 text-center">
            <div className="text-base font-semibold tracking-tight text-foreground">暂无日志</div>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              当前任务还没有产生可展示的执行日志。
            </p>
          </div>
        ) : (
          <ScrollArea className="h-[420px] rounded-[1.4rem] border border-border/70 bg-card/60">
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
              <div ref={logsEndRef} />
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  )
}
