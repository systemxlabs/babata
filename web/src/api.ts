// API 客户端
import type {
  Agent,
  FileEntry,
  Task,
  TaskStatus,
  TaskFilter,
  TaskListResponse,
  TaskControlAction,
  CountResponse,
  TasksResponse,
  AgentsResponse,
  SkillsResponse,
  CreateTaskRequest,
  CreateTaskResponse,
  CreateAgentRequest,
  UpdateAgentRequest,
  GetAgentResponse,
  ListAgentsResponse,
} from './types';

const API_BASE_URL = '/api';

// 通用请求函数
async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${url}`, {
    headers: {
      'Content-Type': 'application/json',
    },
    ...options,
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API Error: ${response.status} ${response.statusText}${errorText ? ' - ' + errorText : ''}`);
  }

  // 处理空响应 (204 No Content)
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// 任务树响应类型
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

// 获取根任务列表（支持分页和筛选）
export function getRootTasks(filter: TaskFilter): Promise<TaskListResponse> {
  const params = new URLSearchParams();

  if (filter.status && filter.status !== 'all') {
    params.append('status', filter.status);
  }
  // 注意：主分支 API 目前不支持 agent 筛选和搜索
  params.append('page', filter.page.toString());
  params.append('page_size', filter.pageSize.toString());

  const queryString = params.toString();
  return fetchApi<TaskListResponse>(`/tasks${queryString ? `?${queryString}` : ''}`);
}

// 获取任务树（包含所有层级的子任务）
export function getTaskTree(taskId: string): Promise<TaskTreeResponse> {
  return fetchApi<TaskTreeResponse>(`/tasks/${taskId}/tree`);
}

// 获取任务文件列表
export function getTaskFiles(taskId: string): Promise<FileEntry[]> {
  return fetchApi<FileEntry[]>(`/tasks/${taskId}/files`);
}

// 获取文件内容
export async function getTaskFile(taskId: string, path: string): Promise<string> {
  const response = await fetch(`${API_BASE_URL}/tasks/${taskId}/files/${path}`, {
    headers: {
      'Content-Type': 'application/json',
    },
  });

  if (!response.ok) {
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  // 返回纯文本内容
  return response.text();
}

// 获取任务日志
export function getTaskLogs(
  taskId: string,
  limit?: number,
  offset?: number
): Promise<string[]> {
  const params = new URLSearchParams();
  if (limit !== undefined) params.append('limit', limit.toString());
  if (offset !== undefined) params.append('offset', offset.toString());

  const queryString = params.toString();
  return fetchApi<string[]>(`/tasks/${taskId}/logs${queryString ? `?${queryString}` : ''}`);
}

// 删除任务
export function deleteTask(taskId: string): Promise<void> {
  return fetchApi<void>(`/tasks/${taskId}`, {
    method: 'DELETE',
  });
}

// 删除技能
export function deleteSkill(name: string): Promise<void> {
  return fetchApi<void>(`/skills/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

// 控制任务（暂停/恢复/取消）
export function controlTask(taskId: string, action: TaskControlAction): Promise<void> {
  return fetchApi<void>(`/tasks/${taskId}/control`, {
    method: 'POST',
    body: JSON.stringify({ action }),
  });
}

// 获取单个任务详情
export function getTask(taskId: string): Promise<Task> {
  return fetchApi<Task>(`/tasks/${taskId}`);
}

// 获取任务列表（旧接口，保留兼容）
export function getTasks(params?: { status?: TaskStatus; limit?: number }): Promise<Task[]> {
  const queryParams = new URLSearchParams();

  if (params?.status) {
    queryParams.append('status', params.status);
  }
  if (params?.limit) {
    queryParams.append('limit', params.limit.toString());
  }

  const queryString = queryParams.toString();
  return fetchApi<Task[]>(`/tasks${queryString ? `?${queryString}` : ''}`);
}

// 获取所有 Agent 列表（用于筛选）
export function getAgentsList(): Promise<{ name: string; description: string }[]> {
  return fetchApi<{ agents: { name: string; description: string }[] }>('/agents')
    .then(res => res.agents);
}

// 获取任务数量统计
export function getTaskCount(status?: TaskStatus): Promise<{ count: number }> {
  const params = new URLSearchParams();
  if (status) {
    params.append('status', status);
  }
  const queryString = params.toString();
  return fetchApi<{ count: number }>(`/tasks/count${queryString ? `?${queryString}` : ''}`);
}

// API 对象（兼容主分支的 Dashboard 组件）
export const api = {
  // 获取运行中任务数量
  getRunningTasksCount(): Promise<CountResponse> {
    return fetchApi<CountResponse>(`/tasks/count?status=running`);
  },

  // 获取总任务数量
  getTotalTasksCount(): Promise<CountResponse> {
    return fetchApi<CountResponse>(`/tasks/count`);
  },

  // 获取 Agent 列表
  getAgents(): Promise<AgentsResponse> {
    return fetchApi<AgentsResponse>(`/agents`);
  },

  // 获取 Skill 列表
  getSkills(): Promise<SkillsResponse> {
    return fetchApi<SkillsResponse>(`/skills`);
  },

  // 获取运行中的任务列表
  getRunningTasks(limit: number = 20): Promise<TasksResponse> {
    return fetchApi<TasksResponse>(`/tasks?status=running&limit=${limit}`);
  },

  // 创建新任务
  createTask(request: CreateTaskRequest): Promise<CreateTaskResponse> {
    return fetchApi<CreateTaskResponse>(`/tasks`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  },

  // 删除技能
  deleteSkill(name: string): Promise<void> {
    return fetchApi<void>(`/skills/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
  },

  // 获取单个 Agent 详情
  async getAgent(name: string): Promise<Agent | null> {
    try {
      const response = await fetchApi<GetAgentResponse>(`/agents/${encodeURIComponent(name)}`);
      return response;
    } catch (error) {
      if (error instanceof Error && error.message.includes('404')) {
        return null;
      }
      throw error;
    }
  },

  // 创建 Agent
  createAgent(agent: CreateAgentRequest): Promise<void> {
    return fetchApi<void>('/agents', {
      method: 'POST',
      body: JSON.stringify(agent),
    });
  },

  // 更新 Agent
  updateAgent(name: string, agent: UpdateAgentRequest): Promise<void> {
    return fetchApi<void>(`/agents/${encodeURIComponent(name)}`, {
      method: 'PUT',
      body: JSON.stringify(agent),
    });
  },

  // 删除 Agent
  deleteAgent(name: string): Promise<void> {
    return fetchApi<void>(`/agents/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
  },
};

// ========== Agent CRUD API 函数 (独立导出) ==========

/**
 * 获取单个 Agent 详情
 * @param name Agent 名称
 * @returns Agent 详情或 null (如果不存在)
 */
export async function getAgent(name: string): Promise<Agent | null> {
  try {
    const response = await fetchApi<GetAgentResponse>(`/agents/${encodeURIComponent(name)}`);
    return response;
  } catch (error) {
    if (error instanceof Error && error.message.includes('404')) {
      return null;
    }
    throw error;
  }
}

/**
 * 创建 Agent
 * @param agent 创建 Agent 的请求数据
 */
export async function createAgent(agent: CreateAgentRequest): Promise<void> {
  return fetchApi<void>('/agents', {
    method: 'POST',
    body: JSON.stringify(agent),
  });
}

/**
 * 更新 Agent
 * @param name Agent 名称
 * @param agent 更新 Agent 的请求数据
 */
export async function updateAgent(name: string, agent: UpdateAgentRequest): Promise<void> {
  return fetchApi<void>(`/agents/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: JSON.stringify(agent),
  });
}

/**
 * 删除 Agent
 * @param name Agent 名称
 */
export async function deleteAgent(name: string): Promise<void> {
  return fetchApi<void>(`/agents/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  });
}

/**
 * 列出所有 Agents
 * @returns Agent 列表
 */
export async function listAgents(): Promise<Agent[]> {
  const response = await fetchApi<ListAgentsResponse>('/agents');
  return response.agents;
}
