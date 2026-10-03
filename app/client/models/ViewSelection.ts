import type { RuntimeView } from '/models/RuntimeView.ts';
import type { ObservatoryModel } from '/models/ObservatoryModel.ts';
export interface ViewSelection {
  group: RuntimeView | null;
  mode: string;
  model?: ObservatoryModel;
  pid?: number;
  graphGroup?: string;
  session?: string;
  sessionPid?: number | null;
  sessionStarted?: string | null;
  showGraph?: (event: Event, group: string) => void;
  pick?: (event: Event, group: string, process: number) => void;
}
