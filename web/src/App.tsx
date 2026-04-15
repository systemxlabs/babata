import type { ComponentType } from "react"
import { useEffect, useMemo } from "react"
import {
  Blocks,
  BrainCircuit,
  Cable,
  LayoutDashboard,
  Menu,
  PlugZap,
  Sparkles,
  Workflow,
} from "lucide-react"
import { BrowserRouter, NavLink, Navigate, Route, Routes, useLocation } from "react-router-dom"

import { TooltipProvider } from "@/components/ui/tooltip"
import { Button } from "@/components/ui/button"
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"
import { Agents } from "@/pages/Agents/Agents"
import { Channels } from "@/pages/Channels/Channels"
import { Dashboard } from "@/pages/Dashboard/Dashboard"
import { Providers } from "@/pages/Providers/Providers"
import { Skills } from "@/pages/Skills/Skills"
import { Tasks } from "@/pages/Tasks/Tasks"

type PageType = "dashboard" | "tasks" | "agents" | "providers" | "channels" | "skills"

const navItems: {
  key: PageType
  path: string
  label: string
  browserTitle: string
  description: string
  icon: ComponentType<{ className?: string }>
}[] = [
  {
    key: "dashboard",
    path: "/",
    label: "Overview",
    browserTitle: "控制台总览",
    description: "运行状态总览与快捷任务入口",
    icon: LayoutDashboard,
  },
  {
    key: "tasks",
    path: "/tasks",
    label: "Tasks",
    browserTitle: "任务工作台",
    description: "根任务列表、任务树与执行详情",
    icon: Workflow,
  },
  {
    key: "agents",
    path: "/agents",
    label: "Agents",
    browserTitle: "Agent 管理",
    description: "角色定义、模型绑定与目录查看",
    icon: BrainCircuit,
  },
  {
    key: "providers",
    path: "/providers",
    label: "Providers",
    browserTitle: "Provider 配置",
    description: "模型服务接入与凭据管理",
    icon: PlugZap,
  },
  {
    key: "channels",
    path: "/channels",
    label: "Channels",
    browserTitle: "Channel 接入",
    description: "消息入口与用户触达配置",
    icon: Cable,
  },
  {
    key: "skills",
    path: "/skills",
    label: "Skills",
    browserTitle: "Skill 资源库",
    description: "技能目录、正文与文件浏览",
    icon: Blocks,
  },
]

function AppNav({ mobile = false }: { mobile?: boolean }) {
  return (
    <nav className={cn("flex flex-col gap-2", mobile && "pt-3")}>
      {navItems.map((item) => {
        const Icon = item.icon
        return (
          <NavLink
            key={item.key}
            to={item.path}
            end={item.path === "/"}
            className={({ isActive }) =>
              cn(
                "group rounded-2xl border border-transparent px-3 py-3 transition-all",
                "hover:border-border/80 hover:bg-accent/70",
                isActive
                  ? "border-border bg-card shadow-[0_16px_45px_-30px_rgba(15,23,42,0.4)]"
                  : "text-muted-foreground"
              )
            }
          >
            {({ isActive }) => (
              <div className="flex items-start gap-3">
                <div
                  className={cn(
                    "mt-0.5 rounded-xl border p-2.5 transition-colors",
                    isActive
                      ? "border-primary/20 bg-primary/12 text-primary"
                      : "border-border/70 bg-background/70 text-muted-foreground group-hover:text-foreground"
                  )}
                >
                  <Icon className="size-4.5" />
                </div>
                <div className="space-y-1">
                  <div
                    className={cn(
                      "text-sm font-semibold tracking-tight",
                      isActive ? "text-foreground" : "text-foreground/90"
                    )}
                  >
                    {item.label}
                  </div>
                  <div className="text-xs leading-5 text-muted-foreground">
                    {item.description}
                  </div>
                </div>
              </div>
            )}
          </NavLink>
        )
      })}
    </nav>
  )
}

function AppShell() {
  const location = useLocation()
  const currentItem = useMemo(
    () =>
      navItems.find((item) =>
        item.path === "/"
          ? location.pathname === "/"
          : location.pathname.startsWith(item.path)
      ) ?? navItems[0],
    [location.pathname]
  )

  useEffect(() => {
    document.title = `${currentItem.browserTitle} | Babata Console`
  }, [currentItem.browserTitle])

  return (
    <div className="relative min-h-screen">
      <div className="absolute inset-x-0 top-0 -z-10 h-[420px] bg-[radial-gradient(circle_at_top,rgba(99,102,241,0.16),transparent_46%),radial-gradient(circle_at_75%_18%,rgba(14,165,233,0.12),transparent_24%)]" />

      <div className="mx-auto flex min-h-screen max-w-[1680px] gap-6 px-4 py-4 md:px-6 lg:px-8">
        <aside className="sticky top-4 hidden h-[calc(100vh-2rem)] w-[320px] shrink-0 rounded-[2rem] border border-border/70 bg-sidebar/85 p-4 shadow-[0_28px_80px_-36px_rgba(15,23,42,0.45)] backdrop-blur-2xl lg:flex lg:flex-col">
          <div className="rounded-[1.5rem] border border-border/70 bg-card/75 p-4">
            <div className="flex items-center gap-3">
              <div className="rounded-2xl bg-primary/12 p-3 text-primary">
                <Sparkles className="size-5" />
              </div>
              <div>
                <div className="text-lg font-semibold tracking-tight">Babata Console</div>
                <div className="text-xs leading-5 text-muted-foreground">
                  shadcn/ui powered operations workspace
                </div>
              </div>
            </div>
          </div>

          <ScrollArea className="mt-4 flex-1 pr-2">
            <AppNav />
          </ScrollArea>

          <div className="mt-4 rounded-[1.5rem] border border-border/70 bg-card/75 p-4">
            <div className="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
              Current Focus
            </div>
            <div className="mt-3 text-sm font-semibold text-foreground">
              {currentItem.label}
            </div>
            <p className="mt-1 text-xs leading-5 text-muted-foreground">
              {currentItem.description}
            </p>
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <header className="sticky top-4 z-20 rounded-[2rem] border border-border/70 bg-card/80 px-4 py-3 shadow-[0_20px_60px_-36px_rgba(15,23,42,0.45)] backdrop-blur-2xl lg:hidden">
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-3">
                <div className="rounded-2xl bg-primary/12 p-2.5 text-primary">
                  <Sparkles className="size-4.5" />
                </div>
                <div>
                  <div className="text-sm font-semibold tracking-tight">Babata Console</div>
                  <div className="text-xs text-muted-foreground">{currentItem.label}</div>
                </div>
              </div>
              <Sheet>
                <SheetTrigger asChild>
                  <Button variant="outline" size="icon">
                    <Menu className="size-4.5" />
                  </Button>
                </SheetTrigger>
                <SheetContent side="left" className="w-[90vw] max-w-[360px] border-border/70 bg-sidebar/95 p-0">
                  <SheetHeader className="border-b border-border/70 p-5 text-left">
                    <SheetTitle className="flex items-center gap-3 text-base">
                      <span className="rounded-2xl bg-primary/12 p-2 text-primary">
                        <Sparkles className="size-4.5" />
                      </span>
                      Babata Console
                    </SheetTitle>
                  </SheetHeader>
                  <ScrollArea className="h-full px-4 pb-8">
                    <AppNav mobile />
                  </ScrollArea>
                </SheetContent>
              </Sheet>
            </div>
          </header>

          <main className="min-w-0 flex-1">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/tasks" element={<Tasks />} />
              <Route path="/agents" element={<Agents />} />
              <Route path="/providers" element={<Providers />} />
              <Route path="/channels" element={<Channels />} />
              <Route path="/skills" element={<Skills />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </main>
        </div>
      </div>
    </div>
  )
}

function App() {
  return (
    <TooltipProvider>
      <BrowserRouter>
        <AppShell />
      </BrowserRouter>
    </TooltipProvider>
  )
}

export default App
