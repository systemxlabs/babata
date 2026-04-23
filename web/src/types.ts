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
  unread_steer_messages?: SteerMessage[];
}

export interface SteerMessage {
  content: MessageContentPart[];
  created_at: string;
}

export interface RootTask extends Task {
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

export interface ToolCall {
  call_id: string;
  tool_name: string;
  args: string;
}

export type MessageType =
  | 'user_prompt'
  | 'user_steering'
  | 'assistant_response'
  | 'assistant_tool_calls'
  | 'assistant_thinking'
  | 'tool_result';

export type MessageContentPart =
  | { type: 'text'; text: string }
  | { type: 'image_url'; url: string }
  | { type: 'image_data'; data: string; media_type: string }
  | { type: 'audio_data'; data: string; media_type: string };

export interface MessageRecord {
  task_id: string;
  message_type: MessageType;
  content: MessageContentPart[] | null;
  signature: string | null;
  tool_calls: ToolCall[] | null;
  result: string | null;
  created_at: string;
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

export type CompatibleApi = 'openai' | 'anthropic';

export type ChannelKind = 'telegram' | 'wechat';

export interface ProviderModelConfig {
  id: string;
  context_window: number;
}

export interface ProviderConfig {
  name: string;
  api_key: string;
  base_url: string;
  compatible_api: CompatibleApi;
  models: ProviderModelConfig[];
}

export interface ProvidersResponse {
  providers: ProviderConfig[];
}

export interface TestProviderConnectionResponse {
  latency_ms: number;
}

export interface TelegramChannelConfig {
  name: string;
  kind: 'telegram';
  bot_token: string;
  user_id: number;
}

export interface WechatChannelConfig {
  name: string;
  kind: 'wechat';
  bot_token: string;
  user_id: string;
}

export type ChannelConfig = TelegramChannelConfig | WechatChannelConfig;

export interface ChannelsResponse {
  channels: ChannelConfig[];
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

export interface ImageDataContent {
  type: 'image_data';
  data: string;
  media_type: 'image/png' | 'image/jpeg' | 'image/webp' | 'image/gif';
}

export interface CreateTaskRequest {
  agent: string;
  prompt: string;
  description: string;
  images?: ImageDataContent[];
  never_ends?: boolean;
}

export interface CreateTaskResponse {
  task_id: string;
  status: string;
}

export interface TextContent {
  type: 'text';
  text: string;
}

export type CreateTaskPromptPart = TextContent | ImageDataContent;

export interface SteerTaskRequest {
  content: TextContent[];
}

export const STATUS_LABELS: Record<TaskStatus | 'all', string> = {
  all: '全部',
  running: '运行中',
  completed: '已完成',
  failed: '失败',
  paused: '已暂停',
  canceled: '已取消',
};

export type LogEntry = string;
