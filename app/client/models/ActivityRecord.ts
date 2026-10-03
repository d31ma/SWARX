export interface ActivityRecord {
  id: string;
  agent_id: string;
  agent_name: string;
  agent_family: string;
  freshness: string;
  reported_at: number;
  name: string;
  status: string;
  status_label: string;
  running: boolean;
  reported_status: string | null;
  session_id: string | null;
  session_name: string | null;
  task_id: string | null;
  task_name: string | null;
  source: string | null;
  started_label: string | null;
  finished_label: string | null;
  duration_label: string | null;
  duration_note: string | null;
  elapsed_label: string | null;
  exit_code: number | null;
  captured_at: number;
}
