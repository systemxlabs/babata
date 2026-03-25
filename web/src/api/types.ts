export type TaskStatus = 'running' | 'paused' | 'done' | 'canceled';

export interface TaskActions {
  pause: boolean;
  resume: boolean;
  cancel: boolean;
  relaunch: boolean;
}

export interface TaskSummary {
  task_id: string;
  description: string;
  agent: string | null;
  status: TaskStatus;
  actions: TaskActions;
  parent_task_id: string | null;
  root_task_id: string;
  created_at: number;
  never_ends: boolean;
}

export interface TaskListResponse {
  tasks: TaskSummary[];
}

export interface OverviewResponse {
  status_counts: {
    total: number;
    running: number;
    paused: number;
    canceled: number;
    done: number;
  };
  recent_tasks: TaskSummary[];
}

export interface SystemResponse {
  version: string;
  http_addr: string;
}

export interface TaskContentResponse {
  task_id: string;
  task_markdown: string;
  progress_markdown: string;
}

export interface TaskTreeResponse {
  root_task_id: string;
  parent: TaskSummary | null;
  current: TaskSummary;
  children: TaskSummary[];
}

export interface TaskArtifact {
  path: string;
  size_bytes: number;
  is_text: boolean;
  text_preview: string | null;
}

export interface TaskArtifactsResponse {
  task_id: string;
  artifacts: TaskArtifact[];
}

export interface TaskLogEntry {
  path: string;
  content: string;
}

export interface TaskLogsResponse {
  task_id: string;
  supported: boolean;
  entries?: TaskLogEntry[];
  reason?: string;
}
