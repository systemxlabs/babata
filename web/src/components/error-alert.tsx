import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { cn } from "@/lib/utils"

interface ErrorAlertProps {
  message: string
  onDismiss?: () => void
  className?: string
  compact?: boolean
}

export function ErrorAlert({
  message,
  onDismiss,
  className,
  compact = false,
}: ErrorAlertProps) {
  return (
    <Card className={cn("border-destructive/25 bg-destructive/5", className)}>
      <CardContent
        className={cn(
          "text-sm text-destructive",
          compact ? "p-4" : "flex items-center justify-between gap-4 p-5"
        )}
      >
        <span>{message}</span>
        {onDismiss ? (
          <Button variant="ghost" size="sm" onClick={onDismiss}>
            关闭
          </Button>
        ) : null}
      </CardContent>
    </Card>
  )
}
