import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

/** @typedef {import('../../app/client/services/browser/ViewContinuityService.js').default} ViewContinuityService */

/** @typedef {import('../../app/client/models/ViewSelection.ts').ViewSelection} ViewSelection */
/** @typedef {{view: TestView, watcher: ViewContinuityService, lifecycle: AbortController}} ScrollFixture */
/** @typedef {ScrollFixture & {first: GraphTab, last: GraphTab, selected: string[]}} KeyboardFixture */
/** @typedef {{target: ScrollElement, key: string, prevented: boolean, preventDefault: () => void}} KeyboardInput */

class ViewContinuityServiceTest {
  /** @returns {ViewContinuityService} */
  static service() {
    const context = vm.createContext({HTMLElement: globalThis.HTMLElement, MutationObserver: globalThis.MutationObserver,
      ResizeObserver: globalThis.ResizeObserver, document: globalThis.document, getComputedStyle: globalThis.getComputedStyle,
      CSS: {escape: /** @param {string} value @returns {string} */ value => value},
      window: {}, setTimeout, clearTimeout, Map, Set, WeakRef, Date, Intl});
    ViewContinuityServiceTest.load(context, 'GraphService');
    ViewContinuityServiceTest.load(context, 'ChartService');
    ViewContinuityServiceTest.load(context, 'ViewContinuityService');
    const service = new context.ViewContinuityService();
    service.graph = new context.GraphService();
    service.chart = new context.ChartService();
    return service;
  }
  /** @param {vm.Context} context @param {string} name @returns {void} */
  static load(context, name) {
    const url = new URL('../../app/client/services/browser/' + name + '.js', import.meta.url);
    const source = readFileSync(url, 'utf8')
      .replace(/^\s*@(?:Service|Needs|OnStop)\s*$/gm, '')
      .replace('export default ', '');
    vm.runInContext(source + '\nglobalThis.' + name + ' = ' + name + ';', context, {filename: url.href});
  }
  /** @returns {Promise<void>} */
  static delay() {
    return new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setTimeout(resolve, 40); });
  }
  /** @returns {KeyboardFixture} */
  static keyboardFixture() {
    const values = fixture();
    const first = new GraphTab('resources');
    const last = new GraphTab('activity');
    /** @type {string[]} */
    const selected = [];
    values.view.tabs = [first, last];
    values.watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (values.view)), values.lifecycle.signal,
      /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu',
        /** @param {Event} event @param {string} identifier @returns {void} */
        showTab(event, identifier) { selected.push(identifier); }}));
    return {...values, first, last, selected};
  }
  /** @param {KeyboardFixture} values @param {string} key @param {ScrollElement} target @returns {KeyboardInput} */
  static press(values, key, target) {
    const event = {key, target, prevented: false,
      /** @returns {void} */
      preventDefault() { this.prevented = true; }};
    const listener = values.view.listeners.get('keydown');
    assert.ok(listener);
    listener(/** @type {Event} */ (/** @type {unknown} */ (event)));
    return event;
  }
  /** @param {TestView} view @param {ScrollElement} target @returns {void} */
  static focus(view, target) {
    const listener = view.listeners.get('focusin');
    assert.ok(listener);
    listener(/** @type {Event} */ (/** @type {unknown} */ ({target})));
  }
  /** @returns {void} */
  static loseFocus() {
    const boundaryDocument = /** @type {{activeElement: object | null, body: object}} */ (/** @type {unknown} */ (globalThis.document));
    boundaryDocument.activeElement = boundaryDocument.body;
  }
}

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
  matches(selector) { return selector === '.logical-scroll'; }
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

class GraphTab extends ScrollElement {
  /** @param {string} identifier */
  constructor(identifier) {
    super();
    this.dataset.agentTab = identifier;
    /** @type {boolean} */
    this.focused = false;
    /** @type {FocusOptions | null} */
    this.focusOptions = null;
    /** @type {ScrollIntoViewOptions | null} */
    this.scrollOptions = null;
  }
  /** @param {string} selector @returns {GraphTab | null} */
  closest(selector) { return selector === '[data-agent-tab]' ? this : null; }
  /** @param {string} selector @returns {boolean} */
  matches(selector) { return selector === '[data-agent-tab]'; }
  /** @param {FocusOptions} options @returns {void} */
  focus(options) { this.focused = true; this.focusOptions = options; }
  /** @param {ScrollIntoViewOptions} options @returns {void} */
  scrollIntoView(options) { this.scrollOptions = options; }
}

class FocusControl extends ScrollElement {
  /** @param {string} selector @param {Record<string, string | undefined>} data */
  constructor(selector, data) {
    super();
    /** @type {string} */
    this.selector = selector;
    this.dataset = data;
    /** @type {boolean} */
    this.visible = true;
    /** @type {string} */
    this.visibility = 'visible';
    /** @type {boolean} */
    this.focused = false;
    /** @type {FocusOptions | null} */
    this.focusOptions = null;
  }
  /** @param {string} selector @returns {FocusControl | null} */
  closest(selector) { return this.matches(selector) ? this : null; }
  /** @param {string} selector @returns {boolean} */
  matches(selector) { return selector === this.selector; }
  /** @returns {DOMRect[]} */
  getClientRects() { return this.visible ? [/** @type {DOMRect} */ ({width: 120, height: 24})] : []; }
  /** @param {FocusOptions} options @returns {void} */
  focus(options) {
    this.focused = true;
    this.focusOptions = options;
    const boundaryDocument = /** @type {{activeElement: object | null}} */ (/** @type {unknown} */ (globalThis.document));
    boundaryDocument.activeElement = this;
  }
  /** @param {string} [view] @param {string} [session] @param {string} [started] @returns {FocusControl} */
  static process(view = 'tree', session = 'Codex:exact-session', started = 'exact-birth') {
    return new FocusControl('.session-process-pick', {session, pid: '11', started, processView: view});
  }
  /** @param {string} control @returns {FocusControl} */
  static agent(control) {
    return new FocusControl('[data-agent-focus]', {agentFocus: control});
  }
  /** @param {string} [group] @param {string} [action] @returns {FocusControl} */
  static zoom(group = 'Codex:exact-session', action = 'in') {
    return new FocusControl('[data-zoom]', {group, zoom: action});
  }
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
    /** @type {GraphTab[]} */
    this.tabs = [];
    /** @type {FocusControl[]} */
    this.controls = [];
    /** @type {Map<string, EventListener>} */
    this.listeners = new Map();
    /** @type {number} */
    this.added = 0;
  }
  /** @param {string} name @param {EventListener} callback @returns {void} */
  addEventListener(name, callback) { this.listeners.set(name, callback); this.added += 1; }
  /** @param {string} name @returns {void} */
  removeEventListener(name) { this.listeners.delete(name); }
  /** @param {string} selector @returns {ScrollElement | null} */
  querySelector(selector) { return selector === '.logical-scroll' ? this.panel : null; }
  /** @param {string} selector @returns {ScrollElement[]} */
  querySelectorAll(selector) {
    if (selector === '[data-agent-tab]') { return this.tabs; }
    if (['.session-process-pick', '[data-zoom]', '[data-agent-focus]', '.agent-row', '.agent-drawer'].includes(selector)) {
      return this.controls.filter(/** @param {FocusControl} element @returns {boolean} */ element => element.matches(selector));
    }
    return this.details.filter(/** @param {CommandDetails} element @returns {boolean} */ element => element.matches(selector));
  }
}

class TestMutationObserver {
  /** @type {(() => void) | undefined} */
  static callback;
  /** @type {TestMutationObserver[]} */
  static instances = [];
  /** @param {() => void} callback */
  constructor(callback) {
    TestMutationObserver.callback = callback;
    /** @type {() => void} */
    this.callback = callback;
    /** @type {number} */
    this.disconnected = 0;
    /** @type {number} */
    this.observed = 0;
    TestMutationObserver.instances.push(this);
  }
  /** @returns {void} */
  observe() { this.observed += 1; }
  /** @returns {void} */
  disconnect() { this.disconnected += 1; }
}

class TestResizeObserver {
  /** @type {TestResizeObserver[]} */
  static instances = [];
  /** @param {() => void} callback */
  constructor(callback) {
    /** @type {() => void} */
    this.callback = callback;
    /** @type {number} */
    this.observed = 0;
    /** @type {number} */
    this.disconnected = 0;
    TestResizeObserver.instances.push(this);
  }
  /** @returns {void} */
  observe() { this.observed += 1; }
  /** @returns {void} */
  disconnect() { this.disconnected += 1; }
}

/** @returns {ScrollFixture} */
function fixture() {
  TestMutationObserver.instances = [];
  TestResizeObserver.instances = [];
  globalThis.HTMLElement = /** @type {typeof HTMLElement} */ (/** @type {unknown} */ (ScrollElement));
  globalThis.MutationObserver = /** @type {typeof MutationObserver} */ (/** @type {unknown} */ (TestMutationObserver));
  globalThis.ResizeObserver = /** @type {typeof ResizeObserver} */ (/** @type {unknown} */ (TestResizeObserver));
  globalThis.document = /** @type {Document} */ (/** @type {unknown} */ ({body: {}, activeElement: {}}));
  globalThis.getComputedStyle = /** @param {Element} element @returns {CSSStyleDeclaration} */ element =>
    /** @type {CSSStyleDeclaration} */ ({visibility: /** @type {FocusControl} */ (/** @type {unknown} */ (element)).visibility});
  globalThis.requestAnimationFrame = /** @returns {number} */ () => 1;
  globalThis.cancelAnimationFrame = /** @returns {void} */ () => {};
  const view = new TestView();
  const lifecycle = new AbortController();
  const watcher = ViewContinuityServiceTest.service();
  watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (view)), lifecycle.signal, /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'}));
  return {view, watcher, lifecycle};
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

test('bind registers the live view and dispose removes every listener and observer', /** @returns {void} */ () => {
  const {view, watcher} = fixture();
  assert.equal(view.listeners.size, 5);
  assert.equal(TestMutationObserver.instances[0].observed, 1);
  assert.equal(TestResizeObserver.instances[0].observed, 1);
  watcher.dispose();
  watcher.dispose();
  assert.equal(view.listeners.size, 0);
  assert.equal(TestMutationObserver.instances[0].disconnected, 1);
  assert.equal(TestResizeObserver.instances[0].disconnected, 1);
});

test('Home selects and focuses the first agent tab without scrolling focus', /** @returns {void} */ () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  const event = ViewContinuityServiceTest.press(values, 'Home', values.last);
  assert.deepEqual(values.selected, ['resources']);
  assert.equal(values.first.focused, true);
  assert.equal(values.first.focusOptions?.preventScroll, true);
  assert.equal(values.first.scrollOptions?.block, 'nearest');
  assert.equal(values.first.scrollOptions?.inline, 'nearest');
  assert.equal(event.prevented, true);
  values.watcher.dispose();
});

test('End selects and focuses the last agent tab', /** @returns {void} */ () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  ViewContinuityServiceTest.press(values, 'End', values.first);
  assert.deepEqual(values.selected, ['activity']);
  assert.equal(values.last.focused, true);
  values.watcher.dispose();
});

test('ArrowLeft wraps the first agent tab to the last', /** @returns {void} */ () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  ViewContinuityServiceTest.press(values, 'ArrowLeft', values.first);
  assert.deepEqual(values.selected, ['activity']);
  assert.equal(values.last.focused, true);
  values.watcher.dispose();
});

test('ArrowRight wraps the last agent tab to the first', /** @returns {void} */ () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  ViewContinuityServiceTest.press(values, 'ArrowRight', values.last);
  assert.deepEqual(values.selected, ['resources']);
  assert.equal(values.first.focused, true);
  values.watcher.dispose();
});

test('an unrelated key retains browser keyboard behavior and tab selection', /** @returns {void} */ () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  const event = ViewContinuityServiceTest.press(values, 'Tab', values.first);
  assert.deepEqual(values.selected, []);
  assert.equal(values.first.focused, false);
  assert.equal(event.prevented, false);
  values.watcher.dispose();
});

test('navigation outside an agent tab retains selection', /** @returns {void} */ () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  const event = ViewContinuityServiceTest.press(values, 'Home', values.view.panel);
  assert.deepEqual(values.selected, []);
  assert.equal(event.prevented, false);
  values.watcher.dispose();
});

test('poll replacement restores a lost agent tab focus by its identifier', /** @returns {Promise<void>} */ async () => {
  const values = ViewContinuityServiceTest.keyboardFixture();
  const focus = values.view.listeners.get('focusin');
  assert.ok(focus);
  focus(/** @type {Event} */ (/** @type {unknown} */ ({target: values.last})));
  const replacement = new GraphTab('activity');
  values.view.tabs = [replacement, new GraphTab('tree')];
  const boundaryDocument = /** @type {{activeElement: object | null, body: object}} */ (/** @type {unknown} */ (globalThis.document));
  boundaryDocument.activeElement = boundaryDocument.body;
  await restorePoll();
  assert.equal(replacement.focused, true);
  assert.equal(replacement.focusOptions?.preventScroll, true);
  values.watcher.dispose();
});

test('focus continuity restores a replaced tree process by session PID birth and view', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process();
  const otherView = FocusControl.process('table');
  view.controls = [otherView, replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, true);
  assert.equal(replacement.focusOptions?.preventScroll, true);
  assert.equal(otherView.focused, false);
  watcher.dispose();
});

test('focus continuity restores a replaced table process without selecting its tree button', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process('table'));
  const replacement = FocusControl.process('table');
  const tree = FocusControl.process();
  view.controls = [tree, replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, true);
  assert.equal(tree.focused, false);
  watcher.dispose();
});

test('focus continuity rejects a reused PID with a different birth', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process('tree', 'Codex:exact-session', 'new-birth');
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity rejects the same PID and birth in another session', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process('tree', 'Claude:other-session');
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity rejects a process shown only in another view', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process('table');
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity ignores hidden old process views and restores the visible match', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const hidden = FocusControl.process();
  hidden.visible = false;
  const replacement = FocusControl.process();
  view.controls = [hidden, replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(hidden.focused, false);
  assert.equal(replacement.focused, true);
  watcher.dispose();
});

test('focus continuity rejects disconnected process elements', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process();
  replacement.isConnected = false;
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity rejects a layout-sized process hidden by CSS visibility', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process();
  replacement.visibility = 'hidden';
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity ignores a process without a verified birth identity', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const previous = FocusControl.process();
  previous.dataset.started = undefined;
  ViewContinuityServiceTest.focus(view, previous);
  const replacement = FocusControl.process();
  replacement.dataset.started = undefined;
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity clears remembered process focus when the query field is focused', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  ViewContinuityServiceTest.focus(view, new ScrollElement());
  const replacement = FocusControl.process();
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity preserves a currently focused query field', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const replacement = FocusControl.process();
  view.controls = [replacement];
  const boundaryDocument = /** @type {{activeElement: object | null}} */ (/** @type {unknown} */ (globalThis.document));
  const query = new ScrollElement();
  boundaryDocument.activeElement = query;
  await restorePoll();
  assert.equal(replacement.focused, false);
  assert.equal(boundaryDocument.activeElement, query);
  watcher.dispose();
});

test('focus continuity restores a replaced zoom control by group and action', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.zoom());
  const otherGroup = FocusControl.zoom('Claude:other-session');
  const otherAction = FocusControl.zoom('Codex:exact-session', 'out');
  const hidden = FocusControl.zoom();
  hidden.visible = false;
  const replacement = FocusControl.zoom();
  view.controls = [otherGroup, otherAction, hidden, replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, true);
  assert.equal(replacement.focusOptions?.preventScroll, true);
  assert.equal(otherGroup.focused, false);
  assert.equal(otherAction.focused, false);
  assert.equal(hidden.focused, false);
  watcher.dispose();
});

test('focus continuity clears remembered zoom focus when another control is focused', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.zoom());
  ViewContinuityServiceTest.focus(view, new ScrollElement());
  const replacement = FocusControl.zoom();
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity clears remembered controls across rebind', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.process());
  const next = new TestView();
  const replacement = FocusControl.process();
  next.controls = [replacement];
  watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (next)), new AbortController().signal,
    /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'}));
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('focus continuity cancels control restoration at stop and forgets it on later bind', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.zoom());
  const replacement = FocusControl.zoom();
  view.controls = [replacement];
  ViewContinuityServiceTest.loseFocus();
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  watcher.dispose();
  await ViewContinuityServiceTest.delay();
  assert.equal(replacement.focused, false);
  watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (view)), new AbortController().signal,
    /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'}));
  await restorePoll();
  assert.equal(replacement.focused, false);
  watcher.dispose();
});

test('abort cancels restoration and later stop is idempotent', /** @returns {Promise<void>} */ async () => {
  const {view, watcher, lifecycle} = fixture();
  view.panel.scrollTop = 72;
  const scroll = view.listeners.get('scroll');
  assert.ok(scroll);
  scroll(/** @type {Event} */ (/** @type {unknown} */ ({target: view.panel})));
  view.panel = new ScrollElement();
  assert.ok(TestMutationObserver.callback);
  TestMutationObserver.callback();
  lifecycle.abort();
  watcher.dispose();
  await ViewContinuityServiceTest.delay();
  assert.equal(view.panel.scrollTop, 0);
  assert.equal(view.listeners.size, 0);
  assert.equal(TestMutationObserver.instances[0].disconnected, 1);
});

test('rebinding clears old offsets and disclosure state and detaches the retired view', /** @returns {Promise<void>} */ async () => {
  const {view, watcher, lifecycle} = fixture();
  view.panel.scrollTop = 80;
  const scroll = view.listeners.get('scroll');
  assert.ok(scroll);
  scroll(/** @type {Event} */ (/** @type {unknown} */ ({target: view.panel})));
  const previous = new CommandDetails('binding-one', true);
  view.details = [previous];
  toggle(view, previous);
  const replacement = new TestView();
  replacement.details = [new CommandDetails('binding-one', false)];
  watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (replacement)), new AbortController().signal, /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'}));
  await restorePoll();
  assert.equal(view.listeners.size, 0);
  assert.equal(replacement.panel.scrollTop, 0);
  assert.equal(replacement.details[0].open, false);
  lifecycle.abort();
  assert.equal(replacement.listeners.size, 5);
  watcher.dispose();
});

test('retired observer callbacks cannot schedule restoration after disposal', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  const observer = TestMutationObserver.instances[0];
  const resize = TestResizeObserver.instances[0];
  watcher.dispose();
  observer.callback();
  resize.callback();
  await ViewContinuityServiceTest.delay();
  assert.equal(view.listeners.size, 0);
  assert.equal(view.panel.scrollTop, 0);
});

test('two continuity instances retain independent disclosure state for the same binding', /** @returns {Promise<void>} */ async () => {
  const first = fixture();
  const firstObserver = TestMutationObserver.instances[0];
  const second = fixture();
  const secondObserver = TestMutationObserver.instances[0];
  const firstDetails = new CommandDetails('same-binding', true);
  const secondDetails = new CommandDetails('same-binding', false);
  first.view.details = [firstDetails];
  second.view.details = [secondDetails];
  toggle(first.view, firstDetails);
  toggle(second.view, secondDetails);
  first.view.details = [new CommandDetails('same-binding', false)];
  second.view.details = [new CommandDetails('same-binding', true)];
  firstObserver.callback();
  secondObserver.callback();
  await ViewContinuityServiceTest.delay();
  assert.equal(first.view.details[0].open, true);
  assert.equal(second.view.details[0].open, false);
  first.watcher.dispose();
  second.watcher.dispose();
});

test('bind rejects a missing graph provider before attaching listeners', /** @returns {void} */ () => {
  const {view, watcher, lifecycle} = fixture();
  watcher.dispose();
  watcher.graph = undefined;
  assert.throws(/** @returns {void} */ () => watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (view)), lifecycle.signal,
    /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'})), /Required browser graph provider/);
  assert.equal(view.listeners.size, 0);
});

test('bind rejects a missing chart provider before attaching listeners', /** @returns {void} */ () => {
  const {view, watcher, lifecycle} = fixture();
  watcher.dispose();
  watcher.chart = undefined;
  assert.throws(/** @returns {void} */ () => watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (view)), lifecycle.signal,
    /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'})), /Required browser chart provider/);
  assert.equal(view.listeners.size, 0);
});

test('binding an already aborted signal does not observe or attach its view', /** @returns {void} */ () => {
  const {view, watcher} = fixture();
  watcher.dispose();
  const replacement = new TestView();
  const lifecycle = new AbortController();
  lifecycle.abort();
  watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (replacement)), lifecycle.signal, /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu'}));
  assert.equal(view.listeners.size, 0);
  assert.equal(replacement.listeners.size, 0);
  assert.equal(TestMutationObserver.instances[0].observed, 1);
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

test('Escape asks the selection to close the agent details', /** @returns {void} */ () => {
  const values = fixture();
  /** @type {string[]} */
  const closed = [];
  values.watcher.bind(/** @type {HTMLElement} */ (/** @type {unknown} */ (values.view)), values.lifecycle.signal,
    /** @returns {ViewSelection} */ () => ({group: null, mode: 'cpu',
      /** @param {Event} event @returns {void} */
      close(event) { closed.push(/** @type {KeyboardEvent} */ (event).key); }}));
  const listener = values.view.listeners.get('keydown');
  assert.ok(listener);
  listener(/** @type {Event} */ (/** @type {unknown} */ ({key: 'Enter', target: values.view.panel, preventDefault() {}})));
  listener(/** @type {Event} */ (/** @type {unknown} */ ({key: 'Escape', target: values.view.panel, preventDefault() {}})));
  assert.deepEqual(closed, ['Escape']);
  values.watcher.dispose();
});

test('focusAgent focuses only the element for the exact agent without scrolling', /** @returns {void} */ () => {
  const {view, watcher} = fixture();
  const other = new FocusControl('.agent-row', {agent: 'session:helper-1'});
  const target = new FocusControl('.agent-row', {agent: 'session:helper'});
  view.controls = [other, target];
  watcher.focusAgent('session:helper', '.agent-row');
  assert.equal(target.focused, true);
  assert.equal(target.focusOptions?.preventScroll, true);
  assert.equal(other.focused, false);
  watcher.focusAgent('missing-agent', '.agent-drawer');
  watcher.dispose();
});

test('focus continuity restores a replaced agent row by its focus key', /** @returns {Promise<void>} */ async () => {
  const {view, watcher} = fixture();
  ViewContinuityServiceTest.focus(view, FocusControl.agent('row:session:helper'));
  const replacement = FocusControl.agent('row:session:helper');
  const neighbour = FocusControl.agent('row:session');
  view.controls = [neighbour, replacement];
  ViewContinuityServiceTest.loseFocus();
  await restorePoll();
  assert.equal(replacement.focused, true);
  assert.equal(neighbour.focused, false);
  watcher.dispose();
});
