import type { ChartDataset } from '/models/ChartDataset.ts';
export interface ChartConfiguration {
  type: 'line';
  data: { datasets: ChartDataset[] };
  options: Record<string, unknown>;
}
