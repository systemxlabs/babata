import { ChevronLeft, ChevronRight } from "lucide-react"

import { Button } from "@/components/ui/button"

interface TaskPaginationProps {
  currentPage: number
  pageSize: number
  total: number
  onPageChange: (page: number) => void
}

export function TaskPagination({
  currentPage,
  pageSize,
  total,
  onPageChange,
}: TaskPaginationProps) {
  const totalPages = Math.ceil(total / pageSize)

  if (totalPages <= 1) return null

  const getPageNumbers = () => {
    const pages: (number | string)[] = []
    const maxVisiblePages = 5

    if (totalPages <= maxVisiblePages) {
      for (let i = 1; i <= totalPages; i += 1) {
        pages.push(i)
      }
    } else {
      pages.push(1)

      if (currentPage > 3) {
        pages.push("...")
      }

      const startPage = Math.max(2, currentPage - 1)
      const endPage = Math.min(totalPages - 1, currentPage + 1)

      for (let i = startPage; i <= endPage; i += 1) {
        if (!pages.includes(i)) {
          pages.push(i)
        }
      }

      if (currentPage < totalPages - 2) {
        pages.push("...")
      }

      if (!pages.includes(totalPages)) {
        pages.push(totalPages)
      }
    }

    return pages
  }

  const pages = getPageNumbers()

  return (
    <div className="mt-6 flex flex-col gap-4 rounded-[1.75rem] border border-border/70 bg-card/70 px-5 py-4 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl md:flex-row md:items-center md:justify-between">
      <div className="text-sm text-muted-foreground">
        共 <span className="font-semibold text-foreground">{total}</span> 条记录，
        每页 <span className="font-semibold text-foreground">{pageSize}</span> 条
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          size="icon"
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage <= 1}
        >
          <ChevronLeft className="size-4" />
        </Button>

        {pages.map((page, index) =>
          page === "..." ? (
            <span key={`ellipsis-${index}`} className="px-2 text-sm text-muted-foreground">
              ...
            </span>
          ) : (
            <Button
              key={page}
              variant={currentPage === page ? "default" : "outline"}
              size="sm"
              onClick={() => onPageChange(page as number)}
              className="min-w-9"
            >
              {page}
            </Button>
          )
        )}

        <Button
          variant="outline"
          size="icon"
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage >= totalPages}
        >
          <ChevronRight className="size-4" />
        </Button>
      </div>
    </div>
  )
}
