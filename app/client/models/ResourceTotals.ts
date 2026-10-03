export interface ResourceTotals {
  cpu_seconds: number;
  received_bytes: number;
  sent_bytes: number;
  disk_read_bytes: number;
  disk_written_bytes: number;
  peak_resident_bytes: number;
  resident_bytes: number;
  samples: number;
}
