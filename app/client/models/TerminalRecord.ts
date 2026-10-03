export interface TerminalRecord {
  started_at: number;
  finished_at: number | null;
  exit_code: number | null;
  status: string;
  name: string;
}
