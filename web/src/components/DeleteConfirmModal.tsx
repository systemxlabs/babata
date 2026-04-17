import { useEffect, useState } from "react"
import { AlertTriangle, LoaderCircle } from "lucide-react"

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

interface DeleteConfirmModalProps {
  isOpen: boolean
  taskId: string
  taskDescription?: string
  isRootTask?: boolean
  onConfirm: () => Promise<void>
  onCancel: () => void
}

export function DeleteConfirmModal({
  isOpen,
  taskId,
  taskDescription,
  isRootTask = true,
  onConfirm,
  onCancel,
}: DeleteConfirmModalProps) {
  const [isDeleting, setIsDeleting] = useState(false)

  useEffect(() => {
    if (isOpen) {
      setIsDeleting(false)
    }
  }, [isOpen, taskId])

  const handleConfirm = async () => {
    setIsDeleting(true)
    try {
      await onConfirm()
    } catch {
      setIsDeleting(false)
    }
  }

  return (
    <AlertDialog open={isOpen} onOpenChange={(open) => (!open ? onCancel() : undefined)}>
      <AlertDialogContent className="rounded-[1.75rem] border-border/70 bg-card/95 shadow-[0_24px_90px_-40px_rgba(15,23,42,0.48)] backdrop-blur-2xl">
        <AlertDialogHeader className="space-y-4">
          <div className="flex items-center gap-3">
            <div className="rounded-2xl bg-destructive/12 p-3 text-destructive">
              <AlertTriangle className="size-5" />
            </div>
            <div>
              <AlertDialogTitle className="text-left text-xl tracking-tight">
                {isRootTask ? "确认删除整棵任务树？" : "确认删除当前任务分支？"}
              </AlertDialogTitle>
              <AlertDialogDescription className="text-left">
                {isRootTask
                  ? "此操作无法撤销，根任务及其关联子任务都会被永久删除。"
                  : "此操作无法撤销，当前任务及其所有子任务都会被永久删除。"}
              </AlertDialogDescription>
            </div>
          </div>

          <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4 text-sm text-muted-foreground">
            {taskDescription ? (
              <div className="mb-2">
                <span className="font-medium text-foreground">任务描述：</span>
                {taskDescription}
              </div>
            ) : null}
            <div className="font-mono text-xs text-muted-foreground">Task ID: {taskId}</div>
          </div>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel
            className="rounded-2xl"
            disabled={isDeleting}
            onClick={onCancel}
          >
            取消
          </AlertDialogCancel>
          <AlertDialogAction
            className="rounded-2xl bg-destructive text-destructive-foreground hover:bg-destructive/90"
            disabled={isDeleting}
            onClick={(event) => {
              event.preventDefault()
              void handleConfirm()
            }}
          >
            {isDeleting ? (
              <>
                <LoaderCircle className="mr-2 size-4 animate-spin" />
                删除中...
              </>
            ) : (
              "确认删除"
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
