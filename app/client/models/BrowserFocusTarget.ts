export type BrowserFocusTarget = {
  kind: 'process';
  session: string;
  pid: string;
  started: string;
  view: string;
} | {
  kind: 'zoom';
  group: string;
  action: string;
} | {
  kind: 'session';
  control: string;
};
