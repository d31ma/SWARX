import type { ProcessGraph } from '/models/ProcessGraph.ts';
import type { ProcessSnapshot } from '/models/ProcessSnapshot.ts';
import type { ResourceMetrics } from '/models/ResourceMetrics.ts';
import type { TaskRecord } from '/models/TaskRecord.ts';
import type { TerminalRecord } from '/models/TerminalRecord.ts';
import type { ProcessRow } from '/models/ProcessRow.ts';
import type { ProcessOwner } from '/models/ProcessOwner.ts';
export interface GroupSnapshot {
  owners?: ProcessOwner[];
  id: string;
  pid: number;
  name: string;
  family: string;
  status: string;
  scope: string;
  shared: boolean;
  instrumented: boolean;
  processes: ProcessSnapshot[];
  tree: ProcessRow[];
  graph: ProcessGraph;
  metrics: ResourceMetrics | null;
  tasks: TaskRecord[];
  terminals: TerminalRecord[];
  history: Array<Array<number | null>>;
}
