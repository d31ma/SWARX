import type { TaskRecord } from '/models/TaskRecord.ts';
export interface TaskView extends TaskRecord {
  dependencies: string;
  statusClass: string;
}
