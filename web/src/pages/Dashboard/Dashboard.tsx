import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Activity,
  Bot,
  Package2,
  PlayCircle,
  Plus,
  PlugZap,
  RefreshCw,
  Sparkles,
  Workflow,
} from "lucide-react"

import { api } from "@/api"
import { EmptyState } from "@/components/empty-state"
import { LoadingState } from "@/components/loading-state"
import { PageHeader } from "@/components/page-header"
import { StatCard } from "@/components/stat-card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { TaskStatusBadge } from "@/pages/Tasks/components/TaskStatusBadge"
import type { Agent, ProviderConfig, Skill, Task } from "@/types"

function formatTimeAgo(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes < 1) return "刚刚"
  if (minutes < 60) return `${minutes} 分钟前`
  if (hours < 24) return `${hours} 小时前`
  return `${days} 天前`
}

export function Dashboard() {
  const [runningCount, setRunningCount] = useState(0)
  const [agents, setAgents] = useState<Agent[]>([])
  const [providers, setProviders] = useState<ProviderConfig[]>([])
  const [skills, setSkills] = useState<Skill[]>([])
  const [tasks, setTasks] = useState<Task[]>([])
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date())
  const [selectedAgent, setSelectedAgent] = useState("")
  const [taskDescription, setTaskDescription] = useState("")
  const [isCreating, setIsCreating] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchStats = useCallback(async () => {
    try {
      const [runningRes, agentsRes, providersRes, skillsRes] = await Promise.all([
        api.getRunningTasksCount(),
        api.getAgents(),
        api.getProviders(),
        api.getSkills(),
      ])

      setRunningCount(runningRes.count)
      setAgents(agentsRes.agents)
      setProviders(providersRes.providers)
      setSkills(skillsRes.skills)

      if (agentsRes.agents.length > 0 && !selectedAgent) {
        const defaultAgent = agentsRes.agents.find((agent) => agent.default)
        setSelectedAgent(defaultAgent?.name ?? agentsRes.agents[0].name)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取统计数据失败")
    }
  }, [selectedAgent])

  const fetchTasks = useCallback(async () => {
    try {
      const response = await api.getRunningTasks(20)
      setTasks(response.tasks)
      setLastUpdate(new Date())
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取任务列表失败")
    }
  }, [])

  const refreshData = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    await Promise.all([fetchStats(), fetchTasks()])
    setIsLoading(false)
  }, [fetchStats, fetchTasks])

  useEffect(() => {
    void refreshData()
  }, [refreshData])

  useEffect(() => {
    const interval = setInterval(() => {
      void fetchStats()
      void fetchTasks()
    }, 10000)

    return () => clearInterval(interval)
  }, [fetchStats, fetchTasks])

  const handleCreateTask = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!selectedAgent || !taskDescription.trim()) return

    setIsCreating(true)
    try {
      await api.createTask({
        agent: selectedAgent,
        prompt: taskDescription.trim(),
        description: taskDescription.trim(),
      })
      setTaskDescription("")
      await refreshData()
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建任务失败")
    } finally {
      setIsCreating(false)
    }
  }

  const rootTasksWithChildren = useMemo(() => {
    const rootTasks = tasks.filter((task) => !task.parent_task_id)

    return rootTasks.map((rootTask) => {
      const childrenCount = tasks.filter(
        (task) => task.root_task_id === rootTask.task_id && task.parent_task_id
      ).length
      return { ...rootTask, childrenCount }
    })
  }, [tasks])

  if (isLoading && agents.length === 0 && tasks.length === 0) {
    return <LoadingState title="初始化控制台" description="正在装载运行态概览与关键资源。" />
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Operations"
        title="Babata Overview"
        description="集中查看运行中任务、资源配置健康度，并从这里快速发起新的根任务。"
        actions={
          <>
            <Badge variant="outline" className="rounded-full px-3 py-1 text-xs">
              最后更新 {lastUpdate.toLocaleTimeString()}
            </Badge>
            <Button onClick={() => void refreshData()} disabled={isLoading}>
              <RefreshCw className={`mr-2 size-4 ${isLoading ? "animate-spin" : ""}`} />
              刷新概览
            </Button>
          </>
        }
      />

      {error ? (
        <Card className="rounded-[1.75rem] border-destructive/25 bg-destructive/5">
          <CardContent className="flex items-center justify-between gap-4 p-5 text-sm text-destructive">
            <span>{error}</span>
            <Button variant="ghost" size="sm" onClick={() => setError(null)}>
              关闭
            </Button>
          </CardContent>
        </Card>
      ) : null}

      <section className="grid gap-4 xl:grid-cols-5 md:grid-cols-2">
        <StatCard icon={Activity} label="运行中任务" value={runningCount} tone="amber" />
        <StatCard icon={Workflow} label="运行中根任务" value={rootTasksWithChildren.length} tone="primary" />
        <StatCard icon={Bot} label="Agents" value={agents.length} tone="cyan" />
        <StatCard icon={PlugZap} label="Providers" value={providers.length} tone="emerald" />
        <StatCard icon={Package2} label="Skills" value={skills.length} tone="rose" />
      </section>

      <Card className="rounded-[2rem] border-border/70 bg-card/70 shadow-[0_20px_65px_-36px_rgba(15,23,42,0.25)] backdrop-blur-xl">
        <CardHeader className="space-y-2">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
            <Sparkles className="size-3.5" />
            Quick Start
          </div>
          <CardTitle className="text-2xl tracking-tight">快速创建根任务</CardTitle>
        </CardHeader>
        <CardContent>
          <form className="space-y-4" onSubmit={handleCreateTask}>
            <div className="grid gap-4 md:grid-cols-[260px_minmax(0,1fr)]">
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">Agent</label>
                <Select
                  value={selectedAgent}
                  onValueChange={setSelectedAgent}
                  disabled={isCreating || agents.length === 0}
                >
                  <SelectTrigger className="h-12 rounded-2xl">
                    <SelectValue placeholder="选择 Agent" />
                  </SelectTrigger>
                  <SelectContent>
                    {agents.map((agent) => (
                      <SelectItem key={agent.name} value={agent.name}>
                        {agent.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">任务描述</label>
                <Input
                  placeholder="例如：检查今天失败的任务并生成汇总"
                  value={taskDescription}
                  onChange={(event) => setTaskDescription(event.target.value)}
                  disabled={isCreating}
                  className="h-12 rounded-2xl"
                />
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-sm text-muted-foreground">
                新任务会以根任务方式创建，并立即进入调度。
              </p>
              <Button
                type="submit"
                size="lg"
                disabled={isCreating || !selectedAgent || !taskDescription.trim()}
              >
                <Plus className="mr-2 size-4" />
                {isCreating ? "创建中..." : "创建任务"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card className="rounded-[2rem] border-border/70 bg-card/70 shadow-[0_20px_65px_-36px_rgba(15,23,42,0.25)] backdrop-blur-xl">
        <CardHeader className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
              <PlayCircle className="size-3.5" />
              Live Queue
            </div>
            <CardTitle className="mt-2 text-2xl tracking-tight">正在运行的根任务</CardTitle>
          </div>
          <Badge variant="outline" className="w-fit rounded-full px-3 py-1.5 text-xs">
            {rootTasksWithChildren.length} 个根任务
          </Badge>
        </CardHeader>
        <CardContent>
          {rootTasksWithChildren.length === 0 ? (
            <EmptyState
              icon={Workflow}
              title="当前没有运行中的根任务"
              description="从上方快速创建区发起一个任务，或者等待外部渠道新的请求进入。"
              className="min-h-[260px] border-none bg-transparent px-0 shadow-none"
            />
          ) : (
            <div className="grid gap-4 xl:grid-cols-2">
              {rootTasksWithChildren.map((task) => (
                <div
                  key={task.task_id}
                  className="rounded-[1.6rem] border border-border/70 bg-background/70 p-5 transition-transform duration-200 hover:-translate-y-0.5"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="space-y-2">
                      <div className="text-lg font-semibold tracking-tight text-foreground">
                        {task.description}
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <Badge variant="outline" className="rounded-full px-3 py-1">
                          {task.agent}
                        </Badge>
                        {task.never_ends ? (
                          <Badge variant="secondary" className="rounded-full px-3 py-1">
                            常驻任务
                          </Badge>
                        ) : null}
                      </div>
                    </div>
                    <TaskStatusBadge status={task.status} />
                  </div>
                  <div className="mt-4 flex flex-wrap gap-4 text-sm text-muted-foreground">
                    <span>创建于 {formatTimeAgo(task.created_at)}</span>
                    <span>{task.childrenCount} 个子任务</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
