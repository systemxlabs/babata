// API 类型定义

export interface Task {
  task_id: string;
  description: string;
  agent: string;
  status: 'running' | 'completed' | 'failed' | 'canceled' | 'paused';
  parent_task_id?: string | null;
  root_task_id: string;
  created_at: number;
  never_ends: boolean;
  subtask_count: number;
}

export interface Agent {
  name: string;
  description?: string;
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
