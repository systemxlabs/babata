import { useMemo, useState } from "react"
import { LoaderCircle, RotateCcw } from "lucide-react"

import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Textarea } from "@/components/ui/textarea"

interface TaskRelaunchDialogProps {
  taskId: string
  taskDescription: string
  onSubmit: (taskId: string, reason: string) => Promise<void>
}

export function TaskRelaunchDialog({
  taskId,
  taskDescription,
  onSubmit,
}: TaskRelaunchDialogProps) {
  const [open, setOpen] = useState(false)
  const [reason, setReason] = useState("")
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const remaining = useMemo(() => 400 - reason.length, [reason.length])

  const reset = () => {
    setReason("")
    setError(null)
    setSubmitting(false)
  }

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen)
    if (!nextOpen) {
      reset()
    }
  }

  const handleSubmit = async () => {
    const content = reason.trim()
    if (!content) {
      setError("请输入 relaunch 原因。")
      return
    }

    setSubmitting(true)
    setError(null)

    try {
      await onSubmit(taskId, content)
      handleOpenChange(false)
    } catch (submitError) {
      setError(submitError instanceof Error ? submitError.message : "重启任务失败")
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <Button
        variant="outline"
        size="sm"
        className="rounded-full"
        onClick={() => setOpen(true)}
        title="重新拉起任务"
      >
        <RotateCcw className="mr-1.5 size-3.5" />
        Relaunch
      </Button>

      <DialogContent className="rounded-[1.7rem] border-border/70 bg-card/95 p-0 shadow-[0_24px_90px_-40px_rgba(15,23,42,0.48)] backdrop-blur-2xl sm:max-w-2xl">
        <DialogHeader className="space-y-4 px-6 pt-6">
          <div>
            <DialogTitle className="text-xl tracking-tight">重新拉起任务</DialogTitle>
            <DialogDescription className="mt-2 leading-6">
              这会触发当前任务重新执行。请提供明确原因，帮助 agent 从现有任务目录状态继续。
            </DialogDescription>
          </div>
          <div className="rounded-[1.3rem] border border-border/70 bg-background/70 p-4">
            <div className="text-xs font-semibold uppercase tracking-[0.22em] text-muted-foreground">
              Task
            </div>
            <div className="mt-2 text-sm font-medium leading-6 text-foreground">
              {taskDescription}
            </div>
            <div className="mt-2 break-all font-mono text-xs text-muted-foreground">
              {taskId}
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4 px-6 pb-6 pt-2">
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-3">
              <label htmlFor={`relaunch-reason-${taskId}`} className="text-sm font-medium text-foreground">
                Relaunch 原因
              </label>
              <span className="text-xs text-muted-foreground">{remaining} 字符剩余</span>
            </div>
            <Textarea
              id={`relaunch-reason-${taskId}`}
              value={reason}
              onChange={(event) => {
                setReason(event.target.value.slice(0, 400))
                if (error) {
                  setError(null)
                }
              }}
              placeholder="例如：上次执行被错误配置打断，请基于当前目录内容重新继续。"
              className="min-h-[160px] resize-y rounded-2xl border-border/70 bg-background/80 px-4 py-3 leading-6"
              disabled={submitting}
            />
          </div>

          {error ? (
            <Alert variant="destructive" className="rounded-2xl border-destructive/20">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          ) : null}
        </div>

        <DialogFooter className="rounded-b-[1.7rem] border-t border-border/70 bg-background/60">
          <Button
            variant="outline"
            className="rounded-2xl"
            onClick={() => handleOpenChange(false)}
            disabled={submitting}
          >
            取消
          </Button>
          <Button
            className="rounded-2xl"
            onClick={() => void handleSubmit()}
            disabled={submitting}
          >
            {submitting ? (
              <>
                <LoaderCircle className="mr-2 size-4 animate-spin" />
                提交中...
              </>
            ) : (
              <>
                <RotateCcw className="mr-2 size-4" />
                确认 relaunch
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
