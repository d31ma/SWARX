import type { TerminalRecord } from '/models/TerminalRecord.ts';
export interface TerminalView extends TerminalRecord {
  duration: string;
  result: string;
  statusClass: string;
}
