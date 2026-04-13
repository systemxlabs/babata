import { useCallback, useEffect, useState } from "react"
import { KeyRound, Pencil, Plus, ShieldAlert, Trash2 } from "lucide-react"

import {
  createProvider,
  deleteProvider,
  listProviders,
  updateProvider,
} from "@/api"
import { EmptyState } from "@/components/empty-state"
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
import type { CompatibleApi, ProviderConfig, ProviderName } from "@/types"

type ProviderFormState = {
  name: ProviderName
  api_key: string
  base_url: string
  compatible_api: CompatibleApi
}

const providerOptions: { value: ProviderName; label: string; hint: string }[] = [
  { value: "openai", label: "OpenAI", hint: "官方 OpenAI 兼容配置" },
  { value: "anthropic", label: "Anthropic", hint: "Claude 官方接口" },
  { value: "deepseek", label: "DeepSeek", hint: "DeepSeek 官方接口" },
  { value: "kimi", label: "Kimi", hint: "月之暗面 Kimi 接口" },
  { value: "moonshot", label: "Moonshot", hint: "Moonshot 官方接口" },
  { value: "minimax", label: "MiniMax", hint: "MiniMax 官方接口" },
  { value: "custom", label: "Custom", hint: "自定义兼容 OpenAI/Anthropic 接口" },
]

function toFormState(provider?: ProviderConfig | null): ProviderFormState {
  if (!provider) {
    return {
      name: "openai",
      api_key: "",
      base_url: "",
      compatible_api: "openai",
    }
  }

  if (provider.name === "custom") {
    return {
      name: provider.name,
      api_key: provider.api_key,
      base_url: provider.base_url,
      compatible_api: provider.compatible_api,
    }
  }

  return {
    name: provider.name,
    api_key: provider.api_key,
    base_url: "",
    compatible_api: "openai",
  }
}

function toProviderConfig(form: ProviderFormState): ProviderConfig {
  if (form.name === "custom") {
    return {
      name: "custom",
      api_key: form.api_key.trim(),
      base_url: form.base_url.trim(),
      compatible_api: form.compatible_api,
    }
  }

  return {
    name: form.name,
    api_key: form.api_key.trim(),
  }
}

function maskApiKey(value: string): string {
  if (!value) return "未配置"
  if (value.length <= 8) return "••••••••"
  return `${value.slice(0, 4)}••••${value.slice(-4)}`
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

  const isCustom = formState.name === "custom"
  const currentOption = providerOptions.find((option) => option.value === formState.name)

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!formState.api_key.trim()) {
      setError("API Key 不能为空")
      return
    }
    if (isCustom && !formState.base_url.trim()) {
      setError("自定义 Provider 需要填写 Base URL")
      return
    }

    setLoading(true)
    setError(null)
    try {
      await onSubmit(toProviderConfig(formState))
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存 Provider 失败")
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="rounded-[1.75rem] border-border/70 bg-card/95 sm:max-w-[640px]">
        <DialogHeader>
          <DialogTitle className="text-2xl tracking-tight">
            {mode === "create" ? "创建 Provider" : "编辑 Provider"}
          </DialogTitle>
          <DialogDescription>{currentOption?.hint}</DialogDescription>
        </DialogHeader>

        <form className="space-y-5" onSubmit={handleSubmit}>
          {error ? (
            <div className="rounded-2xl border border-destructive/25 bg-destructive/5 px-4 py-3 text-sm text-destructive">
              {error}
            </div>
          ) : null}

          <div className="grid gap-5 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Provider 类型</Label>
              <Select
                value={formState.name}
                onValueChange={(value) =>
                  setFormState((current) => ({
                    ...current,
                    name: value as ProviderName,
                    base_url: value === "custom" ? current.base_url : "",
                  }))
                }
                disabled={loading || mode === "edit"}
              >
                <SelectTrigger className="h-11 rounded-2xl">
                  <SelectValue placeholder="选择 Provider 类型" />
                </SelectTrigger>
                <SelectContent>
                  {providerOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
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
          </div>

          {isCustom ? (
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
          ) : null}

          <DialogFooter className="gap-2 sm:gap-0">
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

function ProviderCard({
  provider,
  onEdit,
  onDelete,
}: {
  provider: ProviderConfig
  onEdit: (provider: ProviderConfig) => void
  onDelete: (provider: ProviderConfig) => void
}) {
  return (
    <Card className="rounded-[1.8rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl">
      <CardHeader className="flex flex-row items-start justify-between gap-4 space-y-0">
        <div className="space-y-3">
          <Badge variant="outline" className="rounded-full px-3 py-1 text-[0.72rem] uppercase tracking-[0.2em]">
            Provider
          </Badge>
          <div>
            <CardTitle className="text-2xl capitalize tracking-tight">{provider.name}</CardTitle>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {provider.name === "custom" ? "自定义兼容 Provider" : "内建 Provider 配置"}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
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
        {provider.name === "custom" ? (
          <div className="grid gap-4 md:grid-cols-2">
            <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
              <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                Base URL
              </div>
              <div className="text-sm text-foreground">{provider.base_url}</div>
            </div>
            <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
              <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                Compatible API
              </div>
              <div className="text-sm text-foreground">{provider.compatible_api}</div>
            </div>
          </div>
        ) : null}
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
  const [deleteModalOpen, setDeleteModalOpen] = useState(false)
  const [selectedProvider, setSelectedProvider] = useState<ProviderConfig | null>(null)

  const fetchProviders = useCallback(async () => {
    try {
      setLoading(true)
      const providerList = await listProviders()
      setProviders(providerList)
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

  if (loading && providers.length === 0) {
    return (
      <LoadingState
        title="加载 Provider"
        description="正在同步模型供应商配置与兼容接口信息。"
      />
    )
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Configuration"
        title="Provider 管理"
        description="维护模型服务凭据、自定义兼容接口和系统默认供应商接入方式。"
        actions={
          <>
            <Badge variant="outline" className="rounded-full px-3 py-1.5">
              {providers.length} 个 Provider
            </Badge>
            <Button onClick={() => {
              setModalMode("create")
              setSelectedProvider(null)
              setModalOpen(true)
            }}>
              <Plus className="mr-2 size-4" />
              新建 Provider
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

      {providers.length === 0 ? (
        <EmptyState
          icon={KeyRound}
          title="还没有 Provider"
          description="先配置 Provider，再去 Agent 页面绑定模型、提示词和默认角色。"
          action={
            <Button onClick={() => {
              setModalMode("create")
              setSelectedProvider(null)
              setModalOpen(true)
            }}>
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

      <ProviderDeleteDialog
        provider={selectedProvider}
        open={deleteModalOpen}
        onOpenChange={setDeleteModalOpen}
        onConfirm={handleDelete}
      />
    </div>
  )
}
