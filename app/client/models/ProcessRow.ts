import type { ProcessSnapshot } from '/models/ProcessSnapshot.ts';
import type { ProcessOwner } from '/models/ProcessOwner.ts';
export interface ProcessRow extends ProcessSnapshot {
  owner?: ProcessOwner | null;
  depth: number;
  indentClass: string;
  memory: string;
  cpu: string;
  networkLabel: string;
  diskLabel: string;
  relationship: string;
}
