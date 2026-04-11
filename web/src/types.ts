// API 类型定义

// Agent 类型
export interface Agent {
  name: string;
  description: string;
  provider: string;
  model: string;
  allowed_tools: string[];
  default: boolean;
  body: string;
}

// 创建 Agent 请求
export interface CreateAgentRequest {
  name: string;
  description: string;
  provider: string;
  model: string;
  allowed_tools: string[];
  default: boolean;
  body: string;
}

// 更新 Agent 请求
export interface UpdateAgentRequest {
  description: string;
  provider: string;
  model: string;
  allowed_tools: string[];
  default: boolean;
  body: string;
}

// 获取 Agent 响应
export type GetAgentResponse = Agent;

// Agent 列表响应
export interface ListAgentsResponse {
  agents: Agent[];
}
