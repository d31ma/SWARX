export interface ProcessOwner {
  agent_id: string;
  agent_name: string;
  session_id: string;
  session_name: string;
  task_id: string;
  execution_id: string;
  binding_id: string;
  generation: number;
  pid: number;
  started: string;
  root_pid: number;
  root_started: string;
  scope: string;
  scope_label: string;
  validation: string;
}
