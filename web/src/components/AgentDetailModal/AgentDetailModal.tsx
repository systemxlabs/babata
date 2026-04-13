import { useCallback, useEffect, useState } from "react"
import { Bot, BrainCircuit, ShieldCheck, Sparkles, Wrench } from "lucide-react"

import { getAgent, getAgentFile, getAgentFiles } from "@/api"
import { FileExplorer } from "@/components/FileExplorer/FileExplorer"
import { LoadingState } from "@/components/loading-state"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent } from "@/components/ui/card"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Separator } from "@/components/ui/separator"
import type { AgentDetail, FileEntry } from "@/types"

interface AgentDetailModalProps {
  agentName: string | null
  isOpen: boolean
  onClose: () => void
}

export function AgentDetailModal({ agentName, isOpen, onClose }: AgentDetailModalProps) {
  const [agent, setAgent] = useState<AgentDetail | null>(null)
  const [files, setFiles] = useState<FileEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchAgentDetail = useCallback(async () => {
    if (!agentName) return

    setLoading(true)
    setError(null)

    try {
      const [agentResponse, filesResponse] = await Promise.all([
        getAgent(agentName),
        getAgentFiles(agentName),
      ])

      if (!agentResponse) {
        throw new Error(`Agent "${agentName}" 不存在`)
      }

      setAgent(agentResponse)
      setFiles(filesResponse)
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载 Agent 详情失败")
      setAgent(null)
      setFiles([])
    } finally {
      setLoading(false)
    }
  }, [agentName])

  const loadAgentFile = useCallback(async (path: string) => {
    if (!agentName) {
      throw new Error("Agent 不存在")
    }

    return getAgentFile(agentName, path)
  }, [agentName])

  useEffect(() => {
    if (!isOpen || !agentName) return
    void fetchAgentDetail()
  }, [agentName, fetchAgentDetail, isOpen])

  return (
    <Dialog open={isOpen} onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent className="grid max-h-[calc(100vh-2rem)] grid-rows-[auto_auto_minmax(0,1fr)] overflow-hidden rounded-[1.9rem] border-border/70 bg-card/95 p-0 sm:max-w-[1240px]">
        {agentName ? (
          <>
            <DialogHeader className="space-y-4 px-6 pt-6">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="space-y-3">
                  <Badge variant="outline" className="rounded-full px-3 py-1">
                    <Bot className="mr-2 size-3.5" />
                    Agent
                  </Badge>
                  <div>
                    <DialogTitle className="text-2xl tracking-tight">{agentName}</DialogTitle>
                    <DialogDescription className="mt-2 max-w-3xl text-sm leading-6">
                      Agent 配置详情、系统提示词正文和关联目录文件。
                    </DialogDescription>
                  </div>
                </div>
                <Badge variant="secondary" className="rounded-full px-3 py-1.5">
                  <Sparkles className="mr-2 size-3.5" />
                  可查看完整 Agent 目录
                </Badge>
              </div>
            </DialogHeader>

            <Separator className="mt-5" />

            <div className="min-h-0 overflow-hidden px-6 py-6">
              {loading ? (
                <LoadingState
                  title="加载 Agent 详情"
                  description="正在读取 Agent 配置和目录内容。"
                  className="h-full min-h-[480px]"
                />
              ) : error ? (
                <Card className="rounded-[1.6rem] border-destructive/25 bg-destructive/5">
                  <CardContent className="p-5 text-sm text-destructive">{error}</CardContent>
                </Card>
              ) : agent ? (
                <div className="flex h-full min-h-0 flex-col gap-5">
                  <div className="grid gap-4 lg:grid-cols-3">
                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="p-5">
                        <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          <ShieldCheck className="size-3.5" />
                          Description
                        </div>
                        <p className="mt-3 text-sm leading-7 text-foreground">
                          {agent.description || "暂无描述"}
                        </p>
                      </CardContent>
                    </Card>

                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="space-y-4 p-5">
                        <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          <BrainCircuit className="size-3.5" />
                          Runtime
                        </div>
                        <div className="space-y-3 text-sm">
                          <div>
                            <div className="text-muted-foreground">Provider</div>
                            <div className="mt-1 font-medium text-foreground">{agent.provider}</div>
                          </div>
                          <div>
                            <div className="text-muted-foreground">Model</div>
                            <div className="mt-1 font-medium text-foreground">{agent.model}</div>
                          </div>
                        </div>
                      </CardContent>
                    </Card>

                    <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                      <CardContent className="p-5">
                        <div className="flex items-center gap-2 text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          <Wrench className="size-3.5" />
                          Capability
                        </div>
                        <div className="mt-3 flex flex-wrap gap-2">
                          <Badge
                            variant={agent.default ? "default" : "outline"}
                            className="rounded-full px-3 py-1"
                          >
                            {agent.default ? "默认 Agent" : "普通 Agent"}
                          </Badge>
                          {agent.allowed_tools.length === 0 ? (
                            <Badge variant="outline" className="rounded-full px-3 py-1">
                              无工具限制配置
                            </Badge>
                          ) : (
                            agent.allowed_tools.map((tool) => (
                              <Badge key={tool} variant="outline" className="rounded-full px-3 py-1">
                                {tool}
                              </Badge>
                            ))
                          )}
                        </div>
                      </CardContent>
                    </Card>
                  </div>

                  <Card className="rounded-[1.6rem] border-border/70 bg-background/70">
                    <CardContent className="p-0">
                      <div className="border-b border-border/70 px-5 py-4">
                        <div className="text-[0.72rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                          Body
                        </div>
                      </div>
                      <div className="max-h-[220px] overflow-auto">
                        <pre className="overflow-x-auto px-5 py-5 font-mono text-[13px] leading-6 text-foreground">
                          <code>{agent.body}</code>
                        </pre>
                      </div>
                    </CardContent>
                  </Card>

                  <div className="min-h-0 flex-1">
                    <FileExplorer
                      files={files}
                      loadFileContent={loadAgentFile}
                      treeTitle="Agent 文件"
                      emptyMessage="暂无 Agent 文件"
                      placeholderMessage="选择 Agent 文件查看内容"
                      defaultSelectedPath="AGENT.md"
                    />
                  </div>
                </div>
              ) : null}
            </div>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  )
}
