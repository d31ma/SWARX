import type { LogicalAgentView } from '/models/LogicalAgentView.ts';
export interface SessionView {
  key: string;
  id: string | null;
  name: string;
  family: string;
  source: string;
  captured_at: number | null;
  known: boolean;
  agents: LogicalAgentView[];
}
