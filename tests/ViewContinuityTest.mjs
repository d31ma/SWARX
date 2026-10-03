import assert from 'node:assert/strict';
import {test} from 'node:test';
import {ViewContinuity} from '../app/client/shared/scripts/ViewContinuity.js';

/** @typedef {import('../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {{view: TestView, watcher: ViewContinuity}} ScrollFixture */

class ScrollElement {
  constructor() {
    /** @type {{[name: string]: string | undefined}} */
    this.dataset = {};
    /** @type {boolean} */
    this.isConnected = true;
    /** @type {number} */
    this.clientHeight = 100;
    /** @type {number} */
    this.scrollHeight = 300;
    /** @type {number} */
    this.clientWidth = 200;
    /** @type {number} */
    this.scrollWidth = 200;
    /** @type {number} */
    this.scrollTop = 0;
    /** @type {number} */
    this.scrollLeft = 0;
  }
  /** @param {string} selector @returns {boolean} */
  matches(selector) { return selector === '.session-list'; }
  /** @param {string} _selector @returns {ScrollElement | null} */
  closest(_selector) { return null; }
}

class CommandDetails extends ScrollElement {
  /** @param {string} binding @param {boolean} open */
  constructor(binding, open) {
    super();
    this.dataset.binding = binding;
    /** @type {boolean} */
    this.open = open;
  }
  /** @param {string} selector @returns {boolean} */
  matches(selector) { return selector === '.command-descendants'; }
}

class TestSummary extends ScrollElement {
  /** @param {CommandDetails} parent */
  constructor(parent) {
    super();
    /** @type {CommandDetails} */
    this.parentElement = parent;
  }
  /** @param {string} selector @returns {TestSummary | null} */
  closest(selector) { return selector === 'summary' ? this : null; }
}

class TestView {
  constructor() {
    /** @type {ScrollElement} */
    this.panel = new ScrollElement();
    /** @type {CommandDetails[]} */
    this.details = [];
    /** @type {Map<string, EventListener>} */
    this.listeners = new Map();
  }
  /** @param {string} name @param {EventListener} callback @returns {void} */
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  /** @param {string} name @returns {void} */
  removeEventListener(name) { this.listeners.delete(name); }
  /** @param {string} selector @returns {ScrollElement | null} */
  querySelector(selector) { return selector === '.session-list' ? this.panel : null; }
  /** @param {string} selector @returns {ScrollElement[]} */
  querySelectorAll(selector) {
    return this.details.filter(/** @param {CommandDetails} element @returns {boolean} */ element => element.matches(selector));
  }
}

class TestMutationObserver {
  /** @type {(() => void) | undefined} */
  static callback;
  /** @param {() => void} callback */
  constructor(callback) { TestMutationObserver.callback = callback; }
  /** @returns {void} */
  observe() {}
  /** @returns {void} */
  disconnect() {}
}

/** @returns {ScrollFixture} */
function fixture() {
  globalThis.HTMLElement = /** @type {typeof HTMLElement} */ (/** @type {unknown} */ (ScrollElement));
  globalThis.MutationObserver = /** @type {typeof MutationObserver} */ (/** @type {unknown} */ (TestMutationObserver));
  globalThis.ResizeObserver = /** @type {typeof ResizeObserver} */ (/** @type {unknown} */ (class { /** @returns {void} */ observe() {} /** @returns {void} */ disconnect() {} }));
  globalThis.document = /** @type {Document} */ (/** @type {unknown} */ ({body: {}, activeElement: {}}));
  globalThis.requestAnimationFrame = /** @returns {number} */ () => 1;
  globalThis.cancelAnimationFrame = /** @returns {void} */ () => {};
  const view = new TestView();
  const lifecycle = new AbortController();
  const watcher = new ViewContinuity(/** @type {HTMLElement} */ (/** @type {unknown} */ (view)), lifecycle.signal, /** @returns {ViewSelection} */ () => /** @type {ViewSelection} */ ({mode: 'cpu'}));
  watcher.start();
  return {view, watcher};
}

test('poll replacement retains scroll when animation frames are suspended', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  view.panel.scrollTop = 80;
  const scroll=view.listeners.get('scroll');assert.ok(scroll);
  scroll(/** @type {Event} */ (/** @type {unknown} */ ({target: view.panel})));
  view.panel = new ScrollElement();
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  /** @type {Promise<void>} */
  const restored = new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, 40); });
  await restored;
  assert.equal(view.panel.scrollTop, 80);
  watcher.dispose();
});

test('disposing a watcher cancels queued restoration', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  view.panel.scrollTop = 80;
  const scroll=view.listeners.get('scroll');assert.ok(scroll);
  scroll(/** @type {Event} */ (/** @type {unknown} */ ({target: view.panel})));
  view.panel = new ScrollElement();
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  watcher.dispose();
  /** @type {Promise<void>} */
  const restored = new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, 40); });
  await restored;
  assert.equal(view.panel.scrollTop, 0);
});

/** @param {TestView} view @param {CommandDetails} element @returns {void} */
function toggle(view, element) {
  const listener = view.listeners.get('toggle');
  assert.ok(listener);
  listener(/** @type {Event} */ (/** @type {unknown} */ ({target: element})));
}

/** @param {TestView} view @param {CommandDetails} element @returns {void} */
function clickSummary(view, element) {
  const listener = view.listeners.get('click');
  assert.ok(listener);
  listener(/** @type {Event} */ (/** @type {unknown} */ ({target: new TestSummary(element)})));
  element.open = !element.open;
  toggle(view, element);
}

/** @returns {Promise<void>} */
function restorePoll() {
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  return new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, 40); });
}

test('command descendants retain expansion by binding through reordered poll replacement', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const first = new CommandDetails('binding-one', true);
  view.details = [first, new CommandDetails('binding-two', false)];
  toggle(view, first);
  view.details = [new CommandDetails('binding-two', false), new CommandDetails('binding-one', false)];
  await restorePoll();
  assert.equal(view.details[1].open, true);
  assert.equal(view.details[0].open, false);
  view.details = [new CommandDetails('binding-one', false)];
  await restorePoll();
  assert.equal(view.details[0].open, true);
  watcher.dispose();
});

test('command descendants retain collapsed state without transferring expansion to a new binding', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = new CommandDetails('binding-one', true);
  toggle(view, previous);
  view.details = [new CommandDetails('binding-new', false)];
  await restorePoll();
  assert.equal(view.details[0].open, false);
  const collapsed = new CommandDetails('binding-one', true);
  view.details = [collapsed];
  clickSummary(view, collapsed);
  view.details = [new CommandDetails('binding-one', true)];
  await restorePoll();
  assert.equal(view.details[0].open, false);
  watcher.dispose();
});

test("replacement close toggle does not erase saved expansion before restoration", /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = new CommandDetails("binding-one", true);
  view.details = [previous];
  toggle(view, previous);
  const replacement = new CommandDetails("binding-one", false);
  view.details = [replacement];
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  toggle(view, replacement);
  await restorePoll();
  assert.equal(replacement.open, true);
  watcher.dispose();
});

test('restored details still retain a later deliberate collapse', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = new CommandDetails('binding-one', true);
  view.details = [previous];
  toggle(view, previous);
  const replacement = new CommandDetails('binding-one', false);
  view.details = [replacement];
  await restorePoll();
  assert.equal(replacement.open, true);
  clickSummary(view, replacement);
  view.details = [new CommandDetails('binding-one', true)];
  await restorePoll();
  assert.equal(view.details[0].open, false);
  watcher.dispose();
});

test('summary intent on a replacement before restoration overrides old expansion', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = new CommandDetails('binding-one', true);
  view.details = [previous];
  toggle(view, previous);
  const replacement = new CommandDetails('binding-one', true);
  view.details = [replacement];
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  clickSummary(view, replacement);
  await restorePoll();
  assert.equal(replacement.open, false);
  watcher.dispose();
});

test("prevented summary activation does not alter saved expansion", /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = new CommandDetails("binding-one", true);
  view.details = [previous];
  toggle(view, previous);
  const replacement = new CommandDetails("binding-one", false);
  view.details = [replacement];
  const listener = view.listeners.get("click");
  assert.ok(listener);
  listener(/** @type {Event} */ (/** @type {unknown} */ ({target: new TestSummary(replacement), defaultPrevented: true})));
  toggle(view, replacement);
  await restorePoll();
  assert.equal(replacement.open, true);
  watcher.dispose();
});

test("clicking details content is not disclosure intent", /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = new CommandDetails("binding-one", true);
  view.details = [previous];
  toggle(view, previous);
  const replacement = new CommandDetails("binding-one", false);
  view.details = [replacement];
  const listener = view.listeners.get("click");
  assert.ok(listener);
  listener(/** @type {Event} */ (/** @type {unknown} */ ({target: replacement, defaultPrevented: false})));
  toggle(view, replacement);
  await restorePoll();
  assert.equal(replacement.open, true);
  watcher.dispose();
});
