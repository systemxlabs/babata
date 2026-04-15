import { useCallback, useEffect, useState } from "react"
import { BookMarked, Sparkles } from "lucide-react"

import { getSkillFile, getSkillFiles } from "@/api"
import { ErrorAlert } from "@/components/error-alert"
import { FileExplorer } from "@/components/FileExplorer/FileExplorer"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Separator } from "@/components/ui/separator"
import { LoadingState } from "@/components/loading-state"
import { Card, CardContent } from "@/components/ui/card"
import type { FileEntry, Skill } from "@/types"

interface SkillDetailModalProps {
  skill: Skill | null
  isOpen: boolean
  onClose: () => void
}

export function SkillDetailModal({ skill, isOpen, onClose }: SkillDetailModalProps) {
  const [files, setFiles] = useState<FileEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchSkillFiles = useCallback(async () => {
    if (!skill) return

    setLoading(true)
    setError(null)

    try {
      const response = await getSkillFiles(skill.name)
      setFiles(response)
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载技能详情失败")
      setFiles([])
    } finally {
      setLoading(false)
    }
  }, [skill])

  const loadSkillFile = useCallback(
    async (path: string) => {
      if (!skill) {
        throw new Error("技能不存在")
      }

      return getSkillFile(skill.name, path)
    },
    [skill]
  )

  const loadSkillDirectory = useCallback(
    async (path?: string) => {
      if (!skill) {
        throw new Error("技能不存在")
      }

      return getSkillFiles(skill.name, path)
    },
    [skill]
  )

  useEffect(() => {
    if (!isOpen || !skill) return
    void fetchSkillFiles()
  }, [fetchSkillFiles, isOpen, skill])

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="max-h-[calc(100vh-2rem)] overflow-hidden rounded-[1.9rem] border-border/70 bg-card/95 p-0 sm:max-w-[1180px]">
        {skill ? (
          <>
            <DialogHeader className="space-y-4 px-6 pt-6">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="space-y-3">
                  <Badge variant="outline" className="rounded-full px-3 py-1">
                    <BookMarked className="mr-2 size-3.5" />
                    Skill
                  </Badge>
                  <div>
                    <DialogTitle className="text-2xl tracking-tight">{skill.name}</DialogTitle>
                    <DialogDescription className="mt-2 max-w-3xl text-sm leading-6">
                      技能详情、正文与附属文件目录
                    </DialogDescription>
                  </div>
                </div>
                <Badge variant="secondary" className="rounded-full px-3 py-1.5">
                  <Sparkles className="mr-2 size-3.5" />
                  SKILL.md 优先预览
                </Badge>
              </div>
            </DialogHeader>
            <Separator className="mt-5" />
            <div className="min-h-0 space-y-5 overflow-y-auto px-6 py-6">
              <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                <CardContent className="p-5">
                  <div className="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                    Description
                  </div>
                  <p className="mt-3 text-sm leading-7 text-foreground">
                    {skill.description || "暂无描述"}
                  </p>
                </CardContent>
              </Card>

              {loading ? (
                <LoadingState
                  title="加载技能目录"
                  description="正在读取技能文件与正文。"
                  className="min-h-[420px]"
                />
              ) : error ? (
                <ErrorAlert message={error} compact className="rounded-[1.6rem]" />
              ) : (
                <FileExplorer
                  files={files}
                  loadDirectory={loadSkillDirectory}
                  loadFileContent={loadSkillFile}
                  treeTitle="技能文件"
                  emptyMessage="暂无技能文件"
                  placeholderMessage="选择技能文件查看内容"
                  defaultSelectedPath="SKILL.md"
                />
              )}
            </div>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  )
}
