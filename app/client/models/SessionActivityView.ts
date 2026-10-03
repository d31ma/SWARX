import type { ActivityRecord } from '/models/ActivityRecord.ts';

export interface SessionActivityView {
  id: string;
  name: string;
  family: string;
  session_id: string | null;
  session_name: string | null;
  source: string | null;
  label_reported_at: number | null;
  records: ActivityRecord[];
  captured_at: number;
}
