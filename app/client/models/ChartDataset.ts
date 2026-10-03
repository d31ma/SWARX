import type { ChartPoint } from '/models/ChartPoint.ts';
export interface ChartDataset {
  label: string;
  data: ChartPoint[];
  borderColor: string;
  backgroundColor: string;
  borderWidth: number;
  pointRadius: number;
  pointHoverRadius: number;
  pointHitRadius: number;
  spanGaps: boolean;
  tension: number;
  fill: boolean;
}
