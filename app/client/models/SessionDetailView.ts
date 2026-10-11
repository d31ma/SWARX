import type { ActivityMetricsView } from '/models/ActivityMetricsView.ts';
import type { ActivityRecord } from '/models/ActivityRecord.ts';
import type { BackgroundTaskView } from '/models/BackgroundTaskView.ts';
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
  processCount: number;
  rootCount: number;
  scopeLabel: string;
  chartId: string;
  hasResourceHistory: boolean;
  resourceEnded: boolean;
  graph: ProcessGraph;
  treeLabel: string;
  background: BackgroundTaskView[];
  activity: ActivityRecord[];
  activityLabel: string;
  activityMetrics: ActivityMetricsView;
}
