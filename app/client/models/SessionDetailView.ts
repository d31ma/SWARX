import type { ActivityRecord } from '/models/ActivityRecord.ts';
import type { ProcessGraph } from '/models/ProcessGraph.ts';
import type { SessionMetricView } from '/models/SessionMetricView.ts';

export interface SessionDetailView {
  key: string;
  id: string | null;
  title: string;
  harness: string;
  family: string;
  linked: boolean;
  active: boolean;
  stateLabel: string;
  countLabel: string;
  agentCount: number;
  subAgentCount: number;
  resourceScope: string;
  resourceLabel: string;
  metrics: SessionMetricView[];
  notes: string[];
  processCount: number;
  rootCount: number;
  chartId: string;
  graph: ProcessGraph;
  treeLabel: string;
  activity: ActivityRecord[];
  activityLabel: string;
}
