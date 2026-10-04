// The page copy of NativeBackgroundTaskView; process IDs stay native.
export interface BackgroundTaskView {
  key: string;
  session_key: string;
  title: string;
  action: string | null;
  state: string;
  state_label: string;
  started_label: string;
  process_count: number;
  attribution: string;
}
