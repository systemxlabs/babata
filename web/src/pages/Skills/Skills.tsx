import { useCallback, useEffect, useState } from "react"
import { BookMarked, Trash2 } from "lucide-react"

import { deleteSkill, getSkills } from "@/api"
import { EmptyState } from "@/components/empty-state"
import { ErrorAlert } from "@/components/error-alert"
import { LoadingState } from "@/components/loading-state"
import { PageHeader } from "@/components/page-header"
import { SkillDetailModal } from "@/components/SkillDetailModal/SkillDetailModal"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import type { Skill } from "@/types"

export function Skills() {
  const [skills, setSkills] = useState<Skill[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [selectedSkill, setSelectedSkill] = useState<Skill | null>(null)

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

  const handleDelete = async (skill: Skill) => {
    const confirmed = window.confirm(`确定要删除技能 "${skill.name}" 吗？此操作不可撤销。`)
    if (!confirmed) return

    try {
      await deleteSkill(skill.name)
      if (selectedSkill?.name === skill.name) {
        setSelectedSkill(null)
      }
      await fetchSkills()
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除技能失败")
    }
  }

  if (loading && skills.length === 0) {
    return <LoadingState title="加载 Skills" description="正在同步技能目录与描述信息。" />
  }

  return (
    <div className="space-y-6 pb-6">
      <PageHeader
        eyebrow="Knowledge"
        title="Skills 管理"
        description="浏览技能清单、技能说明与技能目录文件，确保任务执行时能正确注入可复用知识。"
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
                    void handleDelete(skill)
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
    </div>
  )
}
