// API 客户端
import type {
  CountResponse,
  TasksResponse,
  AgentsResponse,
  SkillsResponse,
  CreateTaskRequest,
  CreateTaskResponse,
} from './types';

const API_BASE = '/api';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, options);
  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

export const api = {
  // 获取运行中任务数量
  getRunningTasksCount(): Promise<CountResponse> {
    return fetchJson<CountResponse>(`${API_BASE}/tasks/count?status=running`);
  },

  // 获取总任务数量
  getTotalTasksCount(): Promise<CountResponse> {
    return fetchJson<CountResponse>(`${API_BASE}/tasks/count`);
  },

  // 获取 Agent 列表
  getAgents(): Promise<AgentsResponse> {
    return fetchJson<AgentsResponse>(`${API_BASE}/agents`);
  },

  // 获取 Skill 列表
  getSkills(): Promise<SkillsResponse> {
    return fetchJson<SkillsResponse>(`${API_BASE}/skills`);
  },

  // 获取运行中的任务列表
  getRunningTasks(limit: number = 20): Promise<TasksResponse> {
    return fetchJson<TasksResponse>(`${API_BASE}/tasks?status=running&limit=${limit}`);
  },

  // 创建新任务
  createTask(request: CreateTaskRequest): Promise<CreateTaskResponse> {
    return fetchJson<CreateTaskResponse>(`${API_BASE}/tasks`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  },
};
