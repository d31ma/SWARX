import type { ActivityRecord } from '/models/ActivityRecord.ts';
import type { AgentRowView } from '/models/AgentRowView.ts';
import type { LogicalAgentView } from '/models/LogicalAgentView.ts';
import type { ProcessGraph } from '/models/ProcessGraph.ts';
import type { RuntimeView } from '/models/RuntimeView.ts';

export interface AgentDetailView {
  id: string;
  agent: LogicalAgentView;
  title: string;
  harness: string;
  row: AgentRowView;
  active: boolean;
  stateLabel: string;
  sessionKey: string;
  resourceScope: string;
  resourceLabel: string;
  resources: RuntimeView[];
  graphKey: string;
  graph: ProcessGraph;
  processCount: number;
  treeLabel: string;
  activity: ActivityRecord[];
  activityLabel: string;
}
