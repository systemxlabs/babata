import { useCallback, useEffect, useState, type ReactNode } from "react"
import {
  Ban,
  CalendarClock,
  FolderOpen,
  Pause,
  Play,
  ScrollText,
  Sparkles,
  Workflow,
} from "lucide-react"

import { controlTask, getTask, getTaskFiles, getTaskLogs } from "@/api"
import { LoadingState } from "@/components/loading-state"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Separator } from "@/components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { TaskStatusBadge } from "@/pages/Tasks/components/TaskStatusBadge"
import type { FileEntry, Task } from "@/types"
import { TaskDirectoryTab } from "./components/TaskDirectoryTab/TaskDirectoryTab"
import { TaskLogsTab } from "./components/TaskLogsTab/TaskLogsTab"

type TabType = "directory" | "logs"

interface TaskDetailModalProps {
  taskId: string | null
  isOpen: boolean
  onClose: () => void
}

export function TaskDetailModal({ taskId, isOpen, onClose }: TaskDetailModalProps) {
  const [task, setTask] = useState<Task | null>(null)
  const [files, setFiles] = useState<FileEntry[]>([])
  const [logs, setLogs] = useState<string[]>([])
  const [activeTab, setActiveTab] = useState<TabType>("directory")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [actionLoading, setActionLoading] = useState<"pause" | "resume" | "cancel" | null>(null)

  const fetchTaskDetail = useCallback(async () => {
    if (!taskId) return

    setLoading(true)
    setError(null)

    try {
      const [taskData, filesData, logsData] = await Promise.all([
        getTask(taskId),
        getTaskFiles(taskId),
        getTaskLogs(taskId, 1000),
      ])

      setTask(taskData)
      setFiles(filesData)
      setLogs(logsData)
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载任务详情失败")
      setTask(null)
      setFiles([])
      setLogs([])
    } finally {
      setLoading(false)
    }
  }, [taskId])

  useEffect(() => {
    if (!isOpen || !taskId) return
    void fetchTaskDetail()
  }, [fetchTaskDetail, isOpen, taskId])

  useEffect(() => {
    if (!isOpen) {
      setActiveTab("directory")
    }
  }, [isOpen])

  const handleControlTask = useCallback(async (action: "pause" | "resume" | "cancel") => {
    if (!taskId) return

    setActionLoading(action)
    try {
      await controlTask(taskId, action)
      await fetchTaskDetail()
    } catch (err) {
      setError(err instanceof Error ? err.message : "任务操作失败")
    } finally {
      setActionLoading(null)
    }
  }, [fetchTaskDetail, taskId])

  const formatTime = useCallback((timestamp: string | number): string => {
    return new Date(timestamp).toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    })
  }, [])

  let controlButtons: ReactNode = null

  if (task?.status === "running") {
    controlButtons = (
      <>
        <Button
          variant="outline"
          className="rounded-full"
          onClick={() => void handleControlTask("pause")}
          disabled={actionLoading !== null}
        >
          <Pause className="mr-2 size-4" />
          {actionLoading === "pause" ? "暂停中..." : "暂停"}
        </Button>
        <Button
          variant="outline"
          className="rounded-full text-destructive hover:text-destructive"
          onClick={() => void handleControlTask("cancel")}
          disabled={actionLoading !== null}
        >
          <Ban className="mr-2 size-4" />
          {actionLoading === "cancel" ? "取消中..." : "取消"}
        </Button>
      </>
    )
  } else if (task?.status === "paused") {
    controlButtons = (
      <Button
        variant="outline"
        className="rounded-full"
        onClick={() => void handleControlTask("resume")}
        disabled={actionLoading !== null}
      >
        <Play className="mr-2 size-4" />
        {actionLoading === "resume" ? "恢复中..." : "恢复"}
      </Button>
    )
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="max-h-[calc(100vh-2rem)] overflow-hidden rounded-[1.9rem] border-border/70 bg-card/95 p-0 sm:max-w-[1320px]">
        {taskId ? (
          <>
            <DialogHeader className="space-y-4 px-6 pt-6">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="space-y-3">
                  <Badge variant="outline" className="rounded-full px-3 py-1">
                    <Workflow className="mr-2 size-3.5" />
                    Task
                  </Badge>
                  <div>
                    <DialogTitle className="text-2xl tracking-tight">
                      任务详情
                    </DialogTitle>
                    <DialogDescription className="mt-2 max-w-3xl text-sm leading-6">
                      查看任务元信息、工作目录和执行日志，并在允许的情况下控制任务运行状态。
                    </DialogDescription>
                  </div>
                </div>
                <Badge variant="secondary" className="rounded-full px-3 py-1.5">
                  <Sparkles className="mr-2 size-3.5" />
                  实时执行视图
                </Badge>
              </div>
            </DialogHeader>

            <Separator className="mt-5" />

            <div className="min-h-0 space-y-5 overflow-y-auto px-6 py-6">
              {loading ? (
                <LoadingState
                  title="加载任务详情"
                  description="正在同步任务元信息、目录和日志。"
                  className="min-h-[520px]"
                />
              ) : error ? (
                <Card className="rounded-[1.6rem] border-destructive/25 bg-destructive/5">
                  <CardContent className="p-5 text-sm text-destructive">{error}</CardContent>
                </Card>
              ) : task ? (
                <>
                  <div className="grid gap-4 xl:grid-cols-4 md:grid-cols-2">
                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="p-5">
                        <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          Status
                        </div>
                        <div className="mt-3">
                          <TaskStatusBadge status={task.status} />
                        </div>
                      </CardContent>
                    </Card>
                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="p-5">
                        <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          Agent
                        </div>
                        <div className="mt-3 text-base font-semibold tracking-tight text-foreground">
                          {task.agent}
                        </div>
                      </CardContent>
                    </Card>
                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="p-5">
                        <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          <CalendarClock className="size-3.5" />
                          Created At
                        </div>
                        <div className="mt-3 text-sm font-medium text-foreground">
                          {formatTime(task.created_at)}
                        </div>
                      </CardContent>
                    </Card>
                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="p-5">
                        <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          Flags
                        </div>
                        <div className="mt-3 flex flex-wrap gap-2">
                          <Badge variant="outline" className="rounded-full px-3 py-1">
                            {task.parent_task_id ? "子任务" : "根任务"}
                          </Badge>
                          {task.never_ends ? (
                            <Badge variant="secondary" className="rounded-full px-3 py-1">
                              常驻任务
                            </Badge>
                          ) : null}
                        </div>
                      </CardContent>
                    </Card>
                  </div>

                  <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                    <CardContent className="grid gap-4 p-5 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
                      <div className="space-y-4">
                        <div>
                          <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                            Description
                          </div>
                          <div className="mt-3 text-base font-semibold leading-7 tracking-tight text-foreground">
                            {task.description}
                          </div>
                        </div>
                        <div className="grid gap-4 md:grid-cols-2">
                          <div>
                            <div className="text-xs text-muted-foreground">Task ID</div>
                            <div className="mt-1 break-all font-mono text-[13px] text-foreground">
                              {task.task_id}
                            </div>
                          </div>
                          <div>
                            <div className="text-xs text-muted-foreground">Root Task ID</div>
                            <div className="mt-1 break-all font-mono text-[13px] text-foreground">
                              {task.root_task_id}
                            </div>
                          </div>
                        </div>
                      </div>

                      {controlButtons ? (
                        <div className="flex flex-wrap gap-2">{controlButtons}</div>
                      ) : null}
                    </CardContent>
                  </Card>

                  <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as TabType)}>
                    <TabsList className="h-auto rounded-[1.3rem] border border-border/70 bg-background/70 p-1.5">
                      <TabsTrigger value="directory" className="rounded-xl px-4 py-2">
                        <FolderOpen className="mr-1.5 size-4" />
                        任务目录
                      </TabsTrigger>
                      <TabsTrigger value="logs" className="rounded-xl px-4 py-2">
                        <ScrollText className="mr-1.5 size-4" />
                        任务日志
                      </TabsTrigger>
                    </TabsList>

                    <TabsContent value="directory" className="mt-4">
                      <TaskDirectoryTab taskId={taskId} files={files} />
                    </TabsContent>
                    <TabsContent value="logs" className="mt-4">
                      <TaskLogsTab logs={logs} onRefresh={() => void fetchTaskDetail()} />
                    </TabsContent>
                  </Tabs>
                </>
              ) : null}
            </div>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  )
}
