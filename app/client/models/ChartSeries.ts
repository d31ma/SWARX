import type { ChartPoint } from '/models/ChartPoint.ts';
export interface ChartSeries {
  id: string;
  label: string;
  points: ChartPoint[];
  measuredCount: number;
}
