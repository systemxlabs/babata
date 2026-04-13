export type TaskStatus = 'running' | 'completed' | 'failed' | 'canceled' | 'paused';

export type TaskType = 'roottask' | 'subtask';

export type TaskControlAction = 'pause' | 'resume' | 'cancel';

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

export interface RootTask extends Task {
  subtask_count: number;
  children?: Task[];
}

export interface TaskFilter {
  status?: TaskStatus | 'all';
  agent?: string;
  search?: string;
  page: number;
  pageSize: number;
}

export interface TaskListResponse {
  tasks: RootTask[];
  total: number;
  page: number;
  pageSize: number;
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number | null;
  modified: number | null;
}

export interface TaskFile {
  name: string;
  path: string;
  isDirectory: boolean;
  size?: number;
  modifiedAt?: number;
}

export interface TaskDetail extends Task {
  children: Task[];
  files: TaskFile[];
  logs: string[];
  final_response?: string;
}

export interface AgentFrontmatter {
  name: string;
  description: string;
  provider: string;
  model: string;
  allowed_tools: string[];
  default: boolean;
}

export type Agent = AgentFrontmatter;

export interface AgentDetail extends AgentFrontmatter {
  body: string;
}

export type CreateAgentRequest = AgentDetail;

export interface UpdateAgentRequest {
  description: string;
  provider: string;
  model: string;
  allowed_tools: string[];
  default: boolean;
  body: string;
}

export type GetAgentResponse = AgentDetail;

export interface ListAgentsResponse {
  agents: AgentFrontmatter[];
}

export type BuiltinProviderName =
  | 'openai'
  | 'kimi'
  | 'moonshot'
  | 'deepseek'
  | 'minimax'
  | 'anthropic';

export type ProviderName = BuiltinProviderName | 'custom';

export type CompatibleApi = 'openai' | 'anthropic';

interface ProviderConfigBase {
  api_key: string;
}

export interface BuiltinProviderConfig extends ProviderConfigBase {
  name: BuiltinProviderName;
}

export interface CustomProviderConfig extends ProviderConfigBase {
  name: 'custom';
  base_url: string;
  compatible_api: CompatibleApi;
}

export type ProviderConfig = BuiltinProviderConfig | CustomProviderConfig;

export interface ProvidersResponse {
  providers: ProviderConfig[];
}

export interface Skill {
  name: string;
  description?: string;
}

export interface CountResponse {
  count: number;
}

export interface TasksResponse {
  tasks: Task[];
}

export interface AgentsResponse {
  agents: AgentFrontmatter[];
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

export const STATUS_COLORS: Record<TaskStatus, string> = {
  running: '#F59E0B',
  completed: '#10B981',
  failed: '#EF4444',
  paused: '#F97316',
  canceled: '#6B7280',
};

export const STATUS_LABELS: Record<TaskStatus | 'all', string> = {
  all: '全部',
  running: '运行中',
  completed: '已完成',
  failed: '失败',
  paused: '已暂停',
  canceled: '已取消',
};

export const STATUS_BG_COLORS: Record<TaskStatus, string> = {
  running: 'bg-amber-500/10',
  completed: 'bg-emerald-500/10',
  failed: 'bg-red-500/10',
  paused: 'bg-orange-500/10',
  canceled: 'bg-gray-500/10',
};

export const STATUS_TEXT_COLORS: Record<TaskStatus, string> = {
  running: 'text-amber-500',
  completed: 'text-emerald-500',
  failed: 'text-red-500',
  paused: 'text-orange-500',
  canceled: 'text-gray-500',
};

export type LogEntry = string;
