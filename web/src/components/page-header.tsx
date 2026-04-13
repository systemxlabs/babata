import type { ReactNode } from "react"

import { cn } from "@/lib/utils"

interface PageHeaderProps {
  eyebrow?: string
  title: string
  description?: string
  actions?: ReactNode
  className?: string
}

export function PageHeader({
  eyebrow,
  title,
  description,
  actions,
  className,
}: PageHeaderProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-4 rounded-[2rem] border border-border/70 bg-card/70 px-6 py-5 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.35)] backdrop-blur-xl md:flex-row md:items-end md:justify-between",
        className
      )}
    >
      <div className="space-y-2">
        {eyebrow ? (
          <div className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
            {eyebrow}
          </div>
        ) : null}
        <div className="space-y-1">
          <h1 className="text-3xl font-semibold tracking-tight text-foreground">{title}</h1>
          {description ? (
            <p className="max-w-3xl text-sm leading-6 text-muted-foreground">{description}</p>
          ) : null}
        </div>
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-3">{actions}</div> : null}
    </div>
  )
}
