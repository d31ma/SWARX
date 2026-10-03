import type { LogicalExecution } from '/models/LogicalExecution.ts';
export interface LogicalExecutionView extends LogicalExecution {
  result: string;
}
