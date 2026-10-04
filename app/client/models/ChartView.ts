import type { ChartSeries } from '/models/ChartSeries.ts';
export interface ChartView {
  mode: string;
  unit: string;
  windowStart: number;
  windowEnd: number;
  sampleCount: number;
  measuredCount: number;
  empty: boolean;
  emptyText: string;
  series: ChartSeries[];
  filled?: boolean;
  legend?: boolean;
  zeroBased?: boolean;
  ceiling?: number | null;
}
