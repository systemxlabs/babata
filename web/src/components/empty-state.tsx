import type { ReactNode } from "react"

import type { LucideIcon } from "lucide-react"

import { cn } from "@/lib/utils"

interface EmptyStateProps {
  icon: LucideIcon
  title: string
  description?: string
  action?: ReactNode
  className?: string
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex min-h-[320px] flex-col items-center justify-center rounded-[2rem] border border-dashed border-border/70 bg-card/60 px-6 py-12 text-center shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl",
        className
      )}
    >
      <div className="mb-5 rounded-full bg-primary/10 p-4 text-primary">
        <Icon className="size-7" />
      </div>
      <div className="space-y-2">
        <h3 className="text-xl font-semibold tracking-tight">{title}</h3>
        {description ? (
          <p className="max-w-md text-sm leading-6 text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {action ? <div className="mt-6">{action}</div> : null}
    </div>
  )
}
