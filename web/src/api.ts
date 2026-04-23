import type {
  AgentDetail,
  AgentsResponse,
  ChannelConfig,
  ChannelsResponse,
  CountResponse,
  CreateAgentRequest,
  CreateTaskPromptPart,
  CreateTaskRequest,
  CreateTaskResponse,
  FileEntry,
  GetAgentResponse,
  MessageContentPart,
  MessageRecord,
  ProviderConfig,
  ProvidersResponse,
  Skill,
  SteerTaskRequest,
  SkillsResponse,
  Task,
  TaskControlAction,
  TaskFilter,
  TaskListResponse,
  TaskStatus,
  TestProviderConnectionResponse,
  TasksResponse,
  UpdateAgentRequest,
  LogLevel,
  MessageType,
} from './types';

const API_BASE_URL = '/api';

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const headers = new Headers(options?.headers);
  if (options?.body !== undefined && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(
      `API Error: ${response.status} ${response.statusText}${errorText ? ` - ${errorText}` : ''}`
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const responseText = await response.text();
  if (!responseText) {
    return undefined as T;
  }

  return JSON.parse(responseText) as T;
}

export interface TaskTreeResponse {
  task_id: string;
  description: string;
  agent: string;
  status: TaskStatus;
  parent_task_id: string | null;
  root_task_id: string;
  created_at: number;
  never_ends: boolean;
  children: TaskTreeResponse[];
}

interface BackendCreateTaskRequest {
  agent: string;
  description: string;
  prompt: CreateTaskPromptPart[];
  never_ends: boolean;
}

export function getRootTasks(filter: TaskFilter): Promise<TaskListResponse> {
  const params = new URLSearchParams();

  if (filter.status && filter.status !== 'all') {
    params.append('status', filter.status);
  }

  params.append('page', filter.page.toString());
  params.append('page_size', filter.pageSize.toString());

  const queryString = params.toString();
  return fetchApi<TaskListResponse>(`/tasks${queryString ? `?${queryString}` : ''}`);
}

export function getTaskTree(taskId: string): Promise<TaskTreeResponse> {
  return fetchApi<TaskTreeResponse>(`/tasks/${taskId}/tree`);
}

export function getTaskFiles(taskId: string, path?: string): Promise<FileEntry[]> {
  const suffix = path ? `/${encodeFilePath(path)}` : '';
  return fetchApi<FileEntry[]>(`/tasks/${taskId}/files${suffix}`, {
    cache: 'no-store',
  });
}

function encodeFilePath(path: string): string {
  return path
    .split(/[\\/]+/)
    .filter(Boolean)
    .map((segment) => encodeURIComponent(segment))
    .join('/');
}

async function fetchText(path: string, options?: RequestInit): Promise<string> {
  const response = await fetch(`${API_BASE_URL}${path}`, options);

  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  return response.text();
}

export async function getTaskFile(taskId: string, path: string): Promise<string> {
  return fetchText(`/tasks/${taskId}/files/${encodeFilePath(path)}`, {
    cache: 'no-store',
  });
}

export function getTaskLogs(
  taskId: string,
  limit?: number,
  offset?: number,
  level?: LogLevel
): Promise<string[]> {
  const params = new URLSearchParams();
  if (limit !== undefined) params.append('limit', limit.toString());
  if (offset !== undefined) params.append('offset', offset.toString());
  if (level) params.append('level', level);

  const queryString = params.toString();
  return fetchApi<string[]>(`/tasks/${taskId}/logs${queryString ? `?${queryString}` : ''}`);
}

interface MessageRecordApiResponse extends Omit<MessageRecord, 'content' | 'tool_calls'> {
  content: string | null;
  tool_calls: string | null;
}

function parseJsonStringArray<T>(value: string | null): T[] | null {
  if (!value) {
    return null;
  }

  return JSON.parse(value) as T[];
}

export async function getTaskMessages(
  taskId: string,
  limit: number,
  offset?: number,
  messageType?: MessageType
): Promise<MessageRecord[]> {
  const params = new URLSearchParams();
  params.append('limit', limit.toString());
  if (offset !== undefined) params.append('offset', offset.toString());
  if (messageType) params.append('message_type', messageType);

  const queryString = params.toString();
  const response = await fetchApi<MessageRecordApiResponse[]>(
    `/tasks/${taskId}/messages${queryString ? `?${queryString}` : ''}`
  );

  return response.map((record) => ({
    ...record,
    content: parseJsonStringArray<MessageContentPart>(record.content),
    tool_calls: parseJsonStringArray(record.tool_calls),
  }));
}

export function deleteTask(taskId: string): Promise<void> {
  return fetchApi<void>(`/tasks/${taskId}`, {
    method: 'DELETE',
  });
}

export function controlTask(taskId: string, action: TaskControlAction): Promise<void> {
  return fetchApi<void>(`/tasks/${taskId}/control`, {
    method: 'POST',
    body: JSON.stringify({ action }),
  });
}

export function steerTask(taskId: string, message: string): Promise<void> {
  const request: SteerTaskRequest = {
    content: [{ type: 'text', text: message.trim() }],
  };

  return fetchApi<void>(`/tasks/${taskId}/steer`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export function relaunchTask(taskId: string, reason: string): Promise<void> {
  return fetchApi<void>(`/tasks/${taskId}/relaunch`, {
    method: 'POST',
    body: JSON.stringify({ reason: reason.trim() }),
  });
}

export function getTask(taskId: string): Promise<Task> {
  return fetchApi<Task>(`/tasks/${taskId}`);
}

export function getTasks(params?: { status?: TaskStatus; limit?: number }): Promise<TasksResponse> {
  const queryParams = new URLSearchParams();

  if (params?.status) {
    queryParams.append('status', params.status);
  }
  if (params?.limit) {
    queryParams.append('limit', params.limit.toString());
  }

  const queryString = queryParams.toString();
  return fetchApi<TasksResponse>(`/tasks${queryString ? `?${queryString}` : ''}`);
}

export function getTaskCount(status?: TaskStatus): Promise<CountResponse> {
  const params = new URLSearchParams();
  if (status) {
    params.append('status', status);
  }
  const queryString = params.toString();
  return fetchApi<CountResponse>(`/tasks/count${queryString ? `?${queryString}` : ''}`);
}

export function getSkills(): Promise<SkillsResponse> {
  return fetchApi<SkillsResponse>('/skills');
}

export function getSkillFiles(name: string, path?: string): Promise<FileEntry[]> {
  const suffix = path ? `/${encodeFilePath(path)}` : '';
  return fetchApi<FileEntry[]>(`/skills/${encodeURIComponent(name)}/files${suffix}`, {
    cache: 'no-store',
  });
}

export function getSkillFile(name: string, path: string): Promise<string> {
  return fetchText(`/skills/${encodeURIComponent(name)}/files/${encodeFilePath(path)}`, {
    cache: 'no-store',
  });
}

export function deleteSkill(name: string): Promise<void> {
  return fetchApi<void>(`/skills/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

export function getAgents(): Promise<AgentsResponse> {
  return fetchApi<AgentsResponse>('/agents');
}

export function getChannels(): Promise<ChannelsResponse> {
  return fetchApi<ChannelsResponse>('/channels');
}

export function createChannel(channel: ChannelConfig): Promise<void> {
  return fetchApi<void>('/channels', {
    method: 'POST',
    body: JSON.stringify(channel),
  });
}

export function updateChannel(name: string, channel: ChannelConfig): Promise<void> {
  return fetchApi<void>(`/channels/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: JSON.stringify(channel),
  });
}

export function deleteChannel(name: string): Promise<void> {
  return fetchApi<void>(`/channels/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

export function getAgentFiles(name: string, path?: string): Promise<FileEntry[]> {
  const suffix = path ? `/${encodeFilePath(path)}` : '';
  return fetchApi<FileEntry[]>(`/agents/${encodeURIComponent(name)}/files${suffix}`, {
    cache: 'no-store',
  });
}

export function getAgentFile(name: string, path: string): Promise<string> {
  return fetchText(`/agents/${encodeURIComponent(name)}/files/${encodeFilePath(path)}`, {
    cache: 'no-store',
  });
}

export async function getAgent(name: string): Promise<AgentDetail | null> {
  try {
    return await fetchApi<GetAgentResponse>(`/agents/${encodeURIComponent(name)}`);
  } catch (error) {
    if (error instanceof Error && error.message.includes('404')) {
      return null;
    }
    throw error;
  }
}

export function createAgent(agent: CreateAgentRequest): Promise<void> {
  return fetchApi<void>('/agents', {
    method: 'POST',
    body: JSON.stringify(agent),
  });
}

export function updateAgent(name: string, agent: UpdateAgentRequest): Promise<void> {
  return fetchApi<void>(`/agents/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: JSON.stringify(agent),
  });
}

export function deleteAgent(name: string): Promise<void> {
  return fetchApi<void>(`/agents/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

export function getProviders(): Promise<ProvidersResponse> {
  return fetchApi<ProvidersResponse>('/providers');
}

export function createProvider(provider: ProviderConfig): Promise<void> {
  return fetchApi<void>('/providers', {
    method: 'POST',
    body: JSON.stringify(provider),
  });
}

export function updateProvider(name: string, provider: ProviderConfig): Promise<void> {
  return fetchApi<void>(`/providers/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: JSON.stringify(provider),
  });
}

export function deleteProvider(name: string): Promise<void> {
  return fetchApi<void>(`/providers/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

export function testSavedProviderConnection(
  name: string,
  model: string
): Promise<TestProviderConnectionResponse> {
  return fetchApi<TestProviderConnectionResponse>(
    `/providers/${encodeURIComponent(name)}/test`,
    {
      method: 'POST',
      body: JSON.stringify({ model }),
    }
  );
}

export const api = {
  getRunningTasksCount(): Promise<CountResponse> {
    return getTaskCount('running');
  },

  getTotalTasksCount(): Promise<CountResponse> {
    return getTaskCount();
  },

  getRunningTasks(limit: number = 20): Promise<TasksResponse> {
    return getTasks({ status: 'running', limit });
  },

  createTask(request: CreateTaskRequest): Promise<CreateTaskResponse> {
    const prompt: CreateTaskPromptPart[] = [];
    if (request.prompt.trim()) {
      prompt.push({ type: 'text', text: request.prompt.trim() });
    }
    if (request.images?.length) {
      prompt.push(...request.images);
    }

    const payload: BackendCreateTaskRequest = {
      agent: request.agent,
      description: request.description.trim(),
      prompt,
      never_ends: request.never_ends ?? false,
    };

    return fetchApi<CreateTaskResponse>('/tasks', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
  },

  getAgents,
  getAgent,
  getAgentFiles,
  getAgentFile,
  createAgent,
  updateAgent,
  deleteAgent,
  getProviders,
  getChannels,
  createChannel,
  updateChannel,
  deleteChannel,
  createProvider,
  updateProvider,
  deleteProvider,
  testSavedProviderConnection,
  getSkills,
  getSkillFiles,
  getSkillFile,
  deleteSkill,
};

export type { Skill };
