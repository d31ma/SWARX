import type { ProcessGraphNode } from '/models/ProcessGraphNode.ts';
export interface ProcessGraph {
  nodes: ProcessGraphNode[];
  roots: string[];
}
