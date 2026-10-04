import type { RuntimeView } from '/models/RuntimeView.ts';
import type { ObservatoryModel } from '/models/ObservatoryModel.ts';
export interface ViewSelection {
  group: RuntimeView | null;
  mode: string;
  model?: ObservatoryModel;
  pid?: number;
  session?: string;
  sessionPid?: number | null;
  sessionStarted?: string | null;
  showTab?: (event: Event, tab: string) => void;
  close?: (event: Event) => void;
  pick?: (event: Event, group: string, process: number) => void;
}
