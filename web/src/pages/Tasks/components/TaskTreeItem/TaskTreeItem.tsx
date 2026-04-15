import {
  Ban,
  FolderTree,
  Pause,
  Play,
  Rows3,
  Sparkles,
  Trash2,
} from "lucide-react"

import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { cn } from "@/lib/utils"
import type { Task } from "@/types"
import { TaskStatusBadge } from "../TaskStatusBadge/TaskStatusBadge"
import { TaskRelaunchDialog } from "./TaskRelaunchDialog"
import { TaskSteerDialog } from "./TaskSteerDialog"

type TreeTask = Task & {
  children?: TreeTask[]
}

interface TaskTreeItemProps {
  task: TreeTask
  onClick: (taskId: string) => void
  selectedTaskId?: string | null
  onControlTask?: (taskId: string, action: "pause" | "resume" | "cancel") => void
  onSteerTask?: (taskId: string, message: string) => Promise<void>
  onRelaunchTask?: (taskId: string, reason: string) => Promise<void>
  onDeleteTask?: (task: Task) => void
  formatTime: (timestamp: string | number) => string
}

export function TaskTreeItem({
  task,
  onClick,
  selectedTaskId,
  onControlTask,
  onSteerTask,
  onRelaunchTask,
  onDeleteTask,
  formatTime,
}: TaskTreeItemProps) {
  const isRootTask = !task.parent_task_id
  const actualChildren = task.children ?? []
  const hasVisibleChildren = actualChildren.length > 0
  const isSelected = selectedTaskId === task.task_id

  const truncateDescription = (desc: string, maxLength: number = 60) => {
    if (desc.length <= maxLength) return desc
    return `${desc.substring(0, maxLength)}...`
  }

  return (
    <div className="relative">
      <div className="flex items-start gap-8">
        <Card
          className={cn(
            "relative w-[320px] shrink-0 cursor-pointer rounded-[1.6rem] border-border/70 bg-card/80 shadow-[0_18px_60px_-36px_rgba(15,23,42,0.28)] transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-[0_24px_75px_-42px_rgba(37,99,235,0.38)]",
            isSelected && "border-primary/35 bg-primary/[0.06] shadow-[0_28px_80px_-42px_rgba(37,99,235,0.45)]"
          )}
          onClick={() => onClick(task.task_id)}
        >
          <CardContent className="space-y-5 p-5">
            <div className="flex items-start justify-between gap-3">
              <div className="flex min-w-0 items-start gap-3">
                <div
                  className={cn(
                    "rounded-2xl border p-2.5",
                    isRootTask
                      ? "border-primary/20 bg-primary/12 text-primary"
                      : "border-border/70 bg-background/80 text-muted-foreground"
                  )}
                >
                  {isRootTask ? (
                    <FolderTree className="size-4.5" />
                  ) : (
                    <Rows3 className="size-4.5" />
                  )}
                </div>
                <div className="min-w-0 space-y-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                      {isRootTask ? "Root Task" : "Subtask"}
                    </span>
                    {task.never_ends ? (
                      <span className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-2 py-1 text-[10px] font-medium text-primary">
                        <Sparkles className="size-3" />
                        常驻
                      </span>
                    ) : null}
                  </div>
                  <div
                    className="line-clamp-3 text-base font-semibold leading-6 tracking-tight text-foreground"
                    title={task.description}
                  >
                    {truncateDescription(task.description, isRootTask ? 96 : 72)}
                  </div>
                </div>
              </div>
              <TaskStatusBadge status={task.status} showLabel size="md" />
            </div>

            <div className="grid gap-3 text-sm md:grid-cols-2">
              <div className="rounded-2xl border border-border/70 bg-background/70 px-3 py-2.5">
                <div className="text-[0.68rem] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                  Agent
                </div>
                <div className="mt-1 truncate font-medium text-foreground">{task.agent}</div>
              </div>
              <div className="rounded-2xl border border-border/70 bg-background/70 px-3 py-2.5">
                <div className="text-[0.68rem] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                  Created
                </div>
                <div className="mt-1 truncate font-medium text-foreground">
                  {formatTime(task.created_at)}
                </div>
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3">
              {actualChildren.length > 0 ? (
                <div className="rounded-full border border-border/70 bg-background/70 px-3 py-1.5 text-xs text-muted-foreground">
                  {actualChildren.length} 个子任务
                </div>
              ) : (
                <div />
              )}

              <div
                className="flex flex-wrap items-center gap-2"
                onClick={(event) => event.stopPropagation()}
              >
                {onRelaunchTask ? (
                  <TaskRelaunchDialog
                    taskId={task.task_id}
                    taskDescription={task.description}
                    onSubmit={onRelaunchTask}
                  />
                ) : null}

                {onDeleteTask ? (
                  <Button
                    variant="outline"
                    size="sm"
                    className="rounded-full text-destructive hover:text-destructive"
                    onClick={() => onDeleteTask(task)}
                    title={isRootTask ? "删除任务树" : "删除任务分支"}
                  >
                    <Trash2 className="mr-1.5 size-3.5" />
                    删除
                  </Button>
                ) : null}

                {task.status === "running" && onSteerTask ? (
                  <TaskSteerDialog
                    taskId={task.task_id}
                    taskDescription={task.description}
                    onSubmit={onSteerTask}
                  />
                ) : null}

                {(task.status === "running" || task.status === "paused") && onControlTask ? (
                  <>
                    {task.status === "running" ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="rounded-full"
                        onClick={() => onControlTask(task.task_id, "pause")}
                        title="暂停任务"
                      >
                        <Pause className="mr-1.5 size-3.5" />
                        暂停
                      </Button>
                    ) : null}

                    {task.status === "paused" ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="rounded-full"
                        onClick={() => onControlTask(task.task_id, "resume")}
                        title="恢复任务"
                      >
                        <Play className="mr-1.5 size-3.5" />
                        恢复
                      </Button>
                    ) : null}

                    <Button
                      variant="outline"
                      size="sm"
                      className="rounded-full text-destructive hover:text-destructive"
                      onClick={() => onControlTask(task.task_id, "cancel")}
                      title="取消任务"
                    >
                      <Ban className="mr-1.5 size-3.5" />
                      取消
                    </Button>
                  </>
                ) : null}
              </div>
            </div>
          </CardContent>
        </Card>

        {hasVisibleChildren ? (
          <div className="relative flex flex-col gap-5 pt-6 before:absolute before:left-[-16px] before:top-[48px] before:h-px before:w-4 before:bg-border/80">
            <div className="absolute bottom-[26px] left-[-16px] top-[48px] w-px bg-border/80" />
            {actualChildren.map((child) => (
              <div
                key={child.task_id}
                className="relative before:absolute before:left-[-16px] before:top-[48px] before:h-px before:w-4 before:bg-border/80"
              >
                <TaskTreeItem
                  task={child}
                  onClick={onClick}
                  selectedTaskId={selectedTaskId}
                  onControlTask={onControlTask}
                  onSteerTask={onSteerTask}
                  onRelaunchTask={onRelaunchTask}
                  onDeleteTask={onDeleteTask}
                  formatTime={formatTime}
                />
              </div>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  )
}
