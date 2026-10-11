import type { AgentRowView } from '/models/AgentRowView.ts';
import type { BackgroundTaskView } from '/models/BackgroundTaskView.ts';
import type { SessionLoadView } from '/models/SessionLoadView.ts';

export interface ActiveSessionView {
  key: string;
  id: string | null;
  title: string;
  family: string;
  harness: string;
  linked: boolean;
  remoteControlId: string | null;
  idle: boolean;
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
  loads: SessionLoadView[];
}
