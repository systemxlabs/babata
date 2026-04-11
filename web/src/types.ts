// API 类型定义

// 任务状态类型
export type TaskStatus = 'running' | 'completed' | 'failed' | 'canceled' | 'paused';

// 任务类型
export type TaskType = 'roottask' | 'subtask';

// 任务控制操作
export type TaskControlAction = 'pause' | 'resume' | 'cancel';

// 基础任务信息
export interface Task {
  task_id: string;
  description: string;
  agent: string;
  status: TaskStatus;
  parent_task_id?: string | null;
  root_task_id: string;
  created_at: number;
  never_ends: boolean;
}

// 根任务（包含子任务统计）
export interface RootTask extends Task {
  subtask_count: number;
  children?: Task[];
}

// 任务筛选条件
export interface TaskFilter {
  status?: TaskStatus | 'all';
  agent?: string;
  search?: string;
  page: number;
  pageSize: number;
}

// 任务列表响应
export interface TaskListResponse {
  tasks: RootTask[];
  total: number;
  page: number;
  pageSize: number;
}

// 文件条目类型
export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number | null;
  modified: number;
}

// 任务文件
export interface TaskFile {
  name: string;
  path: string;
  isDirectory: boolean;
  size?: number;
  modifiedAt?: number;
}

// 任务详情
export interface TaskDetail extends Task {
  children: Task[];
  files: TaskFile[];
  logs: string[];
  final_response?: string;
}

// Agent 类型
export interface Agent {
  name: string;
  description?: string;
}

// Skill 类型
export interface Skill {
  name: string;
  description?: string;
}

// API 响应类型
export interface CountResponse {
  count: number;
}

export interface TasksResponse {
  tasks: Task[];
}

export interface AgentsResponse {
  agents: Agent[];
}

export interface SkillsResponse {
  skills: Skill[];
}

export interface CreateTaskRequest {
  agent: string;
  prompt: string;
  description: string;
  task_type?: 'roottask' | 'subtask';
}

export interface CreateTaskResponse {
  task_id: string;
  status: string;
}

// 状态颜色映射
export const STATUS_COLORS: Record<TaskStatus, string> = {
  running: '#F59E0B',   // 琥珀色
  completed: '#10B981', // 翠绿色
  failed: '#EF4444',    // 红色
  paused: '#F97316',    // 橙色
  canceled: '#6B7280',  // 灰色
};

// 状态标签映射
export const STATUS_LABELS: Record<TaskStatus | 'all', string> = {
  all: '全部',
  running: '运行中',
  completed: '已完成',
  failed: '失败',
  paused: '已暂停',
  canceled: '已取消',
};

// 状态背景色映射（用于深色主题）
export const STATUS_BG_COLORS: Record<TaskStatus, string> = {
  running: 'bg-amber-500/10',
  completed: 'bg-emerald-500/10',
  failed: 'bg-red-500/10',
  paused: 'bg-orange-500/10',
  canceled: 'bg-gray-500/10',
};

// 状态文字色映射
export const STATUS_TEXT_COLORS: Record<TaskStatus, string> = {
  running: 'text-amber-500',
  completed: 'text-emerald-500',
  failed: 'text-red-500',
  paused: 'text-orange-500',
  canceled: 'text-gray-500',
};

// 日志条目类型
export type LogEntry = string;
