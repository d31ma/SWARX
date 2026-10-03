export interface TaskRecord {
  id: string;
  name: string;
  status: string;
  depends_on: string[];
}
