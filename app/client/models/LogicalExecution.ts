export interface LogicalExecution {
  id: string;
  name: string;
  status: string;
  exit_code: number | null;
  duration_seconds: number | null;
  started_at?: number;
  reported_status?: string | null;
  task_id?: string | null;
  task_name?: string | null;
  session_id?: string | null;
  session_name?: string | null;
  source?: string | null;
  finished_at?: number | null;
  duration_source?: string | null;
}
