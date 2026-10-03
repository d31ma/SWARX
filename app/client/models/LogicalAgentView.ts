import type { LogicalAgent } from '/models/LogicalAgent.ts';
import type { CommandResourceView } from '/models/CommandResourceView.ts';
export interface LogicalAgentView extends LogicalAgent {
  feedLabel: string;
  taskLabel: string;
  taskState: string;
  capturedLabel: string;
  ageLabel: string;
  currentAction: string | null;
  actionLabel: string;
  actionTiming: string | null;
  command_resources: CommandResourceView[];
}
