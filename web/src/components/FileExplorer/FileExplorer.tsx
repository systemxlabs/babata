import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react"
import {
  ChevronRight,
  FileCode2,
  FileText,
  Folder,
  FolderTree,
  LoaderCircle,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { cn } from "@/lib/utils"
import type { FileEntry } from "@/types"

interface FileExplorerProps {
  files: FileEntry[]
  loadDirectory: (path?: string) => Promise<FileEntry[]>
  loadFileContent: (path: string) => Promise<string>
  treeTitle?: string
  emptyMessage?: string
  placeholderMessage?: string
  defaultSelectedPath?: string
}

const ROOT_DIRECTORY_KEY = "__root__"

function sortEntries(entries: FileEntry[]): FileEntry[] {
  return [...entries].sort((a, b) => {
    if (a.is_dir && !b.is_dir) return -1
    if (!a.is_dir && b.is_dir) return 1
    return a.path.localeCompare(b.path)
  })
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const index = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, index)).toFixed(1))} ${sizes[index]}`
}

function getFileLanguage(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase() || ""
  const langMap: Record<string, string> = {
    js: "javascript",
    ts: "typescript",
    jsx: "jsx",
    tsx: "tsx",
    py: "python",
    rs: "rust",
    java: "java",
    go: "go",
    html: "html",
    css: "css",
    json: "json",
    md: "markdown",
    yaml: "yaml",
    yml: "yaml",
    xml: "xml",
    sql: "sql",
    sh: "bash",
    bash: "bash",
  }
  return langMap[ext] || "text"
}

export function FileExplorer({
  files,
  loadDirectory,
  loadFileContent,
  treeTitle = "文件列表",
  emptyMessage = "暂无文件",
  placeholderMessage = "选择文件查看内容",
  defaultSelectedPath,
}: FileExplorerProps) {
  const [entriesByDirectory, setEntriesByDirectory] = useState<Record<string, FileEntry[]>>({
    [ROOT_DIRECTORY_KEY]: sortEntries(files),
  })
  const [expandedDirectories, setExpandedDirectories] = useState<Record<string, boolean>>({})
  const [directoryLoading, setDirectoryLoading] = useState<Record<string, boolean>>({})
  const [directoryErrors, setDirectoryErrors] = useState<Record<string, string | null>>({})
  const [selectedPath, setSelectedPath] = useState<string | null>(defaultSelectedPath ?? null)
  const [fileContent, setFileContent] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setEntriesByDirectory((previous) => ({
      ...previous,
      [ROOT_DIRECTORY_KEY]: sortEntries(files),
    }))
  }, [files])

  const knownEntries = useMemo(
    () => Object.values(entriesByDirectory).flatMap((entries) => entries),
    [entriesByDirectory]
  )

  const selectedFile =
    selectedPath === null ? null : knownEntries.find((file) => file.path === selectedPath) ?? null

  useEffect(() => {
    if (selectedPath && knownEntries.some((file) => file.path === selectedPath)) {
      return
    }

    if (defaultSelectedPath && files.some((file) => file.path === defaultSelectedPath)) {
      setSelectedPath(defaultSelectedPath)
      return
    }

    if (selectedPath !== null) {
      setSelectedPath(null)
      setFileContent("")
      setError(null)
    }
  }, [defaultSelectedPath, files, knownEntries, selectedPath])

  useEffect(() => {
    if (!selectedFile || selectedFile.is_dir) {
      setFileContent("")
      setLoading(false)
      setError(null)
      return
    }

    let cancelled = false

    const run = async () => {
      setLoading(true)
      setError(null)

      try {
        const content = await loadFileContent(selectedFile.path)
        if (!cancelled) {
          setFileContent(content)
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "加载文件失败")
          setFileContent("")
        }
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    }

    void run()

    return () => {
      cancelled = true
    }
  }, [loadFileContent, selectedFile])

  const handleFileClick = useCallback((file: FileEntry) => {
    if (file.is_dir) {
      return
    }

    setSelectedPath(file.path)
  }, [])

  const handleDirectoryToggle = useCallback(
    async (directory: FileEntry) => {
      const isExpanded = expandedDirectories[directory.path] ?? false
      if (isExpanded) {
        setExpandedDirectories((previous) => ({
          ...previous,
          [directory.path]: false,
        }))
        return
      }

      setExpandedDirectories((previous) => ({
        ...previous,
        [directory.path]: true,
      }))

      if (entriesByDirectory[directory.path] !== undefined || directoryLoading[directory.path]) {
        return
      }

      setDirectoryLoading((previous) => ({
        ...previous,
        [directory.path]: true,
      }))
      setDirectoryErrors((previous) => ({
        ...previous,
        [directory.path]: null,
      }))

      try {
        const entries = await loadDirectory(directory.path)
        setEntriesByDirectory((previous) => ({
          ...previous,
          [directory.path]: sortEntries(entries),
        }))
      } catch (err) {
        setDirectoryErrors((previous) => ({
          ...previous,
          [directory.path]: err instanceof Error ? err.message : "加载目录失败",
        }))
      } finally {
        setDirectoryLoading((previous) => ({
          ...previous,
          [directory.path]: false,
        }))
      }
    },
    [directoryLoading, entriesByDirectory, expandedDirectories, loadDirectory]
  )

  const renderDirectoryEntries = useCallback(
    (directoryKey: string, level = 0): ReactNode[] => {
      const entries = entriesByDirectory[directoryKey] ?? []

      return entries.map((entry) => {
        if (!entry.is_dir) {
          const isSelected = selectedFile?.path === entry.path

          return (
            <button
              key={entry.path}
              type="button"
              className={cn(
                "flex w-full items-center gap-3 rounded-2xl px-3 py-2.5 text-left transition-colors",
                "hover:bg-accent/60",
                isSelected && "bg-primary/10 text-primary"
              )}
              style={{ paddingLeft: `${12 + level * 18}px` }}
              onClick={() => handleFileClick(entry)}
            >
              <FileText className="size-4 shrink-0" />
              <span className="min-w-0 flex-1 truncate">{entry.name}</span>
              {entry.size !== null ? (
                <span className="shrink-0 text-xs text-muted-foreground">
                  {formatFileSize(entry.size)}
                </span>
              ) : null}
            </button>
          )
        }

        const isExpanded = expandedDirectories[entry.path] ?? false
        const isLoadingDirectory = directoryLoading[entry.path] ?? false
        const directoryError = directoryErrors[entry.path]
        const childEntries = entriesByDirectory[entry.path]

        return (
          <div key={entry.path}>
            <button
              type="button"
              className={cn(
                "flex w-full items-center gap-3 rounded-2xl px-3 py-2.5 text-left transition-colors",
                "hover:bg-accent/40"
              )}
              style={{ paddingLeft: `${12 + level * 18}px` }}
              onClick={() => void handleDirectoryToggle(entry)}
            >
              <ChevronRight
                className={cn(
                  "size-4 shrink-0 transition-transform",
                  isExpanded && "rotate-90"
                )}
              />
              <Folder className="size-4 shrink-0 text-muted-foreground" />
              <span className="min-w-0 flex-1 truncate">{entry.name}</span>
              {isLoadingDirectory ? (
                <LoaderCircle className="size-3.5 shrink-0 animate-spin text-primary" />
              ) : null}
            </button>

            {isExpanded ? (
              <div className="space-y-1">
                {directoryError ? (
                  <div
                    className="px-3 py-2 text-sm text-destructive"
                    style={{ paddingLeft: `${30 + level * 18}px` }}
                  >
                    {directoryError}
                  </div>
                ) : isLoadingDirectory && childEntries === undefined ? (
                  <div
                    className="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground"
                    style={{ paddingLeft: `${30 + level * 18}px` }}
                  >
                    <LoaderCircle className="size-3.5 animate-spin text-primary" />
                    正在读取目录...
                  </div>
                ) : childEntries && childEntries.length > 0 ? (
                  renderDirectoryEntries(entry.path, level + 1)
                ) : (
                  <div
                    className="px-3 py-2 text-sm text-muted-foreground"
                    style={{ paddingLeft: `${30 + level * 18}px` }}
                  >
                    空目录
                  </div>
                )}
              </div>
            ) : null}
          </div>
        )
      })
    },
    [
      directoryErrors,
      directoryLoading,
      entriesByDirectory,
      expandedDirectories,
      handleDirectoryToggle,
      handleFileClick,
      selectedFile,
    ]
  )

  const rootEntries = entriesByDirectory[ROOT_DIRECTORY_KEY] ?? []

  return (
    <div className="grid gap-4 lg:grid-cols-[320px_minmax(0,1fr)]">
      <Card className="rounded-[1.75rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl">
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <FolderTree className="size-4.5 text-primary" />
            {treeTitle}
          </CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <ScrollArea className="h-[420px] pr-3">
            {rootEntries.length === 0 ? (
              <div className="flex h-[360px] items-center justify-center rounded-[1.4rem] border border-dashed border-border/70 bg-background/60 px-4 text-center text-sm text-muted-foreground">
                {emptyMessage}
              </div>
            ) : (
              <div className="space-y-1">{renderDirectoryEntries(ROOT_DIRECTORY_KEY)}</div>
            )}
          </ScrollArea>
        </CardContent>
      </Card>

      <Card className="rounded-[1.75rem] border-border/70 bg-card/70 shadow-[0_18px_60px_-32px_rgba(15,23,42,0.24)] backdrop-blur-xl">
        {selectedFile ? (
          <>
            <CardHeader className="pb-3">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                  <div className="rounded-2xl bg-primary/10 p-2 text-primary">
                    <FileCode2 className="size-4.5" />
                  </div>
                  <div>
                    <CardTitle className="text-base">{selectedFile.name}</CardTitle>
                    <p className="mt-1 text-xs text-muted-foreground">{selectedFile.path}</p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant="outline" className="rounded-full">
                    {getFileLanguage(selectedFile.name)}
                  </Badge>
                  {selectedFile.size !== null ? (
                    <Badge variant="secondary" className="rounded-full">
                      {formatFileSize(selectedFile.size)}
                    </Badge>
                  ) : null}
                </div>
              </div>
            </CardHeader>
            <Separator />
            <CardContent className="min-h-0 p-0">
              {loading ? (
                <div className="flex h-[420px] flex-col items-center justify-center gap-3 text-sm text-muted-foreground">
                  <LoaderCircle className="size-5 animate-spin text-primary" />
                  正在加载文件内容...
                </div>
              ) : error ? (
                <div className="flex h-[420px] items-center justify-center px-6 text-center text-sm text-destructive">
                  {error}
                </div>
              ) : (
                <div className="h-[420px] overflow-y-auto px-5 pt-5 pb-8">
                  <pre className="m-0 whitespace-pre-wrap break-words font-mono text-[13px] leading-6 text-foreground">
                    <code>{fileContent}</code>
                  </pre>
                </div>
              )}
            </CardContent>
          </>
        ) : (
          <CardContent className="flex h-[496px] flex-col items-center justify-center gap-4 text-center">
            <div className="rounded-full bg-primary/10 p-4 text-primary">
              <FileCode2 className="size-6" />
            </div>
            <div className="space-y-2">
              <div className="text-lg font-semibold tracking-tight">准备预览</div>
              <p className="max-w-md text-sm leading-6 text-muted-foreground">
                {placeholderMessage}
              </p>
            </div>
          </CardContent>
        )}
      </Card>
    </div>
  )
}
