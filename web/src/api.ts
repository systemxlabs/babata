import type { 
  FileEntry, 
  Task, 
  TaskStatus, 
  TaskFilter, 
  TaskListResponse, 
  TaskControlAction 
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
    throw new Error(`API Error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

// 获取根任务列表（支持分页和筛选）
export function getRootTasks(filter: TaskFilter): Promise<TaskListResponse> {
  const params = new URLSearchParams();
  
  if (filter.status && filter.status !== 'all') {
    params.append('status', filter.status);
  }
  if (filter.agent && filter.agent !== 'all') {
    params.append('agent', filter.agent);
  }
  if (filter.search) {
    params.append('search', filter.search);
  }
  params.append('page', filter.page.toString());
  params.append('page_size', filter.pageSize.toString());

  const queryString = params.toString();
  return fetchApi<TaskListResponse>(`/tasks/roots${queryString ? `?${queryString}` : ''}`);
}

// 获取子任务
export function getTaskChildren(taskId: string): Promise<{ children: Task[] }> {
  return fetchApi<{ children: Task[] }>(`/tasks/${taskId}/children`);
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
export function getAgents(): Promise<{ name: string; description: string }[]> {
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
