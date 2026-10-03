import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {test} from 'node:test';

/** @typedef {import('../../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {import('../../app/client/models/ChartView.ts').ChartView} ChartView */
/** @typedef {import('../../app/client/models/RuntimeView.ts').RuntimeView} RuntimeView */
/** @typedef {import('../../app/client/models/ChartInstance.ts').ChartInstance} ChartInstance */
/** @typedef {{draw: (view: HTMLElement, selection: ViewSelection) => void, dispose: () => void}} ChartService */

class ChartCanvas {
  /** @param {string} identifier */
  constructor(identifier) {
    /** @type {{group: string, points?: string}} */
    this.dataset = {group: identifier};
    /** @type {ChartCard} */
    this.card = new ChartCard(this);
    /** @type {ChartCard} */
    this.parentNode = this.card;
  }
  /** @param {string} selector @returns {ChartCard | null} */
  closest(selector) { return selector === '.history-card' ? this.card : null; }
}

class ChartCard {
  /** @param {ChartCanvas} canvas */
  constructor(canvas) {
    /** @type {ChartCanvas} */
    this.canvas = canvas;
    /** @type {{textContent: string, hidden: boolean}} */
    this.empty = {textContent: '', hidden: false};
  }
  /** @param {string} selector @returns {{textContent: string, hidden: boolean} | null} */
  querySelector(selector) { return selector === '.chart-empty' ? this.empty : null; }
  /** @param {ChartCanvas} previous @param {ChartCanvas} replacement @returns {void} */
  replaceChild(previous, replacement) { this.canvas = previous; previous.card = this; previous.parentNode = this; }
}

class ChartBoundary {
  /** @type {ChartBoundary[]} */
  static created = [];
  /** @param {ChartCanvas} canvas @param {import('../../app/client/models/ChartConfiguration.ts').ChartConfiguration} configuration */
  constructor(canvas, configuration) {
    /** @type {ChartCanvas} */
    this.canvas = canvas;
    /** @type {import('../../app/client/models/ChartConfiguration.ts').ChartConfiguration['data']} */
    this.data = configuration.data;
    /** @type {Record<string, unknown>} */
    this.options = configuration.options;
    /** @type {string[]} */
    this.updates = [];
    /** @type {number} */
    this.destroyed = 0;
    ChartBoundary.created.push(this);
  }
  /** @param {string} mode @returns {void} */
  update(mode) { this.updates.push(mode); }
  /** @returns {void} */
  destroy() { this.destroyed += 1; }
}

class ChartServiceTest {
  /** @returns {new () => ChartService} */
  static serviceClass() {
    const url = new URL('../../app/client/services/browser/ChartService.js', import.meta.url);
    const source = readFileSync(url, 'utf8')
      .replace(/^\s*@(?:Service|OnStop)\s*$/gm, '')
      .replace('export default ', '');
    const context = vm.createContext({window: {Chart: ChartBoundary}, Date, Intl, Map, Set});
    vm.runInContext(source + '\nglobalThis.ServiceClass = ChartService;', context, {filename: url.href});
    return context.ServiceClass;
  }
  /** @param {string} [mode] @param {number | null} [value] @returns {ChartView} */
  static projection(mode = 'cpu', value = 7) {
    return {mode, unit: '%', windowStart: 100, windowEnd: 220, sampleCount: 2, measuredCount: 1,
      empty: false, emptyText: '', series: [{id: 'cpu', label: 'Measured CPU', measuredCount: 1, points: [{x: 120, y: value}]}]};
  }
  /** @param {string} identifier @param {ChartView[]} charts @returns {RuntimeView} */
  static resource(identifier, charts) { return /** @type {RuntimeView} */ ({id: identifier, charts}); }
  /** @param {RuntimeView[]} resources @param {string} [mode] @returns {ViewSelection} */
  static selection(resources, mode = 'cpu') {
    return /** @type {ViewSelection} */ (/** @type {unknown} */ ({group: null, mode, model: {chartResources: resources, runtimes: []}}));
  }
  /** @param {ChartCanvas[]} canvases @returns {HTMLElement} */
  static view(canvases) {
    return /** @type {HTMLElement} */ (/** @type {unknown} */ ({
      /** @param {string} selector @returns {ChartCanvas[]} */
      querySelectorAll(selector) { return selector === 'canvas.resource-chart' ? canvases : []; }
    }));
  }
  /** @returns {{service: ChartService, canvas: ChartCanvas, resource: RuntimeView}} */
  static fixture() {
    ChartBoundary.created = [];
    return {service: new (ChartServiceTest.serviceClass())(), canvas: new ChartCanvas('session:root'),
      resource: ChartServiceTest.resource('session:root', [ChartServiceTest.projection()])};
  }
}

test('chart resources draw root history and separate context without using runtime fallback', /** @returns {void} */ () => {
  const {service, canvas, resource} = ChartServiceTest.fixture();
  const context = new ChartCanvas('session:context');
  const missing = new ChartCanvas('runtime-only');
  const selection = ChartServiceTest.selection([resource, ChartServiceTest.resource('session:context', [ChartServiceTest.projection('cpu', 19)])]);
  /** @type {import('../../app/client/models/ObservatoryModel.ts').ObservatoryModel} */ (selection.model).runtimes = [ChartServiceTest.resource('runtime-only', [ChartServiceTest.projection('cpu', 99)])];
  service.draw(ChartServiceTest.view([canvas, context, missing]), selection);
  assert.equal(ChartBoundary.created.length, 2);
  assert.equal(ChartBoundary.created[0].data.datasets[0].data[0].y, 7);
  assert.equal(ChartBoundary.created[1].data.datasets[0].data[0].y, 19);
  assert.equal(canvas.dataset.points, '1');
  service.dispose();
});

test('chart drawing retains measured gaps, source window and unavailable message', /** @returns {void} */ () => {
  const {service, canvas} = ChartServiceTest.fixture();
  const projection = {...ChartServiceTest.projection('network', null), empty: true, emptyText: 'Network unavailable', measuredCount: 0};
  service.draw(ChartServiceTest.view([canvas]), ChartServiceTest.selection([ChartServiceTest.resource('session:root', [projection])], 'network'));
  const chart = ChartBoundary.created[0];
  assert.equal(chart.data.datasets[0].data[0].y, null);
  assert.equal(chart.data.datasets[0].spanGaps, false);
  assert.equal(chart.data.datasets[0].fill, false);
  assert.equal(/** @type {{x: {min: number, max: number}}} */ (chart.options.scales).x.min, 100);
  assert.equal(/** @type {{x: {min: number, max: number}}} */ (chart.options.scales).x.max, 220);
  assert.equal(canvas.card.empty.textContent, 'Network unavailable');
  assert.equal(canvas.card.empty.hidden, false);
  assert.equal(canvas.dataset.points, '0');
  service.dispose();
});

test('poll replacement reuses the owned canvas and updates its chart', /** @returns {void} */ () => {
  const {service, canvas, resource} = ChartServiceTest.fixture();
  service.draw(ChartServiceTest.view([canvas]), ChartServiceTest.selection([resource]));
  const replacement = new ChartCanvas('session:root');
  service.draw(ChartServiceTest.view([replacement]), ChartServiceTest.selection([ChartServiceTest.resource('session:root', [ChartServiceTest.projection('cpu', 13)])]));
  assert.equal(ChartBoundary.created.length, 1);
  assert.equal(replacement.card.canvas, canvas);
  assert.deepEqual(ChartBoundary.created[0].updates, ['none']);
  assert.equal(ChartBoundary.created[0].data.datasets[0].data[0].y, 13);
  service.dispose();
});

test('missing chart resources retire charts without assigning another history', /** @returns {void} */ () => {
  const {service, canvas, resource} = ChartServiceTest.fixture();
  service.draw(ChartServiceTest.view([canvas]), ChartServiceTest.selection([resource]));
  service.draw(ChartServiceTest.view([canvas]), /** @type {ViewSelection} */ ({group: resource, mode: 'cpu', model: {runtimes: [resource]}}));
  assert.equal(ChartBoundary.created[0].destroyed, 1);
  assert.equal(ChartBoundary.created.length, 1);
  service.dispose();
  assert.equal(ChartBoundary.created[0].destroyed, 1);
});

test('two service instances isolate matching identifiers and idempotent disposal', /** @returns {void} */ () => {
  ChartBoundary.created = [];
  const Service = ChartServiceTest.serviceClass();
  const first = new Service();
  const second = new Service();
  const resource = ChartServiceTest.resource('session:root', [ChartServiceTest.projection()]);
  first.draw(ChartServiceTest.view([new ChartCanvas('session:root')]), ChartServiceTest.selection([resource]));
  second.draw(ChartServiceTest.view([new ChartCanvas('session:root')]), ChartServiceTest.selection([resource]));
  first.dispose();
  first.dispose();
  assert.equal(ChartBoundary.created.length, 2);
  assert.equal(ChartBoundary.created[0].destroyed, 1);
  assert.equal(ChartBoundary.created[1].destroyed, 0);
  second.dispose();
  assert.equal(ChartBoundary.created[1].destroyed, 1);
});

test('a resource without a chart projection leaves the canvas unallocated', /** @returns {void} */ () => {
  const {service, canvas} = ChartServiceTest.fixture();
  service.draw(ChartServiceTest.view([canvas]), ChartServiceTest.selection([ChartServiceTest.resource('session:root', [])]));
  assert.equal(ChartBoundary.created.length, 0);
  service.dispose();
});
