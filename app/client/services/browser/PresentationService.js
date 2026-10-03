@Service
export default class PresentationService {
  /** @type {import('/services/browser/GraphService.js').default | undefined} */
  @Needs
  graph;
  /** @type {import('/services/browser/ChartService.js').default | undefined} */
  @Needs
  chart;
  /** @type {import('/services/browser/ViewContinuityService.js').default | undefined} */
  @Needs
  viewContinuity;

  /** @param {HTMLElement} view @param {AbortSignal} lifecycle @param {() => import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  watch(view, lifecycle, selection) {
    this.#continuity().bind(view, lifecycle, selection);
  }

  /** @param {HTMLElement} view @param {import('/models/ViewSelection.ts').ViewSelection} selection @returns {void} */
  render(view, selection) {
    this.#graph().draw(view, selection);
    this.#chart().draw(view, selection);
  }

  /** @returns {void} */
  @OnStop
  dispose() {
    this.#continuity().dispose();
    this.#graph().dispose();
    this.#chart().dispose();
  }

  /** @returns {import('/services/browser/GraphService.js').default} */
  #graph() {
    return this.#provider(this.graph);
  }

  /** @returns {import('/services/browser/ChartService.js').default} */
  #chart() {
    return this.#provider(this.chart);
  }

  /** @returns {import('/services/browser/ViewContinuityService.js').default} */
  #continuity() {
    return this.#provider(this.viewContinuity);
  }

  /** @template T @param {T | undefined} value @returns {T} */
  #provider(value) {
    if (value === undefined) {
      throw new Error('Required browser presentation provider is unavailable.');
    }
    return value;
  }
}
