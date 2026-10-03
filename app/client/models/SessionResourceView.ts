import type { SessionView } from '/models/SessionView.ts';
import type { RuntimeView } from '/models/RuntimeView.ts';
import type { ProcessRow } from '/models/ProcessRow.ts';
import type { ProcessGraph } from '/models/ProcessGraph.ts';

export interface SessionResourceView extends Omit<SessionView, 'agents'> {
  sourceRecordCount: number;
  commands: RuntimeView[];
  tree: ProcessRow[];
  graph: ProcessGraph;
  resourceNotice: string;
  commandCount: number;
  processCount: number;
}
