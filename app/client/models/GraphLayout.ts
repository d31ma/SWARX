import type { GraphLayoutBox } from '/models/GraphLayoutBox.ts';

export interface GraphLayout {
  boxes: Map<string, GraphLayoutBox>;
  width: number;
  height: number;
}
