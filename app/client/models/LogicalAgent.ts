import type { SessionLink } from '/models/SessionLink.ts';
import type { TaskRecord } from '/models/TaskRecord.ts';
import type { LogicalExecution } from '/models/LogicalExecution.ts';
export interface LogicalAgent {
  session: SessionLink | null;
  id: string;
  name: string;
  family: string;
  status: string;
  source: string;
  captured_at: number;
  feed_mode: string;
  task: TaskRecord | null;
  executions: LogicalExecution[];
  age_seconds: number;
  stale: boolean;
}
