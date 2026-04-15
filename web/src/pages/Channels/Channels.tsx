import { useCallback, useEffect, useState } from "react"
import { Cable, Pencil, Plus, Trash2 } from "lucide-react"

import {
  createChannel,
  deleteChannel,
  getChannels,
  updateChannel,
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
import type { ChannelConfig, ChannelName } from "@/types"

type ChannelFormState = {
  name: ChannelName
  bot_token: string
  user_id: string
}

const channelOptions: { value: ChannelName; label: string; hint: string }[] = [
  { value: "telegram", label: "Telegram", hint: "通过 Telegram Bot 接收用户消息" },
  { value: "wechat", label: "Wechat", hint: "通过 Wechat 渠道接收用户消息" },
]

function toFormState(channel?: ChannelConfig | null): ChannelFormState {
  if (!channel) {
    return {
      name: "telegram",
      bot_token: "",
      user_id: "",
    }
  }

  return {
    name: channel.name,
    bot_token: channel.bot_token,
    user_id: String(channel.user_id),
  }
}

function toChannelConfig(form: ChannelFormState): ChannelConfig {
  if (form.name === "telegram") {
    return {
      name: "telegram",
      bot_token: form.bot_token.trim(),
      user_id: Number(form.user_id),
    }
  }

  return {
    name: "wechat",
    bot_token: form.bot_token.trim(),
    user_id: form.user_id.trim(),
  }
}

function maskSecret(value: string): string {
  if (!value) return "未配置"
  if (value.length <= 8) return "••••••••"
  return `${value.slice(0, 4)}••••${value.slice(-4)}`
}

function ChannelModal({
  isOpen,
  mode,
  channel,
  onClose,
  onSubmit,
}: {
  isOpen: boolean
  mode: "create" | "edit"
  channel: ChannelConfig | null
  onClose: () => void
  onSubmit: (channel: ChannelConfig) => Promise<void>
}) {
  const [formState, setFormState] = useState<ChannelFormState>(toFormState())
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!isOpen) return
    setFormState(toFormState(channel))
    setError(null)
  }, [channel, isOpen])

  const currentOption = channelOptions.find((option) => option.value === formState.name)

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!formState.bot_token.trim()) {
      setError("Bot Token 不能为空")
      return
    }
    if (!formState.user_id.trim()) {
      setError("User ID 不能为空")
      return
    }
    if (
      formState.name === "telegram" &&
      (!/^\d+$/.test(formState.user_id) || Number(formState.user_id) <= 0)
    ) {
      setError("Telegram User ID 必须是正整数")
      return
    }

    setLoading(true)
    setError(null)
    try {
      await onSubmit(toChannelConfig(formState))
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存 Channel 失败")
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="rounded-[1.75rem] border-border/70 bg-card/95 sm:max-w-[640px]">
        <DialogHeader>
          <DialogTitle className="text-2xl tracking-tight">
            {mode === "create" ? "创建 Channel" : "编辑 Channel"}
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
              <Label>Channel 类型</Label>
              <Select
                value={formState.name}
                onValueChange={(value) =>
                  setFormState((current) => ({
                    ...current,
                    name: value as ChannelName,
                    user_id: "",
                  }))
                }
                disabled={loading || mode === "edit"}
              >
                <SelectTrigger className="h-11 rounded-2xl">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {channelOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>Bot Token</Label>
              <Input
                type="password"
                value={formState.bot_token}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, bot_token: event.target.value }))
                }
                disabled={loading}
                className="h-11 rounded-2xl"
                placeholder="输入 Bot Token"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label>{formState.name === "telegram" ? "Telegram User ID" : "Wechat User ID"}</Label>
            <Input
              type={formState.name === "telegram" ? "number" : "text"}
              value={formState.user_id}
              onChange={(event) =>
                setFormState((current) => ({ ...current, user_id: event.target.value }))
              }
              disabled={loading}
              className="h-11 rounded-2xl"
              placeholder={formState.name === "telegram" ? "例如 123456789" : "例如 wxid_xxx"}
            />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={loading}>
              取消
            </Button>
            <Button type="submit" disabled={loading}>
              {loading ? "保存中..." : mode === "create" ? "创建 Channel" : "保存修改"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function ChannelDeleteDialog({
  channel,
  open,
  onOpenChange,
  onConfirm,
}: {
  channel: ChannelConfig | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => Promise<void>
}) {
  const [loading, setLoading] = useState(false)

  if (!channel) return null

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="rounded-[1.75rem] border-border/70 bg-card/95">
        <AlertDialogHeader>
          <AlertDialogTitle>删除 Channel</AlertDialogTitle>
          <AlertDialogDescription>
            删除后系统将不再从该消息入口接收用户请求。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4 text-sm text-muted-foreground">
          即将删除 <span className="font-semibold text-foreground">{channel.name}</span>
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

function ChannelCard({
  channel,
  onEdit,
  onDelete,
}: {
  channel: ChannelConfig
  onEdit: (channel: ChannelConfig) => void
  onDelete: (channel: ChannelConfig) => void
}) {
  return (
    <Card className="rounded-[1.8rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl">
      <CardHeader className="flex flex-row items-start justify-between gap-4 space-y-0">
        <div className="space-y-3">
          <Badge variant="outline" className="rounded-full px-3 py-1 text-[0.72rem] uppercase tracking-[0.2em]">
            Channel
          </Badge>
          <div>
            <CardTitle className="text-2xl capitalize tracking-tight">{channel.name}</CardTitle>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {channel.name === "telegram" ? "Telegram Bot 对话入口" : "Wechat 消息接入入口"}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="icon" onClick={() => onEdit(channel)}>
            <Pencil className="size-4" />
          </Button>
          <Button variant="outline" size="icon" onClick={() => onDelete(channel)}>
            <Trash2 className="size-4 text-destructive" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="grid gap-4 md:grid-cols-2">
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
          <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Bot Token
          </div>
          <div className="min-w-0 break-all font-mono text-sm text-foreground">
            {maskSecret(channel.bot_token)}
          </div>
        </div>
        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4">
          <div className="mb-1 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            User ID
          </div>
          <div className="min-w-0 break-all font-mono text-sm text-foreground">
            {String(channel.user_id)}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export function Channels() {
  const [channels, setChannels] = useState<ChannelConfig[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [modalMode, setModalMode] = useState<"create" | "edit">("create")
  const [modalOpen, setModalOpen] = useState(false)
  const [deleteModalOpen, setDeleteModalOpen] = useState(false)
  const [selectedChannel, setSelectedChannel] = useState<ChannelConfig | null>(null)

  const fetchChannels = useCallback(async () => {
    try {
      setLoading(true)
      const response = await getChannels()
      setChannels(response.channels)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取 Channel 列表失败")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void fetchChannels()
  }, [fetchChannels])

  const handleCreate = async (channel: ChannelConfig) => {
    await createChannel(channel)
    await fetchChannels()
  }

  const handleEdit = async (channel: ChannelConfig) => {
    if (!selectedChannel) return
    await updateChannel(selectedChannel.name, channel)
    await fetchChannels()
  }

  const handleDelete = async () => {
    if (!selectedChannel) return
    await deleteChannel(selectedChannel.name)
    await fetchChannels()
    setSelectedChannel(null)
    setDeleteModalOpen(false)
  }

  if (loading && channels.length === 0) {
    return (
      <LoadingState
        title="加载 Channel"
        description="正在同步消息渠道接入配置。"
      />
    )
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Inbound"
        title="Channel 管理"
        description="管理消息接入渠道，让系统可以从 Telegram 或 Wechat 等入口接收新的用户任务。"
        actions={
          <>
            <Badge variant="outline" className="rounded-full px-3 py-1.5">
              {channels.length} 个 Channel
            </Badge>
            <Button onClick={() => {
              setModalMode("create")
              setSelectedChannel(null)
              setModalOpen(true)
            }}>
              <Plus className="mr-2 size-4" />
              新建 Channel
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

      {channels.length === 0 ? (
        <EmptyState
          icon={Cable}
          title="还没有 Channel"
          description="配置 Channel 后，Babata 才能从外部消息入口创建新的根任务。"
          action={
            <Button onClick={() => {
              setModalMode("create")
              setSelectedChannel(null)
              setModalOpen(true)
            }}>
              <Plus className="mr-2 size-4" />
              创建第一个 Channel
            </Button>
          }
        />
      ) : (
        <div className="grid gap-5 xl:grid-cols-2">
          {channels.map((channel) => (
            <ChannelCard
              key={channel.name}
              channel={channel}
              onEdit={(nextChannel) => {
                setModalMode("edit")
                setSelectedChannel(nextChannel)
                setModalOpen(true)
              }}
              onDelete={(nextChannel) => {
                setSelectedChannel(nextChannel)
                setDeleteModalOpen(true)
              }}
            />
          ))}
        </div>
      )}
      <ChannelModal
        isOpen={modalOpen}
        mode={modalMode}
        channel={selectedChannel}
        onClose={() => setModalOpen(false)}
        onSubmit={modalMode === "create" ? handleCreate : handleEdit}
      />

      <ChannelDeleteDialog
        channel={selectedChannel}
        open={deleteModalOpen}
        onOpenChange={setDeleteModalOpen}
        onConfirm={handleDelete}
      />
    </div>
  )
}
