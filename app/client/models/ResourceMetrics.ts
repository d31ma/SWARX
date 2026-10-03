import type { ResourceTotals } from '/models/ResourceTotals.ts';
export interface ResourceMetrics {
  cpu_percent: number | null;
  resident_bytes: number | null;
  received_per_second: number | null;
  sent_per_second: number | null;
  disk_read_per_second: number | null;
  disk_write_per_second: number | null;
  network_status: string;
  network_covered_processes: number;
  totals: ResourceTotals;
}
