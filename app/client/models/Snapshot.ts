import type { RuntimeView } from '/models/RuntimeView.ts';
import type { ResourceTotals } from '/models/ResourceTotals.ts';
import type { LogicalAgentView } from '/models/LogicalAgentView.ts';
import type { SessionView } from '/models/SessionView.ts';
import type { SessionActivityView } from '/models/SessionActivityView.ts';
export interface Snapshot {
  groups: RuntimeView[];
  logical_agents: LogicalAgentView[];
  session_groups: SessionView[];
  linked_session_count: number;
  harness_count: number;
  activity?: SessionActivityView[];
  totals: ResourceTotals;
  started_at: number;
  sampled_at: number;
  errors: string[];
  ready: boolean;
  limitations: string[];
}
