import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Bot,
  Pencil,
  Plus,
  ShieldAlert,
  Trash2,
} from "lucide-react"

import {
  createAgent,
  deleteAgent as removeAgent,
  getAgent,
  getAgents,
  getProviders,
  updateAgent,
} from "@/api"
import { AgentDetailModal } from "@/components/AgentDetailModal/AgentDetailModal"
import { EmptyState } from "@/components/empty-state"
import { ErrorAlert } from "@/components/error-alert"
import { LoadingState } from "@/components/loading-state"
import { PageHeader } from "@/components/page-header"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Textarea } from "@/components/ui/textarea"
import type {
  AgentDetail,
  AgentFrontmatter,
  CreateAgentRequest,
  ProviderConfig,
  UpdateAgentRequest,
} from "@/types"

interface AgentModalProps {
  isOpen: boolean
  onClose: () => void
  onSubmit: (data: CreateAgentRequest | UpdateAgentRequest) => Promise<void>
  providerConfigs: ProviderConfig[]
  agent?: AgentDetail | null
  mode: "create" | "edit"
}

function createInitialForm(providerConfigs: ProviderConfig[]): CreateAgentRequest {
  const firstProvider = providerConfigs[0]
  const firstModel = firstProvider?.models[0]

  return {
    name: "",
    description: "",
    provider: firstProvider?.name ?? "",
    model: firstModel?.id ?? "",
    allowed_tools: [],
    default: false,
    body: "",
  }
}

function formatContextWindowTokens(value: number): string {
  return `${value.toLocaleString("en-US")} tokens`
}

function AgentModal({
  isOpen,
  onClose,
  onSubmit,
  providerConfigs,
  agent,
  mode,
}: AgentModalProps) {
  const [formData, setFormData] = useState<CreateAgentRequest>(createInitialForm(providerConfigs))
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!isOpen) return

    if (agent && mode === "edit") {
      setFormData({
        name: agent.name,
        description: agent.description,
        provider: agent.provider,
        model: agent.model,
        allowed_tools: agent.allowed_tools,
        default: agent.default,
        body: agent.body,
      })
    } else {
      setFormData(createInitialForm(providerConfigs))
    }

    setError(null)
  }, [agent, isOpen, mode, providerConfigs])

  const selectedProvider = useMemo(
    () => providerConfigs.find((provider) => provider.name === formData.provider) ?? null,
    [formData.provider, providerConfigs]
  )

  const selectedModel = useMemo(
    () => selectedProvider?.models.find((model) => model.id === formData.model) ?? null,
    [formData.model, selectedProvider]
  )

  useEffect(() => {
    if (!isOpen) return

    if (!selectedProvider) {
      if (providerConfigs.length > 0) {
        const firstProvider = providerConfigs[0]
        setFormData((current) => ({
          ...current,
          provider: firstProvider.name,
          model: firstProvider.models[0]?.id ?? "",
        }))
      }
      return
    }

    const modelExists = selectedProvider.models.some((model) => model.id === formData.model)
    if (!modelExists) {
      setFormData((current) => ({
        ...current,
        model: selectedProvider.models[0]?.id ?? "",
      }))
    }
  }, [formData.model, isOpen, providerConfigs, selectedProvider])

  const toolsValue = useMemo(() => formData.allowed_tools.join(", "), [formData.allowed_tools])

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()

    if (!formData.name.trim() || !formData.body.trim()) {
      setError("名称和 Body 不能为空")
      return
    }
    if (!formData.provider.trim()) {
      setError("Provider 不能为空")
      return
    }
    if (!formData.model.trim()) {
      setError("Model 不能为空")
      return
    }

    setLoading(true)
    setError(null)

    try {
      await onSubmit({
        ...formData,
        name: formData.name.trim(),
        description: formData.description.trim(),
        provider: formData.provider.trim(),
        model: formData.model.trim(),
        body: formData.body,
      })
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存 Agent 失败")
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="grid max-h-[calc(100vh-2rem)] grid-rows-[auto_minmax(0,1fr)] overflow-hidden rounded-[1.85rem] border-border/70 bg-card/95 sm:max-w-[760px]">
        <DialogHeader>
          <DialogTitle className="text-2xl tracking-tight">
            {mode === "create" ? "创建 Agent" : "编辑 Agent"}
          </DialogTitle>
          <DialogDescription>
            配置 Agent 的描述、Provider、模型、工具白名单和系统提示词正文。
          </DialogDescription>
        </DialogHeader>

        <form className="grid min-h-0 grid-rows-[minmax(0,1fr)_auto]" onSubmit={handleSubmit}>
          <div className="min-h-0 space-y-5 overflow-y-auto pr-1">
            {error ? (
              <div className="rounded-2xl border border-destructive/25 bg-destructive/5 px-4 py-3 text-sm text-destructive">
                {error}
              </div>
            ) : null}

            <div className="grid gap-5 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="agent-name">名称</Label>
                <Input
                  id="agent-name"
                  value={formData.name}
                  onChange={(event) =>
                    setFormData((current) => ({ ...current, name: event.target.value }))
                  }
                  disabled={loading || mode === "edit"}
                  className="h-11 rounded-2xl"
                  placeholder="例如 planner"
                />
              </div>
              <div className="space-y-2">
                <Label>Provider</Label>
                {providerConfigs.length > 0 ? (
                  <Select
                    value={formData.provider}
                    onValueChange={(value) => {
                      const nextProvider =
                        providerConfigs.find((provider) => provider.name === value) ?? null
                      setFormData((current) => ({
                        ...current,
                        provider: value,
                        model: nextProvider?.models[0]?.id ?? "",
                      }))
                    }}
                    disabled={loading}
                  >
                    <SelectTrigger className="h-11 rounded-2xl">
                      <SelectValue placeholder="选择 Provider" />
                    </SelectTrigger>
                    <SelectContent>
                      {providerConfigs.map((provider) => (
                        <SelectItem key={provider.name} value={provider.name}>
                          {provider.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                ) : (
                  <Input
                    value=""
                    disabled
                    className="h-11 rounded-2xl"
                    placeholder="请先创建 Provider"
                  />
                )}
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-description">描述</Label>
              <Input
                id="agent-description"
                value={formData.description}
                onChange={(event) =>
                  setFormData((current) => ({ ...current, description: event.target.value }))
                }
                disabled={loading}
                className="h-11 rounded-2xl"
                placeholder="简短说明这个 Agent 的职责"
              />
            </div>

            <div className="grid gap-5 md:grid-cols-2">
              <div className="space-y-2">
                <Label>Model</Label>
                {selectedProvider ? (
                  <Select
                    value={formData.model}
                    onValueChange={(value) =>
                      setFormData((current) => ({ ...current, model: value }))
                    }
                    disabled={loading}
                  >
                    <SelectTrigger className="h-11 rounded-2xl">
                      <SelectValue placeholder="选择模型" />
                    </SelectTrigger>
                    <SelectContent>
                      {selectedProvider.models.map((model) => (
                        <SelectItem key={model.id} value={model.id}>
                          {model.id}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                ) : (
                  <Input
                    value=""
                    disabled
                    className="h-11 rounded-2xl"
                    placeholder="请先选择 Provider"
                  />
                )}
                {selectedModel ? (
                  <div className="text-sm text-muted-foreground">
                    上下文长度: {formatContextWindowTokens(selectedModel.context_window)}
                  </div>
                ) : null}
              </div>

              <div className="space-y-2">
                <Label htmlFor="agent-tools">允许工具</Label>
                <Input
                  id="agent-tools"
                  value={toolsValue}
                  onChange={(event) =>
                    setFormData((current) => ({
                      ...current,
                      allowed_tools: event.target.value
                        .split(",")
                        .map((tool) => tool.trim())
                        .filter(Boolean),
                    }))
                  }
                  disabled={loading}
                  className="h-11 rounded-2xl"
                  placeholder="例如 shell, write_file"
                />
              </div>
            </div>

            <div className="flex items-center gap-3 rounded-[1.4rem] border border-border/70 bg-background/70 px-4 py-3">
              <Checkbox
                id="agent-default"
                checked={formData.default}
                onCheckedChange={(checked) =>
                  setFormData((current) => ({ ...current, default: checked === true }))
                }
                disabled={loading}
              />
              <Label htmlFor="agent-default" className="cursor-pointer text-sm font-medium">
                设为默认 Agent
              </Label>
            </div>

            <div className="space-y-2">
              <Label htmlFor="agent-body">Body</Label>
              <Textarea
                id="agent-body"
                value={formData.body}
                onChange={(event) =>
                  setFormData((current) => ({ ...current, body: event.target.value }))
                }
                disabled={loading}
                className="min-h-[320px] rounded-[1.5rem]"
                placeholder="输入 Agent 的系统提示词正文..."
              />
            </div>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={loading}>
              取消
            </Button>
            <Button type="submit" disabled={loading || providerConfigs.length === 0}>
              {loading ? "保存中..." : mode === "create" ? "创建 Agent" : "保存修改"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function AgentDeleteDialog({
  agent,
  open,
  onOpenChange,
  onConfirm,
}: {
  agent: AgentFrontmatter | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => Promise<void>
}) {
  const [loading, setLoading] = useState(false)

  if (!agent) return null

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="rounded-[1.75rem] border-border/70 bg-card/95">
        <AlertDialogHeader>
          <div className="mb-3 flex items-center gap-3">
            <div className="rounded-2xl bg-destructive/12 p-3 text-destructive">
              <ShieldAlert className="size-5" />
            </div>
            <div>
              <AlertDialogTitle>删除 Agent</AlertDialogTitle>
              <AlertDialogDescription>
                删除后该角色定义与对应目录内容将从管理列表中移除。
              </AlertDialogDescription>
            </div>
          </div>
        </AlertDialogHeader>

        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4 text-sm text-muted-foreground">
          即将删除 <span className="font-semibold text-foreground">{agent.name}</span>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={loading}>取消</AlertDialogCancel>
          <AlertDialogAction
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            disabled={loading}
            onClick={(event) => {
              event.preventDefault()
              setLoading(true)
              void onConfirm().finally(() => setLoading(false))
            }}
          >
            {loading ? "删除中..." : "确认删除"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}

function AgentCard({
  agent,
  onView,
  onEdit,
  onDelete,
}: {
  agent: AgentFrontmatter
  onView: (agent: AgentFrontmatter) => void
  onEdit: (agent: AgentFrontmatter) => void
  onDelete: (agent: AgentFrontmatter) => void
}) {
  return (
    <Card
      className="group cursor-pointer rounded-[1.9rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-[0_24px_75px_-42px_rgba(37,99,235,0.28)]"
      onClick={() => onView(agent)}
    >
      <CardHeader className="flex flex-row items-start justify-between gap-4 space-y-0">
        <div className="space-y-3">
          <Badge variant="outline" className="rounded-full px-3 py-1 text-[0.72rem] uppercase tracking-[0.2em]">
            Agent
          </Badge>
          <div>
            <CardTitle className="text-2xl tracking-tight">{agent.name}</CardTitle>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {agent.description || "暂无描述"}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="icon"
            onClick={(event) => {
              event.stopPropagation()
              onEdit(agent)
            }}
          >
            <Pencil className="size-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            onClick={(event) => {
              event.stopPropagation()
              onDelete(agent)
            }}
          >
            <Trash2 className="size-4 text-destructive" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
            <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
              Provider
            </div>
            <div className="text-sm font-medium text-foreground">{agent.provider}</div>
          </div>
          <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
            <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
              Model
            </div>
            <div className="text-sm font-medium text-foreground">{agent.model}</div>
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          {agent.default ? <Badge className="rounded-full px-3 py-1">默认 Agent</Badge> : null}
          {agent.allowed_tools.length === 0 ? (
            <Badge variant="outline" className="rounded-full px-3 py-1">
              无工具限制配置
            </Badge>
          ) : (
            agent.allowed_tools.slice(0, 4).map((tool) => (
              <Badge key={tool} variant="outline" className="rounded-full px-3 py-1">
                {tool}
              </Badge>
            ))
          )}
          {agent.allowed_tools.length > 4 ? (
            <Badge variant="secondary" className="rounded-full px-3 py-1">
              +{agent.allowed_tools.length - 4}
            </Badge>
          ) : null}
        </div>
      </CardContent>
    </Card>
  )
}

export function Agents() {
  const [agents, setAgents] = useState<AgentFrontmatter[]>([])
  const [providerConfigs, setProviderConfigs] = useState<ProviderConfig[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [modalOpen, setModalOpen] = useState(false)
  const [modalMode, setModalMode] = useState<"create" | "edit">("create")
  const [selectedAgent, setSelectedAgent] = useState<AgentDetail | null>(null)
  const [detailAgentName, setDetailAgentName] = useState<string | null>(null)
  const [deleteModalOpen, setDeleteModalOpen] = useState(false)
  const [agentToDelete, setAgentToDelete] = useState<AgentFrontmatter | null>(null)

  const fetchAgents = useCallback(async () => {
    try {
      setLoading(true)
      const [agentsResponse, providersResponse] = await Promise.all([
        getAgents(),
        getProviders(),
      ])

      setAgents(agentsResponse.agents)
      setProviderConfigs(providersResponse.providers)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取 Agent 列表失败")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void fetchAgents()
  }, [fetchAgents])

  const sortedAgents = useMemo(() => {
    return [...agents].sort((a, b) => {
      if (a.default !== b.default) {
        return a.default ? -1 : 1
      }
      return a.name.localeCompare(b.name, "zh-CN")
    })
  }, [agents])

  const handleCreate = async (data: CreateAgentRequest | UpdateAgentRequest) => {
    await createAgent(data as CreateAgentRequest)
    await fetchAgents()
  }

  const handleEdit = async (data: CreateAgentRequest | UpdateAgentRequest) => {
    if (!selectedAgent) return
    await updateAgent(selectedAgent.name, data as UpdateAgentRequest)
    await fetchAgents()
  }

  const handleDelete = async () => {
    if (!agentToDelete) return

    await removeAgent(agentToDelete.name)
    if (detailAgentName === agentToDelete.name) {
      setDetailAgentName(null)
    }
    setAgentToDelete(null)
    setDeleteModalOpen(false)
    await fetchAgents()
  }

  const openCreateModal = () => {
    setModalMode("create")
    setSelectedAgent(null)
    setModalOpen(true)
  }

  const openEditModal = async (agent: AgentFrontmatter) => {
    try {
      const detail = await getAgent(agent.name)
      if (!detail) {
        throw new Error(`Agent "${agent.name}" 不存在`)
      }

      setSelectedAgent(detail)
      setModalMode("edit")
      setModalOpen(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取 Agent 详情失败")
    }
  }

  if (loading && agents.length === 0) {
    return <LoadingState title="加载 Agents" description="正在同步角色定义与模型绑定配置。" />
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Roles"
        title="Agent 管理"
        description="维护 Agent 的职责描述、模型绑定、默认角色设定，以及对应目录与系统提示词。"
        actions={
          <>
            <Badge variant="outline" className="rounded-full px-3 py-1.5">
              {agents.length} 个 Agent
            </Badge>
            <Button onClick={openCreateModal}>
              <Plus className="mr-2 size-4" />
              新建 Agent
            </Button>
          </>
        }
      />

      {error ? (
        <ErrorAlert
          message={error}
          onDismiss={() => setError(null)}
          className="rounded-[1.75rem]"
        />
      ) : null}

      {agents.length === 0 ? (
        <EmptyState
          icon={Bot}
          title="还没有 Agent"
          description="先创建一个 Agent，再为它绑定 Provider、模型和系统提示词。"
          action={
            <Button onClick={openCreateModal}>
              <Plus className="mr-2 size-4" />
              创建第一个 Agent
            </Button>
          }
        />
      ) : (
        <div className="grid gap-5 xl:grid-cols-2">
          {sortedAgents.map((agent) => (
            <AgentCard
              key={agent.name}
              agent={agent}
              onView={(nextAgent) => setDetailAgentName(nextAgent.name)}
              onEdit={(nextAgent) => void openEditModal(nextAgent)}
              onDelete={(nextAgent) => {
                setAgentToDelete(nextAgent)
                setDeleteModalOpen(true)
              }}
            />
          ))}
        </div>
      )}
      <AgentModal
        isOpen={modalOpen}
        onClose={() => setModalOpen(false)}
        onSubmit={modalMode === "create" ? handleCreate : handleEdit}
        providerConfigs={providerConfigs}
        agent={selectedAgent}
        mode={modalMode}
      />

      <AgentDeleteDialog
        agent={agentToDelete}
        open={deleteModalOpen}
        onOpenChange={setDeleteModalOpen}
        onConfirm={handleDelete}
      />

      <AgentDetailModal
        agentName={detailAgentName}
        isOpen={detailAgentName !== null}
        onClose={() => setDetailAgentName(null)}
      />
    </div>
  )
}
