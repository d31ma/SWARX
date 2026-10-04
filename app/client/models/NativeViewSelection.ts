// Keys match the Rust NativeViewSelection field names exactly: Tac reads native call arguments by field name.
export interface NativeViewSelection {
  selected: string;
  query: string;
  pid: number | null;
  selected_session: string;
  session_pid: number | null;
  session_started: string | null;
  expanded_sessions: string[];
}
