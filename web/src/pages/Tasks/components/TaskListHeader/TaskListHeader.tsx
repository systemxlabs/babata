import { useCallback } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { STATUS_LABELS, type TaskFilter, type TaskStatus } from "@/types"

interface TaskListHeaderProps {
  filter: TaskFilter
  onFilterChange: (filter: Partial<TaskFilter>) => void
  loading?: boolean
}

const STATUS_OPTIONS: TaskStatus[] = [
  "running",
  "completed",
  "failed",
  "paused",
  "canceled",
]

export function TaskListHeader({ filter, onFilterChange, loading }: TaskListHeaderProps) {
  const handleStatusToggle = useCallback((status: TaskStatus) => {
    const nextStatus = filter.status === status ? "all" : status
    onFilterChange({ status: nextStatus })
  }, [filter.status, onFilterChange])

  const activeStatus = filter.status && filter.status !== "all" ? filter.status : null

  return (
    <div className="rounded-[1.85rem] border border-border/70 bg-card/70 p-4 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
        <div className="space-y-3">
          <div className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
            Status Filter
          </div>
          <div
            className="inline-flex flex-wrap gap-2 rounded-[1.4rem] border border-border/70 bg-background/70 p-2"
            aria-label="任务状态筛选"
          >
            {STATUS_OPTIONS.map((status) => (
              <Button
                key={status}
                type="button"
                variant={activeStatus === status ? "default" : "ghost"}
                className={cn(
                  "rounded-xl px-4",
                  activeStatus !== status && "text-muted-foreground hover:text-foreground"
                )}
                onClick={() => handleStatusToggle(status)}
                disabled={loading}
                aria-pressed={activeStatus === status}
              >
                {STATUS_LABELS[status]}
              </Button>
            ))}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Badge variant="outline" className="rounded-full px-3 py-1.5">
            {activeStatus ? `当前筛选：${STATUS_LABELS[activeStatus]}` : "当前筛选：全部根任务"}
          </Badge>
          {!activeStatus ? null : (
            <button
              type="button"
              className="text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
              onClick={() => onFilterChange({ status: "all" })}
              disabled={loading}
            >
              清除筛选
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
