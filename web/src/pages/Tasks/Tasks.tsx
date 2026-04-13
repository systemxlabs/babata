import { useCallback, useEffect, useMemo, useState } from "react"
import {
  FolderTree,
  GitBranchPlus,
  RefreshCw,
  Trash2,
  Workflow,
} from "lucide-react"

import { controlTask, deleteTask, getRootTasks, getTaskTree } from "@/api"
import type { TaskTreeResponse } from "@/api"
import { DeleteConfirmModal } from "@/components/DeleteConfirmModal"
import { EmptyState } from "@/components/empty-state"
import { LoadingState } from "@/components/loading-state"
import { PageHeader } from "@/components/page-header"
import { TaskDetailModal } from "@/components/TaskDetailModal"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { TaskListHeader } from "@/pages/Tasks/components/TaskListHeader"
import { TaskPagination } from "@/pages/Tasks/components/TaskPagination"
import { TaskStatusBadge } from "@/pages/Tasks/components/TaskStatusBadge"
import { TaskTreeItem } from "@/pages/Tasks/components/TaskTreeItem"
import { STATUS_LABELS, type RootTask, type Task, type TaskFilter } from "@/types"

function countTreeNodes(tree: TaskTreeResponse | null): number {
  if (!tree) return 0
  return 1 + tree.children.reduce((total, child) => total + countTreeNodes(child), 0)
}

export function Tasks() {
  const [tasks, setTasks] = useState<RootTask[]>([])
  const [loading, setLoading] = useState(false)
  const [treeLoading, setTreeLoading] = useState(false)
  const [total, setTotal] = useState(0)
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null)
  const [selectedRootTaskId, setSelectedRootTaskId] = useState<string | null>(null)
  const [selectedTree, setSelectedTree] = useState<TaskTreeResponse | null>(null)
  const [treeError, setTreeError] = useState<string | null>(null)
  const [showDeleteModal, setShowDeleteModal] = useState(false)
  const [taskToDelete, setTaskToDelete] = useState<Task | null>(null)
  const [filter, setFilter] = useState<TaskFilter>({
    status: "running",
    page: 1,
    pageSize: 20,
  })

  const hasStatusFilter = filter.status !== undefined && filter.status !== "all"

  const sortTaskTreeByCreatedAt = useCallback((tree: TaskTreeResponse): TaskTreeResponse => {
    return {
      ...tree,
      children: [...tree.children]
        .sort((a, b) => a.created_at - b.created_at)
        .map(sortTaskTreeByCreatedAt),
    }
  }, [])

  const fetchTasks = useCallback(async () => {
    setLoading(true)
    try {
      const response = await getRootTasks(filter)
      setTasks(response.tasks)
      setTotal(response.total)
    } catch (error) {
      console.error("Failed to fetch tasks:", error)
    } finally {
      setLoading(false)
    }
  }, [filter])

  const fetchTree = useCallback(async (taskId: string) => {
    setTreeLoading(true)
    setTreeError(null)

    try {
      const tree = await getTaskTree(taskId)
      setSelectedTree(sortTaskTreeByCreatedAt(tree))
    } catch (error) {
      console.error("Failed to fetch task tree:", error)
      setSelectedTree(null)
      setTreeError(error instanceof Error ? error.message : "加载任务树失败")
    } finally {
      setTreeLoading(false)
    }
  }, [sortTaskTreeByCreatedAt])

  useEffect(() => {
    void fetchTasks()
  }, [fetchTasks])

  useEffect(() => {
    if (tasks.length === 0) {
      setSelectedRootTaskId(null)
      setSelectedTree(null)
      return
    }

    if (!selectedRootTaskId || !tasks.some((task) => task.task_id === selectedRootTaskId)) {
      setSelectedRootTaskId(tasks[0].task_id)
    }
  }, [selectedRootTaskId, tasks])

  useEffect(() => {
    if (!selectedRootTaskId) {
      setSelectedTree(null)
      setTreeError(null)
      return
    }

    let cancelled = false

    const run = async () => {
      setTreeLoading(true)
      setTreeError(null)

      try {
        const tree = await getTaskTree(selectedRootTaskId)
        if (!cancelled) {
          setSelectedTree(sortTaskTreeByCreatedAt(tree))
        }
      } catch (error) {
        console.error("Failed to fetch task tree:", error)
        if (!cancelled) {
          setSelectedTree(null)
          setTreeError(error instanceof Error ? error.message : "加载任务树失败")
        }
      } finally {
        if (!cancelled) {
          setTreeLoading(false)
        }
      }
    }

    void run()

    return () => {
      cancelled = true
    }
  }, [selectedRootTaskId, sortTaskTreeByCreatedAt])

  useEffect(() => {
    setSelectedTaskId(null)
  }, [selectedRootTaskId])

  useEffect(() => {
    window.scrollTo({ top: 0, behavior: "auto" })
  }, [filter.page])

  const handleFilterChange = useCallback((newFilter: Partial<TaskFilter>) => {
    setFilter((prev) => ({
      ...prev,
      ...newFilter,
      page: 1,
    }))
  }, [])

  const handlePageChange = useCallback((page: number) => {
    setFilter((prev) => ({ ...prev, page }))
  }, [])

  const handleTaskClick = useCallback((taskId: string) => {
    setSelectedTaskId(taskId)
  }, [])

  const selectedRootTask = useMemo(
    () => tasks.find((task) => task.task_id === selectedRootTaskId) ?? null,
    [selectedRootTaskId, tasks]
  )

  const handleRootTaskSelect = useCallback((taskId: string) => {
    setSelectedRootTaskId(taskId)
  }, [])

  const handleDeleteTreeClick = useCallback(() => {
    if (!selectedRootTask) return
    setTaskToDelete(selectedRootTask)
    setShowDeleteModal(true)
  }, [selectedRootTask])

  const handleDeleteConfirm = useCallback(async () => {
    if (!taskToDelete) return

    await deleteTask(taskToDelete.task_id)
    await fetchTasks()
    setShowDeleteModal(false)
    setTaskToDelete(null)
  }, [fetchTasks, taskToDelete])

  const handleControlTask = useCallback(async (taskId: string, action: "pause" | "resume" | "cancel") => {
    try {
      await controlTask(taskId, action)
      await fetchTasks()
      if (selectedRootTaskId) {
        await fetchTree(selectedRootTaskId)
      }
    } catch (error) {
      console.error(`Failed to ${action} task:`, error)
    }
  }, [fetchTasks, fetchTree, selectedRootTaskId])

  const formatTime = useCallback((timestamp: string | number) => {
    return new Date(timestamp).toLocaleString("zh-CN")
  }, [])

  if (loading && tasks.length === 0) {
    return <LoadingState title="加载任务工作区" description="正在同步根任务列表与任务树。" />
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Execution"
        title="任务管理"
        description="按状态筛选根任务，在右侧浏览整棵任务树，并针对节点查看执行详情、目录和日志。"
        actions={
          <>
            <Badge variant="outline" className="rounded-full px-3 py-1.5">
              {total} 个根任务
            </Badge>
            <Button onClick={() => void fetchTasks()} disabled={loading}>
              <RefreshCw className={`mr-2 size-4 ${loading ? "animate-spin" : ""}`} />
              刷新列表
            </Button>
          </>
        }
      />

      <TaskListHeader filter={filter} onFilterChange={handleFilterChange} loading={loading} />

      {tasks.length === 0 ? (
        <EmptyState
          icon={Workflow}
          title="暂无根任务"
          description={
            hasStatusFilter
              ? `当前没有状态为“${filter.status ? STATUS_LABELS[filter.status] : "全部"}”的根任务，可切换其他标签或再次点击当前标签查看全部。`
              : "当前没有任何根任务。"
          }
        />
      ) : (
        <div className="grid gap-6 xl:grid-cols-[320px_minmax(0,1fr)]">
          <Card className="rounded-[2rem] border-border/70 bg-card/70 shadow-[0_20px_65px_-36px_rgba(15,23,42,0.25)] backdrop-blur-xl">
            <CardHeader className="space-y-3">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                    Root Tasks
                  </div>
                  <CardTitle className="mt-2 text-2xl tracking-tight">根任务列表</CardTitle>
                </div>
                <Badge variant="secondary" className="rounded-full px-3 py-1">
                  {tasks.length} 项
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="pt-0">
              <ScrollArea
                key={`${filter.status ?? "all"}-${filter.page}`}
                className="h-[720px] pr-3"
              >
                <div className="space-y-3">
                  {tasks.map((task) => {
                    const isActive = selectedRootTaskId === task.task_id

                    return (
                      <button
                        key={task.task_id}
                        type="button"
                        className={`w-full rounded-[1.5rem] border p-4 text-left transition-all ${
                          isActive
                            ? "border-primary/30 bg-primary/[0.06] shadow-[0_20px_55px_-36px_rgba(37,99,235,0.45)]"
                            : "border-border/70 bg-background/70 hover:-translate-y-0.5 hover:border-border"
                        }`}
                        onClick={() => handleRootTaskSelect(task.task_id)}
                      >
                        <div className="flex items-start justify-between gap-3">
                          <div className="min-w-0 space-y-2">
                            <div className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                              {task.agent}
                            </div>
                            <div className="line-clamp-2 text-base font-semibold leading-6 tracking-tight text-foreground">
                              {task.description}
                            </div>
                          </div>
                          <TaskStatusBadge status={task.status} size="sm" />
                        </div>
                        <div className="mt-4 flex flex-wrap gap-2 text-xs text-muted-foreground">
                          <span>创建于 {formatTime(task.created_at)}</span>
                          <span>{task.subtask_count} 个子任务</span>
                        </div>
                      </button>
                    )
                  })}
                </div>
              </ScrollArea>
            </CardContent>
          </Card>

          <div className="space-y-6">
            <Card className="rounded-[2rem] border-border/70 bg-card/70 shadow-[0_20px_65px_-36px_rgba(15,23,42,0.25)] backdrop-blur-xl">
              <CardContent className="grid gap-4 p-6 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="rounded-[1.5rem] border border-border/70 bg-background/70 p-4">
                    <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                      <FolderTree className="size-3.5" />
                      当前根任务
                    </div>
                    <div className="mt-3 text-lg font-semibold tracking-tight text-foreground">
                      {selectedRootTask?.description ?? "未选择"}
                    </div>
                  </div>
                  <div className="rounded-[1.5rem] border border-border/70 bg-background/70 p-4">
                    <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                      状态
                    </div>
                    <div className="mt-3">
                      {selectedRootTask ? (
                        <TaskStatusBadge status={selectedRootTask.status} />
                      ) : (
                        <span className="text-sm text-muted-foreground">未选择</span>
                      )}
                    </div>
                  </div>
                  <div className="rounded-[1.5rem] border border-border/70 bg-background/70 p-4">
                    <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                      <GitBranchPlus className="size-3.5" />
                      节点总数
                    </div>
                    <div className="mt-3 text-3xl font-semibold tracking-tight text-foreground">
                      {countTreeNodes(selectedTree)}
                    </div>
                  </div>
                </div>

                {selectedRootTask ? (
                  <Button
                    variant="outline"
                    className="rounded-full text-destructive hover:text-destructive"
                    onClick={handleDeleteTreeClick}
                  >
                    <Trash2 className="mr-2 size-4" />
                    删除整棵任务树
                  </Button>
                ) : null}
              </CardContent>
            </Card>

            <Card className="rounded-[2rem] border-border/70 bg-card/70 shadow-[0_20px_65px_-36px_rgba(15,23,42,0.25)] backdrop-blur-xl">
              <CardHeader className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                <div>
                  <div className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                    Tree View
                  </div>
                  <CardTitle className="mt-2 text-2xl tracking-tight">任务树</CardTitle>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    点击节点查看任务详情、文件目录与执行日志。
                  </p>
                </div>
                {selectedRootTask ? (
                  <Badge variant="outline" className="rounded-full px-3 py-1.5">
                    {selectedRootTask.agent}
                  </Badge>
                ) : null}
              </CardHeader>
              <CardContent className="pt-0">
                {treeLoading ? (
                  <LoadingState
                    title="加载任务树"
                    description="正在展开当前根任务的完整层级。"
                    className="min-h-[460px] border-none bg-transparent shadow-none"
                  />
                ) : treeError ? (
                  <EmptyState
                    icon={Workflow}
                    title="任务树加载失败"
                    description={treeError}
                    className="min-h-[460px] border-none bg-transparent shadow-none"
                  />
                ) : selectedTree ? (
                  <div className="overflow-x-auto pb-2">
                    <div className="min-w-max pr-6 pt-2">
                      <TaskTreeItem
                        task={selectedTree}
                        selectedTaskId={selectedTaskId}
                        onClick={handleTaskClick}
                        onControlTask={handleControlTask}
                        formatTime={formatTime}
                      />
                    </div>
                  </div>
                ) : (
                  <EmptyState
                    icon={Workflow}
                    title="未加载到任务树"
                    description="请重新选择根任务或刷新列表。"
                    className="min-h-[460px] border-none bg-transparent shadow-none"
                  />
                )}
              </CardContent>
            </Card>
          </div>
        </div>
      )}

      {total > filter.pageSize ? (
        <TaskPagination
          currentPage={filter.page}
          pageSize={filter.pageSize}
          total={total}
          onPageChange={handlePageChange}
        />
      ) : null}

      <TaskDetailModal
        taskId={selectedTaskId}
        isOpen={selectedTaskId !== null}
        onClose={() => setSelectedTaskId(null)}
      />

      <DeleteConfirmModal
        isOpen={showDeleteModal}
        taskId={taskToDelete?.task_id ?? ""}
        taskDescription={taskToDelete?.description ?? ""}
        onConfirm={handleDeleteConfirm}
        onCancel={() => {
          setShowDeleteModal(false)
          setTaskToDelete(null)
        }}
      />
    </div>
  )
}
