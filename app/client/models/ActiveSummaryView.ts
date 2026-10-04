import type { SummaryCardView } from '/models/SummaryCardView.ts';

export interface ActiveSummaryView {
  current: boolean;
  agents: number;
  subAgents: number;
  sessions: number;
  tools: number;
  backgroundRunning: number;
  backgroundWaiting: number;
  harnesses: string[];
  cards: SummaryCardView[];
}
