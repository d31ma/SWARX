import type { ProcessSnapshot } from '/models/ProcessSnapshot.ts';
import type { CommandOwner } from '/models/CommandOwner.ts';
import type { ResourceTotals } from '/models/ResourceTotals.ts';
import type { ProcessRow } from '/models/ProcessRow.ts';
export interface CommandResourceView {
  binding_id: string;
  generation: number;
  owner: CommandOwner;
  pid: number;
  started: string;
  sampled_at: number;
  scope: string;
  provenance: string;
  validation: string;
  coverage: string;
  metrics: ProcessSnapshot | null;
  descendants: ProcessSnapshot[];
  descendant_rows: ProcessRow[];
  totals: ResourceTotals;
  history: Array<Array<number | null>>;
  name: string;
  cpu: string;
  memory: string;
  diskRead: string;
  diskWrite: string;
  sampleLabel: string;
  trustLabel: string;
}
