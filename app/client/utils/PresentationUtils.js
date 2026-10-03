import { ViewContinuity } from '../shared/scripts/ViewContinuity.js';
import { ChartRenderer } from '../shared/scripts/ChartRenderer.js';

export class PresentationUtils {
  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @param {() => import('/models/ViewSelection.ts').ViewSelection} selection @returns {ViewContinuity} */
  static watch(view, lifecycle, selection) {
    const watcher = new ViewContinuity(view, lifecycle, selection);
    watcher.start();
    return watcher;
  }

  /** @param {ViewContinuity | null} watcher @returns {void} */
  static disposeWatcher(watcher) {
    watcher?.dispose();
    ChartRenderer.dispose();
  }

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  static charts(view, selection) {
    ChartRenderer.draw(view, selection);
  }

}
