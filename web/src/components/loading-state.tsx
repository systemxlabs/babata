import { LoaderCircle } from "lucide-react"

import { cn } from "@/lib/utils"

interface LoadingStateProps {
  title?: string
  description?: string
  className?: string
}

export function LoadingState({
  title = "加载中",
  description = "正在获取最新数据，请稍候。",
  className,
}: LoadingStateProps) {
  return (
    <div
      className={cn(
        "flex min-h-[320px] flex-col items-center justify-center rounded-[2rem] border border-border/70 bg-card/60 px-6 py-12 text-center shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl",
        className
      )}
    >
      <div className="mb-5 rounded-full bg-primary/10 p-4 text-primary">
        <LoaderCircle className="size-7 animate-spin" />
      </div>
      <div className="space-y-1">
        <h3 className="text-xl font-semibold tracking-tight">{title}</h3>
        <p className="text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
    </div>
  )
}
