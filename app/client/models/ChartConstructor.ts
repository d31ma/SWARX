import type { ChartInstance } from '/models/ChartInstance.ts';
import type { ChartConfiguration } from '/models/ChartConfiguration.ts';
export interface ChartConstructor {
  new(canvas: HTMLCanvasElement, configuration: ChartConfiguration): ChartInstance;
}
