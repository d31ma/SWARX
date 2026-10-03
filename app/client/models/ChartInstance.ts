import type { ChartDataset } from '/models/ChartDataset.ts';
export interface ChartInstance {
  canvas: HTMLCanvasElement;
  data: { datasets: ChartDataset[] };
  options: Record<string, unknown>;
  update: (mode: string) => void;
  destroy: () => void;
}
