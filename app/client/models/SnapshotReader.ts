import type { NativeViewSelection } from '/models/NativeViewSelection.ts';
import type { SnapshotPage } from '/models/SnapshotPage.ts';
import type { SnapshotTransfer } from '/models/SnapshotTransfer.ts';
export interface SnapshotReader {
  open(selection: NativeViewSelection): Promise<SnapshotTransfer>;
  page(id: number, index: number): Promise<SnapshotPage>;
  release(id: number): Promise<boolean>;
}
