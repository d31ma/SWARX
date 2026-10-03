export interface ProcessSnapshot {
  pid: number;
  parent: number;
  started: string;
  name: string;
  cpu_percent: number | null;
  resident_bytes: number | null;
  received_per_second: number | null;
  sent_per_second: number | null;
  disk_read_per_second: number | null;
  disk_write_per_second: number | null;
}
