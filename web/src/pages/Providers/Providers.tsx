import { useCallback, useEffect, useState } from "react"
import { CheckCircle2, KeyRound, Pencil, Plus, ShieldAlert, Trash2, Wifi } from "lucide-react"

import {
  createProvider,
  deleteProvider,
  getProviders,
  testSavedProviderConnection,
  updateProvider,
} from "@/api"
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
import type { CompatibleApi, ProviderConfig, ProviderModelConfig } from "@/types"

interface ProviderModelFormState {
  rowKey: string
  id: string
  context_window: string
}

interface ProviderFormState {
  name: string
  api_key: string
  base_url: string
  compatible_api: CompatibleApi
  models: ProviderModelFormState[]
}

let providerModelRowKeyCounter = 0

function createProviderModelRowKey(): string {
  providerModelRowKeyCounter += 1
  return `provider-model-${providerModelRowKeyCounter}`
}

function createEmptyModelFormState(): ProviderModelFormState {
  return {
    rowKey: createProviderModelRowKey(),
    id: "",
    context_window: "",
  }
}

function toFormState(provider?: ProviderConfig | null): ProviderFormState {
  if (!provider) {
    return {
      name: "",
      api_key: "",
      base_url: "",
      compatible_api: "openai",
      models: [createEmptyModelFormState()],
    }
  }

  return {
    name: provider.name,
    api_key: provider.api_key,
    base_url: provider.base_url,
    compatible_api: provider.compatible_api,
    models: provider.models.map((model) => ({
      rowKey: createProviderModelRowKey(),
      id: model.id,
      context_window: model.context_window.toString(),
    })),
  }
}

function maskApiKey(value: string): string {
  if (!value) return "未配置"
  if (value.length <= 8) return "••••••••"
  return `${value.slice(0, 4)}••••${value.slice(-4)}`
}

function getCompatibleApiLabel(compatibleApi: CompatibleApi): string {
  return compatibleApi === "anthropic" ? "Anthropic Compatible" : "OpenAI Compatible"
}

function formatContextWindowTokens(value: number): string {
  return `${value.toLocaleString("en-US")} tokens`
}

interface ProviderModalProps {
  isOpen: boolean
  mode: "create" | "edit"
  provider: ProviderConfig | null
  onClose: () => void
  onSubmit: (provider: ProviderConfig) => Promise<void>
}

function ProviderModal({
  isOpen,
  mode,
  provider,
  onClose,
  onSubmit,
}: ProviderModalProps) {
  const [formState, setFormState] = useState<ProviderFormState>(toFormState())
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!isOpen) return
    setFormState(toFormState(provider))
    setError(null)
  }, [isOpen, provider])

  const validateForm = (): ProviderConfig | null => {
    if (!formState.name.trim()) {
      setError("Provider 名称不能为空")
      return null
    }
    if (!formState.api_key.trim()) {
      setError("API Key 不能为空")
      return null
    }
    if (!formState.base_url.trim()) {
      setError("Base URL 不能为空")
      return null
    }
    if (formState.name.includes("/") || formState.name.includes("\\")) {
      setError("Provider 名称不能包含路径分隔符")
      return null
    }
    if (formState.models.length === 0) {
      setError("至少需要配置一个模型")
      return null
    }

    const models: ProviderModelConfig[] = []
    for (const [index, model] of formState.models.entries()) {
      const modelId = model.id.trim()
      if (!modelId) {
        setError(`模型 #${index + 1} 的 ID 不能为空`)
        return null
      }

      const contextWindowTokens = Number.parseInt(model.context_window.trim(), 10)
      if (!Number.isInteger(contextWindowTokens) || contextWindowTokens <= 0) {
        setError(`模型 ${modelId} 的上下文长度必须是正整数`)
        return null
      }

      if (models.some((existingModel) => existingModel.id === modelId)) {
        setError(`模型 ${modelId} 重复了`)
        return null
      }

      models.push({
        id: modelId,
        context_window: contextWindowTokens,
      })
    }

    return {
      name: formState.name.trim(),
      api_key: formState.api_key.trim(),
      base_url: formState.base_url.trim(),
      compatible_api: formState.compatible_api,
      models,
    }
  }

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    const validatedProvider = validateForm()
    if (!validatedProvider) {
      return
    }

    setLoading(true)
    setError(null)
    try {
      await onSubmit(validatedProvider)
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存 Provider 失败")
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="grid max-h-[calc(100vh-2rem)] grid-rows-[auto_minmax(0,1fr)] overflow-hidden rounded-[1.75rem] border-border/70 bg-card/95 sm:max-w-[760px]">
        <DialogHeader>
          <DialogTitle className="text-2xl tracking-tight">
            {mode === "create" ? "创建 Provider" : "编辑 Provider"}
          </DialogTitle>
          <DialogDescription>
            统一维护 Provider 名称、认证信息、Base URL、兼容 API 和模型列表。
          </DialogDescription>
        </DialogHeader>

        <form className="grid min-h-0 grid-rows-[minmax(0,1fr)_auto]" onSubmit={handleSubmit}>
          <div className="min-h-0 space-y-5 overflow-y-auto pr-1">
            {error ? (
              <div className="rounded-2xl border border-destructive/25 bg-destructive/5 px-4 py-3 text-sm text-destructive">
                {error}
              </div>
            ) : null}

            <div className="space-y-2">
              <Label>Provider 名称</Label>
              <Input
                value={formState.name}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, name: event.target.value }))
                }
                disabled={loading || mode === "edit"}
                className="h-11 rounded-2xl"
                placeholder="例如 openai-main"
              />
            </div>

            <div className="space-y-2">
              <Label>API Key</Label>
              <Input
                type="password"
                value={formState.api_key}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, api_key: event.target.value }))
                }
                disabled={loading}
                className="h-11 rounded-2xl"
                placeholder="输入 Provider API Key"
              />
            </div>

            <div className="grid gap-5 md:grid-cols-2">
              <div className="space-y-2">
                <Label>Base URL</Label>
                <Input
                  type="url"
                  value={formState.base_url}
                  onChange={(event) =>
                    setFormState((current) => ({ ...current, base_url: event.target.value }))
                  }
                  disabled={loading}
                  className="h-11 rounded-2xl"
                  placeholder="https://example.com/v1"
                />
              </div>

              <div className="space-y-2">
                <Label>兼容 API</Label>
                <Select
                  value={formState.compatible_api}
                  onValueChange={(value) =>
                    setFormState((current) => ({
                      ...current,
                      compatible_api: value as CompatibleApi,
                    }))
                  }
                  disabled={loading}
                >
                  <SelectTrigger className="h-11 rounded-2xl">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="openai">OpenAI</SelectItem>
                    <SelectItem value="anthropic">Anthropic</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="space-y-3 rounded-[1.5rem] border border-border/70 bg-background/60 p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="text-sm font-semibold text-foreground">模型列表</div>
                  <div className="text-sm text-muted-foreground">
                    每个模型都需要模型 ID 和上下文长度。
                  </div>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    setFormState((current) => ({
                      ...current,
                      models: [...current.models, createEmptyModelFormState()],
                    }))
                  }
                  disabled={loading}
                >
                  <Plus className="mr-2 size-4" />
                  添加模型
                </Button>
              </div>

              <div className="space-y-3">
                {formState.models.map((model, index) => (
                  <div
                    key={model.rowKey}
                    className="grid gap-3 rounded-[1.25rem] border border-border/70 bg-card/70 p-4 md:grid-cols-[minmax(0,1fr)_220px_auto]"
                  >
                    <div className="space-y-2">
                      <Label>模型 ID</Label>
                      <Input
                        value={model.id}
                        onChange={(event) =>
                          setFormState((current) => ({
                            ...current,
                            models: current.models.map((currentModel, currentIndex) =>
                              currentIndex === index
                                ? { ...currentModel, id: event.target.value }
                                : currentModel
                            ),
                          }))
                        }
                        disabled={loading}
                        className="h-11 rounded-2xl"
                        placeholder="例如 gpt-4.1-mini"
                      />
                    </div>

                    <div className="space-y-2">
                      <Label>上下文长度</Label>
                      <Input
                        type="number"
                        min="1"
                        step="1"
                        value={model.context_window}
                        onChange={(event) =>
                          setFormState((current) => ({
                            ...current,
                            models: current.models.map((currentModel, currentIndex) =>
                              currentIndex === index
                                ? {
                                    ...currentModel,
                                    context_window: event.target.value,
                                  }
                                : currentModel
                            ),
                          }))
                        }
                        disabled={loading}
                        className="h-11 rounded-2xl"
                        placeholder="例如 128000"
                      />
                    </div>

                    <div className="flex items-end">
                      <Button
                        type="button"
                        variant="outline"
                        size="icon"
                        onClick={() =>
                          setFormState((current) => ({
                            ...current,
                            models: current.models.filter((_, currentIndex) => currentIndex !== index),
                          }))
                        }
                        disabled={loading}
                      >
                        <Trash2 className="size-4 text-destructive" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <DialogFooter className="pt-4">
            <Button type="button" variant="outline" onClick={onClose} disabled={loading}>
              取消
            </Button>
            <Button type="submit" disabled={loading}>
              {loading ? "保存中..." : mode === "create" ? "创建 Provider" : "保存修改"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function ProviderDeleteDialog({
  provider,
  open,
  onOpenChange,
  onConfirm,
}: {
  provider: ProviderConfig | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => Promise<void>
}) {
  const [loading, setLoading] = useState(false)

  if (!provider) return null

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="rounded-[1.75rem] border-border/70 bg-card/95">
        <AlertDialogHeader>
          <div className="mb-3 flex items-center gap-3">
            <div className="rounded-2xl bg-destructive/12 p-3 text-destructive">
              <ShieldAlert className="size-5" />
            </div>
            <div>
              <AlertDialogTitle>删除 Provider</AlertDialogTitle>
              <AlertDialogDescription>
                此操作不可撤销，请确认当前没有 Agent 正在依赖它。
              </AlertDialogDescription>
            </div>
          </div>
        </AlertDialogHeader>
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4 text-sm text-muted-foreground">
          即将删除 <span className="font-semibold text-foreground">{provider.name}</span>
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

function ProviderConnectionTestDialog({
  provider,
  open,
  onOpenChange,
  onTest,
}: {
  provider: ProviderConfig | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onTest: (provider: ProviderConfig, model: string) => Promise<{ latencyMs: number }>
}) {
  const [testLoading, setTestLoading] = useState(false)
  const [testModel, setTestModel] = useState("")
  const [testResult, setTestResult] = useState<string | null>(null)
  const [testError, setTestError] = useState<string | null>(null)

  useEffect(() => {
    if (!open || !provider) return
    setTestModel(provider.models[0]?.id ?? "")
    setTestResult(null)
    setTestError(null)
  }, [open, provider])

  if (!provider) return null

  const selectedModel = provider.models.find((model) => model.id === testModel) ?? null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="rounded-[1.75rem] border-border/70 bg-card/95 sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle className="text-2xl tracking-tight">测试 Provider 连接</DialogTitle>
          <DialogDescription>
            使用 Provider 中已配置的模型发起一次真实请求，验证连接与延迟。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-5">
          <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
            <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
              Provider
            </div>
            <div className="text-sm font-medium text-foreground">{provider.name}</div>
          </div>

          <div className="space-y-2">
            <Label>测试模型</Label>
            <Select value={testModel} onValueChange={setTestModel} disabled={testLoading}>
              <SelectTrigger className="h-11 rounded-2xl">
                <SelectValue placeholder="选择测试模型" />
              </SelectTrigger>
              <SelectContent>
                {provider.models.map((model) => (
                  <SelectItem key={model.id} value={model.id}>
                    {model.id}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {selectedModel ? (
              <div className="text-sm text-muted-foreground">
                当前模型上下文长度: {formatContextWindowTokens(selectedModel.context_window)}
              </div>
            ) : null}
          </div>

          {testResult ? (
            <div className="flex items-start gap-2 rounded-[1.4rem] border border-emerald-500/25 bg-emerald-500/5 p-4 text-sm text-emerald-700">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
              <span>{testResult}</span>
            </div>
          ) : null}
          {testError ? (
            <div className="rounded-[1.4rem] border border-destructive/25 bg-destructive/5 p-4 text-sm text-destructive">
              {testError}
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={testLoading}>
            关闭
          </Button>
          <Button
            type="button"
            onClick={() => {
              if (!testModel) {
                setTestError("请选择测试模型")
                setTestResult(null)
                return
              }
              setTestLoading(true)
              setTestResult(null)
              setTestError(null)
              void onTest(provider, testModel)
                .then((result) => setTestResult(`测试成功 (${result.latencyMs} ms)`))
                .catch((err) =>
                  setTestError(err instanceof Error ? err.message : "测试 Provider 连接失败")
                )
                .finally(() => setTestLoading(false))
            }}
            disabled={testLoading}
          >
            <Wifi className="mr-2 size-4" />
            {testLoading ? "测试中..." : "开始测试"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ProviderCard({
  provider,
  onEdit,
  onDelete,
  onOpenTest,
}: {
  provider: ProviderConfig
  onEdit: (provider: ProviderConfig) => void
  onDelete: (provider: ProviderConfig) => void
  onOpenTest: (provider: ProviderConfig) => void
}) {
  return (
    <Card className="rounded-[1.8rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl">
      <CardHeader className="flex flex-row items-start justify-between gap-4 space-y-0">
        <div className="space-y-3">
          <Badge
            variant="outline"
            className="rounded-full px-3 py-1 text-[0.72rem] uppercase tracking-[0.2em]"
          >
            {getCompatibleApiLabel(provider.compatible_api)}
          </Badge>
          <div>
            <CardTitle className="text-2xl tracking-tight">{provider.name}</CardTitle>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {provider.base_url}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="icon" onClick={() => onOpenTest(provider)}>
            <Wifi className="size-4" />
          </Button>
          <Button variant="outline" size="icon" onClick={() => onEdit(provider)}>
            <Pencil className="size-4" />
          </Button>
          <Button variant="outline" size="icon" onClick={() => onDelete(provider)}>
            <Trash2 className="size-4 text-destructive" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
          <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            API Key
          </div>
          <div className="font-mono text-sm text-foreground">{maskApiKey(provider.api_key)}</div>
        </div>
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
          <div className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Models
          </div>
          <div className="space-y-2">
            {provider.models.map((model) => (
              <div
                key={model.id}
                className="flex items-center justify-between gap-3 rounded-2xl border border-border/70 bg-card/70 px-3 py-2 text-sm"
              >
                <span className="font-medium text-foreground">{model.id}</span>
                <span className="text-muted-foreground">
                  {formatContextWindowTokens(model.context_window)}
                </span>
              </div>
            ))}
          </div>
        </div>
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
          <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Compatible API
          </div>
          <div className="text-sm text-foreground">{provider.compatible_api}</div>
        </div>
      </CardContent>
    </Card>
  )
}

export function Providers() {
  const [providers, setProviders] = useState<ProviderConfig[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [modalMode, setModalMode] = useState<"create" | "edit">("create")
  const [modalOpen, setModalOpen] = useState(false)
  const [testModalOpen, setTestModalOpen] = useState(false)
  const [deleteModalOpen, setDeleteModalOpen] = useState(false)
  const [selectedProvider, setSelectedProvider] = useState<ProviderConfig | null>(null)

  const fetchProviders = useCallback(async () => {
    try {
      setLoading(true)
      const response = await getProviders()
      setProviders(response.providers)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取 Provider 列表失败")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void fetchProviders()
  }, [fetchProviders])

  const handleCreate = async (provider: ProviderConfig) => {
    await createProvider(provider)
    await fetchProviders()
  }

  const handleEdit = async (provider: ProviderConfig) => {
    if (!selectedProvider) return
    await updateProvider(selectedProvider.name, provider)
    await fetchProviders()
  }

  const handleDelete = async () => {
    if (!selectedProvider) return
    await deleteProvider(selectedProvider.name)
    await fetchProviders()
    setSelectedProvider(null)
    setDeleteModalOpen(false)
  }

  const handleTestSavedConnection = async (provider: ProviderConfig, model: string) => {
    const response = await testSavedProviderConnection(provider.name, model)
    return { latencyMs: response.latency_ms }
  }

  if (loading && providers.length === 0) {
    return (
      <LoadingState
        title="加载 Provider"
        description="正在同步 Provider 配置和兼容 API 信息。"
      />
    )
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Configuration"
        title="Provider 配置"
        description="维护统一的 ProviderConfig，包括名称、认证信息、Base URL、兼容 API 和模型列表。"
        actions={
          <>
            <Badge variant="outline" className="rounded-full px-3 py-1.5">
              {providers.length} 个 Provider
            </Badge>
            <Button
              onClick={() => {
                setModalMode("create")
                setSelectedProvider(null)
                setModalOpen(true)
              }}
            >
              <Plus className="mr-2 size-4" />
              新建 Provider
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

      {providers.length === 0 ? (
        <EmptyState
          icon={KeyRound}
          title="还没有 Provider"
          description="先配置 Provider 和模型列表，再去 Agent 页面绑定模型、提示词和默认角色。"
          action={
            <Button
              onClick={() => {
                setModalMode("create")
                setSelectedProvider(null)
                setModalOpen(true)
              }}
            >
              <Plus className="mr-2 size-4" />
              创建第一个 Provider
            </Button>
          }
        />
      ) : (
        <div className="grid gap-5 xl:grid-cols-2">
          {providers.map((provider) => (
            <ProviderCard
              key={provider.name}
              provider={provider}
              onOpenTest={(nextProvider) => {
                setSelectedProvider(nextProvider)
                setTestModalOpen(true)
              }}
              onEdit={(nextProvider) => {
                setModalMode("edit")
                setSelectedProvider(nextProvider)
                setModalOpen(true)
              }}
              onDelete={(nextProvider) => {
                setSelectedProvider(nextProvider)
                setDeleteModalOpen(true)
              }}
            />
          ))}
        </div>
      )}
      <ProviderModal
        isOpen={modalOpen}
        mode={modalMode}
        provider={selectedProvider}
        onClose={() => setModalOpen(false)}
        onSubmit={modalMode === "create" ? handleCreate : handleEdit}
      />

      <ProviderConnectionTestDialog
        provider={selectedProvider}
        open={testModalOpen}
        onOpenChange={setTestModalOpen}
        onTest={handleTestSavedConnection}
      />

      <ProviderDeleteDialog
        provider={selectedProvider}
        open={deleteModalOpen}
        onOpenChange={setDeleteModalOpen}
        onConfirm={handleDelete}
      />
    </div>
  )
}
