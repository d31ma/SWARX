import type { AgentRowView } from '/models/AgentRowView.ts';
import type { BackgroundTaskView } from '/models/BackgroundTaskView.ts';

export interface ActiveSessionView {
  key: string;
  id: string | null;
  title: string;
  family: string;
  harness: string;
  linked: boolean;
  agentCount: number;
  subAgentCount: number;
  toolCount: number;
  agentTotal: number;
  countLabel: string;
  collapsible: boolean;
  expanded: boolean;
  toggleLabel: string;
  agents: AgentRowView[];
  background: BackgroundTaskView[];
  backgroundRunning: number;
  backgroundWaiting: number;
}
