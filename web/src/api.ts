// API 客户端
import type {
  Agent,
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

// ========== Agent CRUD API 函数 ==========

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
