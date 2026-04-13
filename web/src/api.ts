import type {
  AgentDetail,
  AgentsResponse,
  ChannelConfig,
  ChannelsResponse,
  CountResponse,
  CreateAgentRequest,
  CreateTaskRequest,
  CreateTaskResponse,
  FileEntry,
  GetAgentResponse,
  ListAgentsResponse,
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
  TasksResponse,
  TextContent,
  UpdateAgentRequest,
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
  prompt: TextContent[];
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

export function getTaskFiles(taskId: string): Promise<FileEntry[]> {
  return fetchApi<FileEntry[]>(`/tasks/${taskId}/files`);
}

function encodeFilePath(path: string): string {
  return path
    .split(/[\\/]+/)
    .filter(Boolean)
    .map((segment) => encodeURIComponent(segment))
    .join('/');
}

async function fetchText(path: string): Promise<string> {
  const response = await fetch(`${API_BASE_URL}${path}`);

  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  return response.text();
}

export async function getTaskFile(taskId: string, path: string): Promise<string> {
  return fetchText(`/tasks/${taskId}/files/${encodeFilePath(path)}`);
}

export function getTaskLogs(taskId: string, limit?: number, offset?: number): Promise<string[]> {
  const params = new URLSearchParams();
  if (limit !== undefined) params.append('limit', limit.toString());
  if (offset !== undefined) params.append('offset', offset.toString());

  const queryString = params.toString();
  return fetchApi<string[]>(`/tasks/${taskId}/logs${queryString ? `?${queryString}` : ''}`);
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

export function getSkillFiles(name: string): Promise<FileEntry[]> {
  return fetchApi<FileEntry[]>(`/skills/${encodeURIComponent(name)}/files`);
}

export function getSkillFile(name: string, path: string): Promise<string> {
  return fetchText(`/skills/${encodeURIComponent(name)}/files/${encodeFilePath(path)}`);
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

export function getAgentFiles(name: string): Promise<FileEntry[]> {
  return fetchApi<FileEntry[]>(`/agents/${encodeURIComponent(name)}/files`);
}

export function getAgentFile(name: string, path: string): Promise<string> {
  return fetchText(`/agents/${encodeURIComponent(name)}/files/${encodeFilePath(path)}`);
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

export async function listAgents(): Promise<ListAgentsResponse['agents']> {
  const response = await fetchApi<ListAgentsResponse>('/agents');
  return response.agents;
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

export async function listProviders(): Promise<ProviderConfig[]> {
  const response = await fetchApi<ProvidersResponse>('/providers');
  return response.providers;
}

export async function listChannels(): Promise<ChannelConfig[]> {
  const response = await fetchApi<ChannelsResponse>('/channels');
  return response.channels;
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
    const payload: BackendCreateTaskRequest = {
      agent: request.agent,
      description: request.description.trim(),
      prompt: [{ type: 'text', text: request.prompt.trim() }],
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
  getSkills,
  getSkillFiles,
  getSkillFile,
  deleteSkill,
};

export type { Skill };
