import type { ProcessRow } from '/models/ProcessRow.ts';
export interface ProcessGraphNode extends ProcessRow {
  identity: string;
  parentIdentity: string | null;
  childIdentities: string[];
  graphDepth: number;
  edgeStatus: string;
  edgeLabel: string;
}
