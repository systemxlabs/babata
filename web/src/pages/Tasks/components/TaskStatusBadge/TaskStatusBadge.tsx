import {
  AlertTriangle,
  Ban,
  CheckCircle2,
  PauseCircle,
  RefreshCw,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import type { TaskStatus } from "@/types"
import { STATUS_LABELS } from "@/types"
import { cn } from "@/lib/utils"

interface TaskStatusBadgeProps {
  status: TaskStatus
  showLabel?: boolean
  size?: "sm" | "md" | "lg"
}

const statusStyles: Record<TaskStatus, string> = {
  running: "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  completed: "border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  failed: "border-rose-500/20 bg-rose-500/10 text-rose-700 dark:text-rose-300",
  paused: "border-orange-500/20 bg-orange-500/10 text-orange-700 dark:text-orange-300",
  canceled: "border-slate-500/20 bg-slate-500/10 text-slate-700 dark:text-slate-300",
}

const sizeStyles = {
  sm: "gap-1 px-2 py-0.5 text-[11px]",
  md: "gap-1.5 px-2.5 py-1 text-xs",
  lg: "gap-2 px-3 py-1.5 text-sm",
}

function StatusIcon({ status }: { status: TaskStatus }) {
  switch (status) {
    case "running":
      return <RefreshCw className="size-3.5 animate-spin" />
    case "completed":
      return <CheckCircle2 className="size-3.5" />
    case "failed":
      return <AlertTriangle className="size-3.5" />
    case "paused":
      return <PauseCircle className="size-3.5" />
    case "canceled":
      return <Ban className="size-3.5" />
    default:
      return null
  }
}

export function TaskStatusBadge({
  status,
  showLabel = true,
  size = "md",
}: TaskStatusBadgeProps) {
  return (
    <Badge
      variant="outline"
      className={cn(
        "rounded-full font-medium shadow-none",
        statusStyles[status],
        sizeStyles[size]
      )}
    >
      <StatusIcon status={status} />
      {showLabel ? <span>{STATUS_LABELS[status]}</span> : null}
    </Badge>
  )
}
