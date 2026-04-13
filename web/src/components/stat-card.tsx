import type { LucideIcon } from "lucide-react"

import { Card, CardContent } from "@/components/ui/card"
import { cn } from "@/lib/utils"

interface StatCardProps {
  icon: LucideIcon
  label: string
  value: number | string
  tone?: "primary" | "cyan" | "emerald" | "amber" | "rose"
  className?: string
}

const toneStyles = {
  primary: "bg-primary/12 text-primary",
  cyan: "bg-sky-500/12 text-sky-600 dark:text-sky-300",
  emerald: "bg-emerald-500/12 text-emerald-600 dark:text-emerald-300",
  amber: "bg-amber-500/12 text-amber-600 dark:text-amber-300",
  rose: "bg-rose-500/12 text-rose-600 dark:text-rose-300",
}

export function StatCard({
  icon: Icon,
  label,
  value,
  tone = "primary",
  className,
}: StatCardProps) {
  return (
    <Card
      className={cn(
        "overflow-hidden rounded-[1.75rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl",
        className
      )}
    >
      <CardContent className="flex items-center gap-4 p-5">
        <div className={cn("rounded-[1.35rem] p-3.5", toneStyles[tone])}>
          <Icon className="size-5" />
        </div>
        <div className="space-y-1">
          <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
            {label}
          </div>
          <div className="text-3xl font-semibold tracking-tight">{value}</div>
        </div>
      </CardContent>
    </Card>
  )
}
