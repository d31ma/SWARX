export interface AgentRowView {
  id: string;
  subAgent: boolean;
  roleLabel: string;
  taskLabel: string;
  taskId: string;
  status: string;
  family: string;
  source: string;
  sessionId: string;
  toolLabel: string;
  actionDetail: string | null;
  toolCount: number;
}
