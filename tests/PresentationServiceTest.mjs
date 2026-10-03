import assert from 'node:assert/strict';
import {test} from 'node:test';
import vm from 'node:vm';
import {BrowserServiceTestUtils} from './BrowserServiceTestUtils.mjs';

/** @typedef {import('../app/client/services/browser/PresentationService.js').default} PresentationService */
/** @typedef {import('../app/client/services/browser/GraphService.js').default} GraphService */
/** @typedef {import('../app/client/services/browser/ChartService.js').default} ChartService */
/** @typedef {import('../app/client/services/browser/ViewContinuityService.js').default} ViewContinuityService */
/** @typedef {import('../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {import('../app/client/models/ChartConfiguration.ts').ChartConfiguration} ChartConfiguration */
/** @typedef {{environment: PresentationEnvironment, presentation: PresentationService, view: PresentationView, lifecycle: AbortController, selection: ViewSelection}} PresentationFixture */

class PresentationElement {
  /** @param {string} selector @param {Record<string, string>} [dataset] */
  constructor(selector, dataset = {}) {
    /** @type {string} */
    this.selector = selector;
    /** @type {Record<string, string>} */
    this.dataset = dataset;
    /** @type {Record<string, string>} */
    this.style = {};
    /** @type {Map<string, PresentationElement>} */
    this.elements = new Map();
    /** @type {Map<string, PresentationElement[]>} */
    this.lists = new Map();
    /** @type {Record<string, string>} */
    this.attributes = {};
    /** @type {PresentationElement | null} */
    this.parentNode = null;
    /** @type {boolean} */
    this.isConnected = true;
    /** @type {boolean} */
    this.open = false;
    /** @type {boolean} */
    this.hidden = false;
    /** @type {boolean} */
    this.selected = false;
    /** @type {string} */
    this.textContent = '';
    /** @type {number} */
    this.clientWidth = 650;
    /** @type {number} */
    this.clientHeight = 240;
    /** @type {number} */
    this.scrollWidth = 650;
    /** @type {number} */
    this.scrollHeight = 800;
    /** @type {number} */
    this.scrollTop = 0;
    /** @type {number} */
    this.scrollLeft = 0;
    /** @type {number} */
    this.offsetHeight = 160;
    /** @type {{toggle: (name: string, value: boolean) => void}} */
    this.classList = {toggle: /** @param {string} _name @param {boolean} value @returns {void} */ (_name, value) => { this.selected = value; }};
  }
  /** @param {string} selector @returns {boolean} */
  matches(selector) { return selector === this.selector; }
  /** @param {string} selector @returns {PresentationElement | null} */
  closest(selector) { return this.matches(selector) ? this : this.parentNode?.closest(selector) ?? null; }
  /** @param {string} selector @returns {PresentationElement | null} */
  querySelector(selector) { return this.elements.get(selector) ?? null; }
  /** @param {string} selector @returns {PresentationElement[]} */
  querySelectorAll(selector) { return this.lists.get(selector) ?? []; }
  /** @param {string} name @param {string} value @returns {void} */
  setAttribute(name, value) { this.attributes[name] = value; }
}

class PresentationCard extends PresentationElement {
  constructor() {
    super('.history-card');
    /** @type {PresentationElement} */
    this.canvas = new PresentationElement('canvas.resource-chart', {group: 'session:root'});
    this.canvas.parentNode = this;
    this.elements.set('.chart-empty', new PresentationElement('.chart-empty'));
  }
  /** @param {PresentationElement} retained @param {PresentationElement} replacement @returns {void} */
  replaceChild(retained, replacement) {
    this.canvas = retained;
    retained.parentNode = this;
    retained.isConnected = true;
    replacement.parentNode = null;
    replacement.isConnected = false;
  }
}

class PresentationView extends PresentationElement {
  constructor() {
    super('view');
    /** @type {Map<string, Set<(event: Event) => void>>} */
    this.listeners = new Map();
    /** @type {PresentationElement} */
    this.panel = new PresentationElement('.session-list');
    /** @type {PresentationElement} */
    this.details = new PresentationElement('.command-descendants', {binding: 'session:root'});
    /** @type {PresentationCard} */
    this.card = new PresentationCard();
    /** @type {PresentationElement} */
    this.group = PresentationServiceTest.group();
  }
  /** @param {string} name @param {(event: Event) => void} listener @returns {void} */
  addEventListener(name, listener) {
    const listeners = this.listeners.get(name) ?? new Set();
    listeners.add(listener);
    this.listeners.set(name, listeners);
  }
  /** @param {string} name @param {(event: Event) => void} listener @returns {void} */
  removeEventListener(name, listener) { this.listeners.get(name)?.delete(listener); }
  /** @param {string} name @param {PresentationElement} target @returns {void} */
  send(name, target) {
    const event = /** @type {Event} */ (/** @type {unknown} */ ({target, defaultPrevented: false}));
    for (const listener of this.listeners.get(name) ?? []) listener(event);
  }
  /** @returns {number} */
  listenerCount() { return Array.from(this.listeners.values()).reduce(/** @param {number} count @param {Set<(event: Event) => void>} listeners @returns {number} */ (count, listeners) => count + listeners.size, 0); }
  /** @param {string} selector @returns {PresentationElement | null} */
  querySelector(selector) { return selector === '.session-list' ? this.panel : null; }
  /** @param {string} selector @returns {PresentationElement[]} */
  querySelectorAll(selector) {
    switch (selector) {
      case '.graph-group': return [this.group];
      case 'canvas.resource-chart': return [this.card.canvas];
      case '.command-descendants': return [this.details];
      case '.group-graph-scroll': return [/** @type {PresentationElement} */ (this.group.querySelector(selector))];
      default: return [];
    }
  }
  /** @returns {void} */
  replacePoll() {
    this.panel.isConnected = false;
    this.details.isConnected = false;
    this.panel = new PresentationElement('.session-list');
    this.details = new PresentationElement('.command-descendants', {binding: 'session:root'});
    this.card.canvas.isConnected = false;
    this.card = new PresentationCard();
    this.group = PresentationServiceTest.group();
  }
}

class PresentationObserver {
  /** @param {() => void} callback */
  constructor(callback) {
    /** @type {() => void} */
    this.callback = callback;
    /** @type {PresentationElement | null} */
    this.target = null;
  }
  /** @param {PresentationElement} target @returns {void} */
  observe(target) { this.target = target; }
  /** @returns {void} */
  disconnect() { this.target = null; }
  /** @returns {void} */
  deliver() { this.callback(); }
}

class PresentationChart {
  /** @param {PresentationElement} canvas @param {ChartConfiguration} configuration */
  constructor(canvas, configuration) {
    /** @type {PresentationElement} */
    this.canvas = canvas;
    /** @type {ChartConfiguration['data']} */
    this.data = configuration.data;
    /** @type {Record<string, unknown>} */
    this.options = configuration.options;
    /** @type {string[]} */
    this.updates = [];
    /** @type {number} */
    this.destroyed = 0;
  }
  /** @param {string} mode @returns {void} */
  update(mode) { this.updates.push(mode); }
  /** @returns {void} */
  destroy() { this.destroyed += 1; }
}

class PresentationEnvironment {
  constructor() {
    /** @type {PresentationObserver[]} */
    this.observers = [];
    /** @type {PresentationChart[]} */
    this.charts = [];
    /** @type {Map<number, () => void>} */
    this.timers = new Map();
    /** @type {number} */
    this.nextTimer = 0;
    const observers = this.observers;
    const charts = this.charts;
    const body = new PresentationElement('body');
    /** @type {vm.Context} */
    this.context = vm.createContext({HTMLElement: PresentationElement, document: {body, activeElement: body},
      MutationObserver: class extends PresentationObserver {
        /** @param {() => void} callback */
        constructor(callback) { super(callback); observers.push(this); }
      },
      ResizeObserver: class extends PresentationObserver {
        /** @param {() => void} callback */
        constructor(callback) { super(callback); observers.push(this); }
      },
      window: {Chart: class extends PresentationChart {
        /** @param {PresentationElement} canvas @param {ChartConfiguration} configuration */
        constructor(canvas, configuration) { super(canvas, configuration); charts.push(this); }
      }},
      CSS: {escape: /** @param {string} value @returns {string} */ value => value},
      getComputedStyle: /** @returns {{visibility: string}} */ () => ({visibility: 'visible'}),
      setTimeout: /** @param {() => void} callback @returns {number} */ callback => this.schedule(callback),
      clearTimeout: /** @param {number} identifier @returns {void} */ identifier => { this.timers.delete(identifier); }});
    BrowserServiceTestUtils.load(this.context, 'GraphService');
    BrowserServiceTestUtils.load(this.context, 'ChartService');
    BrowserServiceTestUtils.load(this.context, 'ViewContinuityService');
    BrowserServiceTestUtils.load(this.context, 'PresentationService');
  }
  /** @param {() => void} callback @returns {number} */
  schedule(callback) { this.nextTimer += 1; this.timers.set(this.nextTimer, callback); return this.nextTimer; }
  /** @returns {void} */
  flush() { const callbacks = Array.from(this.timers.values()); this.timers.clear(); for (const callback of callbacks) callback(); }
  /** @param {PresentationView} view @returns {void} */
  notify(view) { for (const observer of this.observers.filter(/** @param {PresentationObserver} observer @returns {boolean} */ observer => observer.target === view)) observer.deliver(); }
  /** @param {PresentationView} view @returns {number} */
  activeObservers(view) { return this.observers.filter(/** @param {PresentationObserver} observer @returns {boolean} */ observer => observer.target === view).length; }
  /** @returns {PresentationFixture} */
  fixture() {
    const graph = /** @type {GraphService} */ (new this.context.GraphService());
    const chart = /** @type {ChartService} */ (new this.context.ChartService());
    const continuity = /** @type {ViewContinuityService} */ (new this.context.ViewContinuityService());
    continuity.graph = graph;
    continuity.chart = chart;
    const presentation = /** @type {PresentationService} */ (new this.context.PresentationService());
    presentation.graph = graph;
    presentation.chart = chart;
    presentation.viewContinuity = continuity;
    return {environment: this, presentation, view: new PresentationView(), lifecycle: new AbortController(), selection: PresentationServiceTest.selection()};
  }
}

class PresentationServiceTest {
  /** @returns {PresentationElement} */
  static group() {
    const group = new PresentationElement('.graph-group', {group: 'session:root', session: 'session:root'});
    const node = new PresentationElement('.graph-node', {identity: '11:birth', pid: '11', started: 'birth', depth: '0'});
    group.lists.set('.graph-node', [node]);
    group.elements.set('.group-graph-scroll', new PresentationElement('.group-graph-scroll', {group: 'session:root'}));
    group.elements.set('.graph-canvas', new PresentationElement('.graph-canvas'));
    group.elements.set('.group-graph-size', new PresentationElement('.group-graph-size'));
    group.elements.set('.graph-edges', new PresentationElement('.graph-edges'));
    group.elements.set('.graph-zoom-value', new PresentationElement('.graph-zoom-value'));
    return group;
  }
  /** @param {number} [value] @returns {ViewSelection} */
  static selection(value = 7) {
    const charts = [{mode: 'cpu', unit: '%', windowStart: 100, windowEnd: 220, sampleCount: 2, measuredCount: 1,
      empty: false, emptyText: '', series: [{id: 'cpu', label: 'Measured CPU', measuredCount: 1, points: [{x: 120, y: value}]}]}];
    const model = /** @type {import('../app/client/models/ObservatoryModel.ts').ObservatoryModel} */ (/** @type {unknown} */ ({chartResources: [{id: 'session:root', charts}], selected: ''}));
    return {group: null, mode: 'cpu', session: 'session:root', sessionPid: 11, sessionStarted: 'birth', model};
  }
  /** @param {PresentationFixture} fixture @returns {void} */
  static watch(fixture) {
    fixture.presentation.watch(/** @type {HTMLElement} */ (/** @type {unknown} */ (fixture.view)), fixture.lifecycle.signal,
      /** @returns {ViewSelection} */ () => fixture.selection);
  }
  /** @param {PresentationFixture} fixture @returns {void} */
  static render(fixture) { fixture.presentation.render(/** @type {HTMLElement} */ (/** @type {unknown} */ (fixture.view)), fixture.selection); }
  /** @param {PresentationFixture} fixture @returns {void} */
  static start(fixture) { this.watch(fixture); this.render(fixture); }
  /** @param {PresentationFixture} fixture @returns {void} */
  static zoomOut(fixture) { fixture.view.send('click', new PresentationElement('[data-zoom]', {group: 'session:root', zoom: 'out'})); }
  /** @param {PresentationFixture} fixture @returns {string | undefined} */
  static zoomLabel(fixture) { return fixture.view.group.querySelector('.graph-zoom-value')?.textContent; }
}

test('composed watch and render place the selected graph node and allocate the projected chart', /** @returns {void} */ () => {
  const fixture = new PresentationEnvironment().fixture();
  PresentationServiceTest.start(fixture);
  assert.equal(fixture.view.group.querySelectorAll('.graph-node')[0].selected, true);
  assert.equal(PresentationServiceTest.zoomLabel(fixture), '100%');
  assert.equal(fixture.environment.charts[0].data.datasets[0].data[0].y, 7);
  assert.equal(fixture.view.card.canvas.dataset.points, '1');
  assert.equal(fixture.view.listenerCount(), 5);
  fixture.presentation.dispose();
});

test('watching twice leaves one live binding and coalesces observer deliveries into one chart update', /** @returns {void} */ () => {
  const fixture = new PresentationEnvironment().fixture();
  PresentationServiceTest.start(fixture);
  PresentationServiceTest.watch(fixture);
  fixture.environment.notify(fixture.view);
  assert.equal(fixture.view.listenerCount(), 5);
  assert.equal(fixture.environment.activeObservers(fixture.view), 2);
  assert.equal(fixture.environment.timers.size, 1);
  fixture.environment.flush();
  assert.deepEqual(fixture.environment.charts[0].updates, ['none']);
  assert.equal(fixture.environment.charts.length, 1);
  fixture.presentation.dispose();
});

test('poll restoration keeps graph zoom scroll disclosure and the owned canvas while using live projection data', /** @returns {void} */ () => {
  const fixture = new PresentationEnvironment().fixture();
  PresentationServiceTest.start(fixture);
  const canvas = fixture.view.card.canvas;
  PresentationServiceTest.zoomOut(fixture);
  fixture.view.panel.scrollTop = 120;
  fixture.view.send('scroll', fixture.view.panel);
  fixture.view.details.open = true;
  fixture.view.send('toggle', fixture.view.details);
  fixture.view.replacePoll();
  fixture.selection = PresentationServiceTest.selection(19);
  fixture.environment.notify(fixture.view);
  fixture.environment.flush();
  assert.equal(PresentationServiceTest.zoomLabel(fixture), '90%');
  assert.equal(fixture.view.panel.scrollTop, 120);
  assert.equal(fixture.view.details.open, true);
  assert.equal(fixture.view.card.canvas, canvas);
  assert.equal(fixture.environment.charts.length, 1);
  assert.equal(fixture.environment.charts[0].data.datasets[0].data[0].y, 19);
  fixture.presentation.dispose();
});

test('dispose cancels queued restoration and late observer delivery cannot recreate charts or listeners', /** @returns {void} */ () => {
  const fixture = new PresentationEnvironment().fixture();
  PresentationServiceTest.start(fixture);
  fixture.environment.notify(fixture.view);
  assert.equal(fixture.environment.timers.size, 1);
  fixture.presentation.dispose();
  fixture.presentation.dispose();
  fixture.environment.observers[0].deliver();
  fixture.environment.observers[1].deliver();
  fixture.view.send('click', new PresentationElement('[data-zoom]', {group: 'session:root', zoom: 'out'}));
  fixture.environment.flush();
  assert.equal(fixture.view.listenerCount(), 0);
  assert.equal(fixture.environment.activeObservers(fixture.view), 0);
  assert.equal(fixture.environment.timers.size, 0);
  assert.equal(fixture.environment.charts.length, 1);
  assert.equal(fixture.environment.charts[0].destroyed, 1);
  assert.deepEqual(fixture.environment.charts[0].updates, []);
  assert.equal(PresentationServiceTest.zoomLabel(fixture), '100%');
});

test('abort detaches continuity and the owning stop call releases its remaining chart once', /** @returns {void} */ () => {
  const fixture = new PresentationEnvironment().fixture();
  PresentationServiceTest.start(fixture);
  fixture.environment.notify(fixture.view);
  fixture.lifecycle.abort();
  assert.equal(fixture.view.listenerCount(), 0);
  assert.equal(fixture.environment.timers.size, 0);
  assert.equal(fixture.environment.charts[0].destroyed, 0);
  fixture.presentation.dispose();
  fixture.presentation.dispose();
  assert.equal(fixture.environment.charts[0].destroyed, 1);
  assert.equal(fixture.environment.activeObservers(fixture.view), 0);
});

test('disposing one composed owner preserves another owner with matching identifiers and forgets retired zoom', /** @returns {void} */ () => {
  const environment = new PresentationEnvironment();
  const first = environment.fixture();
  const second = environment.fixture();
  PresentationServiceTest.start(first);
  PresentationServiceTest.start(second);
  PresentationServiceTest.zoomOut(first);
  first.presentation.dispose();
  environment.notify(second.view);
  environment.flush();
  assert.equal(environment.charts[0].destroyed, 1);
  assert.equal(environment.charts[1].destroyed, 0);
  assert.deepEqual(environment.charts[1].updates, ['none']);
  assert.equal(second.view.listenerCount(), 5);
  assert.equal(PresentationServiceTest.zoomLabel(second), '100%');
  PresentationServiceTest.start(first);
  assert.equal(PresentationServiceTest.zoomLabel(first), '100%');
  assert.equal(environment.charts.length, 3);
  first.presentation.dispose();
  second.presentation.dispose();
});

test('missing continuity provider rejects watch before registering DOM or canvas resources', /** @returns {void} */ () => {
  const fixture = new PresentationEnvironment().fixture();
  fixture.presentation.viewContinuity = undefined;
  assert.throws(/** @returns {void} */ () => PresentationServiceTest.watch(fixture), /Required browser presentation provider is unavailable/);
  assert.equal(fixture.view.listenerCount(), 0);
  assert.equal(fixture.environment.activeObservers(fixture.view), 0);
  assert.equal(fixture.environment.charts.length, 0);
});
