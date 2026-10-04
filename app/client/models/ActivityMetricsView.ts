import type { ActivityFactView } from '/models/ActivityFactView.ts';
import type { ActivityToolView } from '/models/ActivityToolView.ts';

export interface ActivityMetricsView {
  available: boolean;
  average: string;
  averageLabel: string;
  facts: ActivityFactView[];
  tools: ActivityToolView[];
  toolLabel: string;
}
