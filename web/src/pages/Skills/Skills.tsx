import { useCallback, useEffect, useState } from "react"
import { BookMarked, Trash2, AlertTriangle } from "lucide-react"

import { deleteSkill, getSkills } from "@/api"
import { EmptyState } from "@/components/empty-state"
import { ErrorAlert } from "@/components/error-alert"
import { LoadingState } from "@/components/loading-state"
import { PageHeader } from "@/components/page-header"
import { SkillDetailModal } from "@/components/SkillDetailModal/SkillDetailModal"
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
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import type { Skill } from "@/types"

export function Skills() {
  const [skills, setSkills] = useState<Skill[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [selectedSkill, setSelectedSkill] = useState<Skill | null>(null)
  const [skillToDelete, setSkillToDelete] = useState<Skill | null>(null)
  const [deleteModalOpen, setDeleteModalOpen] = useState(false)

  const fetchSkills = useCallback(async () => {
    try {
      setLoading(true)
      const response = await getSkills()
      setSkills(response.skills)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取技能列表失败")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void fetchSkills()
  }, [fetchSkills])

  const handleDeleteConfirm = async () => {
    if (!skillToDelete) return
    try {
      await deleteSkill(skillToDelete.name)
      if (selectedSkill?.name === skillToDelete.name) {
        setSelectedSkill(null)
      }
      await fetchSkills()
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除技能失败")
    } finally {
      setSkillToDelete(null)
      setDeleteModalOpen(false)
    }
  }

  if (loading && skills.length === 0) {
    return <LoadingState title="加载 Skills" description="正在同步技能目录与描述信息。" />
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Knowledge"
        title="Skill 资源库"
        description="浏览技能清单、技能说明与目录文件，确保任务执行时能正确注入可复用知识。"
        actions={
          <Badge variant="outline" className="rounded-full px-3 py-1.5">
            {skills.length} 个 Skill
          </Badge>
        }
      />

      {error ? (
        <ErrorAlert
          message={error}
          onDismiss={() => setError(null)}
          className="rounded-[1.75rem]"
        />
      ) : null}

      {skills.length === 0 ? (
        <EmptyState
          icon={BookMarked}
          title="还没有 Skill"
          description="当你把技能目录放入 Babata 的 skill 目录后，这里会显示它们的元信息和正文。"
        />
      ) : (
        <div className="grid gap-5 xl:grid-cols-2">
          {skills.map((skill) => (
            <Card
              key={skill.name}
              className="group flex h-full cursor-pointer rounded-[1.8rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-[0_22px_70px_-36px_rgba(15,23,42,0.32)] backdrop-blur-xl"
              onClick={() => setSelectedSkill(skill)}
            >
              <CardHeader className="flex flex-1 flex-row items-start justify-between gap-4 space-y-0">
                <div className="flex min-h-[132px] flex-1 flex-col space-y-3">
                  <Badge variant="outline" className="rounded-full px-3 py-1 text-[0.72rem] uppercase tracking-[0.2em]">
                    Skill
                  </Badge>
                  <div className="flex-1">
                    <CardTitle className="text-2xl tracking-tight">{skill.name}</CardTitle>
                    <p className="mt-2 line-clamp-4 text-sm leading-6 text-muted-foreground">
                      {skill.description || "暂无描述"}
                    </p>
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="icon"
                  onClick={(event) => {
                    event.stopPropagation()
                    setSkillToDelete(skill)
                    setDeleteModalOpen(true)
                  }}
                >
                  <Trash2 className="size-4 text-destructive" />
                </Button>
              </CardHeader>
              <CardContent className="mt-auto pt-0" />
            </Card>
          ))}
        </div>
      )}

      <SkillDetailModal
        skill={selectedSkill}
        isOpen={selectedSkill !== null}
        onClose={() => setSelectedSkill(null)}
      />

      <SkillDeleteDialog
        skill={skillToDelete}
        open={deleteModalOpen}
        onOpenChange={setDeleteModalOpen}
        onConfirm={handleDeleteConfirm}
      />
    </div>
  )
}

function SkillDeleteDialog({
  skill,
  open,
  onOpenChange,
  onConfirm,
}: {
  skill: Skill | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => Promise<void>
}) {
  const [loading, setLoading] = useState(false)

  if (!skill) return null

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="rounded-[1.75rem] border-border/70 bg-card/95">
        <AlertDialogHeader>
          <div className="mb-3 flex items-center gap-3">
            <div className="rounded-2xl bg-destructive/12 p-3 text-destructive">
              <AlertTriangle className="size-5" />
            </div>
            <div>
              <AlertDialogTitle>删除 Skill</AlertDialogTitle>
              <AlertDialogDescription>
                删除后该技能文件将从系统移除，已关联的任务可能受影响。
              </AlertDialogDescription>
            </div>
          </div>
        </AlertDialogHeader>

        <div className="rounded-[1.4rem] border border-border/70 bg-background/70 p-4 text-sm text-muted-foreground">
          即将删除 <span className="font-semibold text-foreground">{skill.name}</span>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={loading}>取消</AlertDialogCancel>
          <AlertDialogAction
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            disabled={loading}
            onClick={(event) => {
              event.preventDefault()
              setLoading(true)
              void onConfirm().finally(() => setLoading(false))
            }}
          >
            {loading ? "删除中..." : "确认删除"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
