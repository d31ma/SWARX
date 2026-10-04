import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';
import { BrowserServiceTestUtils } from './BrowserServiceTestUtils.mjs';

/** @typedef {import('../../app/client/models/NativeViewSelection.ts').NativeViewSelection} NativeSelection */
/** @typedef {Pick<import('../../app/client/models/ObservatoryModel.ts').ObservatoryModel, 'selected' | 'liveStatus' | 'detail' | 'selectedSession' | 'selectedSessionProcess' | 'resourceSessions' | 'sharedResources' | 'sessionView'>} TestResponse */
/** @typedef {{selection: NativeSelection, resolve: (value: TestResponse | PromiseLike<TestResponse>) => void, reject: (reason?: unknown) => void}} PendingRequest */
/** @typedef {{model?: TestResponse, series: string, selectedSession: string, sessionTab: string, sessionView: string, problem: string, status: string, snapshot: {read: (selection: NativeSelection) => Promise<TestResponse>}, taskScheduler: {render: () => Promise<void>}, lifecycle: AbortSignal, view: RequestView, presentation: import('../../app/client/services/browser/PresentationService.js').default, initialize: () => void, dispose: () => void, search: () => void, selectSessionProcess: (event: object, pid: number, started: string) => void, setSeries: (event: object, mode: string) => void, openSession: (event: object, key: string) => void, closeSession: (event: object) => void, toggleSession: (event: object, key: string) => void}} FixtureController *
/** @typedef {{controller: FixtureController, requests: PendingRequest[], poll: (() => void) | null}} RequestFrame */
/** @typedef {Omit<RequestFrame, 'controller'> & {controller: FixtureController | null}} PreparingFrame */

class OverviewRequestTest {
  /** @type {URL} */
  static SOURCE = new URL('../../app/client/services/ObservatoryService.js', import.meta.url);
  /** @type {URL} */
  static STAGED_SOURCE = new URL('../../client/services/ObservatoryService.js', import.meta.url);
  /** @type {URL} */
  static NATIVE_SELECTION = new URL('../../app/client/models/NativeViewSelection.rs', import.meta.url);

  // Tac hands native call arguments to Rust by exact field name (serde renames do not apply).
  /** @returns {string[]} */
  static nativeFields() {
    const source = readFileSync(OverviewRequestTest.NATIVE_SELECTION, 'utf8');
    return [...source.matchAll(/^\s*pub (\w+):/gm)].map(/** @param {RegExpMatchArray} match @returns {string} */ match => match[1]);
  }

  /** @returns {URL} */
  static source() {
    if (existsSync(OverviewRequestTest.STAGED_SOURCE)) return OverviewRequestTest.STAGED_SOURCE;
    return OverviewRequestTest.SOURCE;
  }

  /** @returns {RequestFrame} */
  static frame() {
    /** @type {PreparingFrame} */
    const frame = { controller: null, requests: [], poll: null };
    const context = vm.createContext({
      document: {head: {/** @returns {void} */ append() {}},
        /** @returns {null} */ querySelector() {return null;},
        /** @returns {{defer: boolean, dataset: Record<string,string>, src: string}} */ createElement() {return {defer: false, dataset: {}, src: ''};},
        documentElement: {/** @returns {void} */ setAttribute() {}}},
      localStorage: {/** @returns {null} */ getItem() {return null;}, /** @returns {void} */ setItem() {}},
      window: {}, HTMLElement: RequestView, MutationObserver: RequestObserver, ResizeObserver: RequestObserver,
      Map, Set, WeakRef, Date, Intl, URL, setTimeout, clearTimeout,
      /** @param {() => void} callback @returns {number} */
      setInterval(callback) { frame.poll = callback; return 1; },
      /** @returns {void} */
      clearInterval() {},
      Promise,
      Error
    });
    OverviewRequestTest.services(context);
    const source = readFileSync(OverviewRequestTest.source(), 'utf8')
      .replace(/^import .*;\n/gm, '')
      .replace(/^\s*@(?:Service|Needs|OnStart|OnStop)\s*$/gm, '')
      .replace('export default class ObservatoryService', 'class ObservatoryService');
    vm.runInContext(source + '\nglobalThis.Controller = ObservatoryService;', context);
    const controller = new (/** @type {new () => FixtureController} */ (context.Controller))();
    controller.snapshot = { /** @param {NativeSelection} selection @returns {Promise<TestResponse>} */ read(selection) { return new Promise(/** @param {(value: TestResponse | PromiseLike<TestResponse>) => void} resolve @param {(reason?: unknown) => void} reject @returns {number} */ (resolve, reject) => frame.requests.push({ selection, resolve, reject })); } };
    controller.taskScheduler = { /** @returns {Promise<void>} */ render() { return Promise.resolve(); } };
    controller.lifecycle = new AbortController().signal;
    controller.view = new RequestView();
    controller.presentation = OverviewRequestTest.presentation(context);
    frame.controller = controller;
    return /** @type {RequestFrame} */ (frame);
  }

  /** @param {vm.Context} context @returns {void} */
  static services(context) {
    BrowserServiceTestUtils.load(context, 'GraphService');
    BrowserServiceTestUtils.load(context, 'ChartService');
    BrowserServiceTestUtils.load(context, 'ViewContinuityService');
    BrowserServiceTestUtils.load(context, 'PresentationService');
    BrowserServiceTestUtils.load(context, 'ScriptUtils', 'utils');
  }

  /** @param {vm.Context} context @returns {import('../../app/client/services/browser/PresentationService.js').default} */
  static presentation(context) {
    const graph = new context.GraphService();
    const chart = new context.ChartService();
    const continuity = new context.ViewContinuityService();
    continuity.graph = graph;
    continuity.chart = chart;
    const presentation = new context.PresentationService();
    presentation.graph = graph;
    presentation.chart = chart;
    presentation.viewContinuity = continuity;
    return presentation;
  }

  /** @param {RequestFrame} frame @returns {TestResponse} */
  static acceptedModel(frame) {
    const model = frame.controller.model;
    assert.ok(model);
    return model;
  }

  /** @param {RequestFrame} frame @returns {string} */
  static failureText(frame) {
    const message = frame.controller.view.dataset.nativeFailure;
    assert.ok(message);
    return message;
  }

  /** @returns {Promise<void>} */
  static async flush() {
    /** @type {Promise<void>} */
    const pending = new Promise(/** @param {() => void} resolve @returns {NodeJS.Immediate} */ resolve => setImmediate(resolve));
    await pending;
  }

  /** @returns {Promise<RequestFrame>} */
  static async selectedFrame() {
    const frame = OverviewRequestTest.frame();
    frame.controller.initialize();
    await OverviewRequestTest.flush();
    frame.controller.openSession({}, 'Codex:actual-id');
    frame.controller.selectSessionProcess({}, 42, 'actual-birth');
    frame.requests[0].resolve(OverviewRequestTest.response('ignored'));
    await OverviewRequestTest.flush();
    frame.requests[1].resolve({...OverviewRequestTest.response('selected-runtime'), sessionView: 'detail',
      selectedSession: 'Codex:actual-id', selectedSessionProcess: OverviewRequestTest.selectedProcess()});
    await OverviewRequestTest.flush();
    return frame;
  }

  /** @returns {import('../../app/client/models/ProcessRow.ts').ProcessRow} */
  static selectedProcess() {
    return {pid: 42, parent: 1, started: 'actual-birth', name: 'boundary-process',
      cpu_percent: null, resident_bytes: null, received_per_second: null, sent_per_second: null,
      disk_read_per_second: null, disk_write_per_second: null, depth: 0, indentClass: '',
      memory: 'Unavailable', cpu: 'Unavailable', networkLabel: 'Unavailable', diskLabel: 'Unavailable', relationship: 'Reported root'};
  }

  /** @param {RequestFrame} frame @returns {Promise<void>} */
  static async assertRetainedSelection(frame) {
    assert.equal(frame.requests.length, 2);
    assert.ok(frame.poll);
    frame.poll();
    await OverviewRequestTest.flush();
    assert.equal(frame.requests.length, 3);
    const selection = frame.requests[2].selection;
    assert.deepEqual({...selection, expanded_sessions: [...selection.expanded_sessions]}, {
      selected: 'selected-runtime', query: '', pid: 0,
      selected_session: 'Codex:actual-id', session_pid: 42, session_started: 'actual-birth',
      expanded_sessions: []});
  }

  /** @param {string} selected @returns {TestResponse} */
  static response(selected) {
    return { selected, liveStatus: 'Live', detail: null, selectedSession: '', selectedSessionProcess: null, resourceSessions: [], sharedResources: [], sessionView: 'list' };
  }
}

class RequestView {
  /** @type {Record<string, string | undefined>} */
  dataset = {};
  /** @type {Map<string, EventListener>} */
  listeners = new Map();
  /** @param {string} type @param {EventListener} listener @returns {void} */
  addEventListener(type, listener) {this.listeners.set(type, listener);}
  /** @param {string} type @returns {void} */
  removeEventListener(type) {this.listeners.delete(type);}
  /** @param {string} group @param {number} pid @returns {void} */
  pick(group, pid) {
    const listener = this.listeners.get('click');
    assert.ok(listener);
    const target = new RequestProcessTarget(group, pid);
    listener(/** @type {Event} */ (/** @type {unknown} */ ({target, defaultPrevented: false})));
  }
  /** @param {string} name @returns {void} */
  tab(name) {
    const listener = this.listeners.get('click');
    assert.ok(listener);
    listener(/** @type {Event} */ (/** @type {unknown} */ ({target: new RequestTabTarget(name), defaultPrevented: false})));
  }
  /** @returns {Element[]} */
  querySelectorAll() {return [];}
  /** @returns {null} */
  querySelector() {return null;}
  /** @param {string} key @returns {void} */
  press(key) {
    const listener = this.listeners.get('keydown');
    assert.ok(listener);
    listener(/** @type {Event} */ (/** @type {unknown} */ ({key, target: new RequestProcessTarget('', 0), preventDefault() {}})));
  }
}

class RequestTabTarget extends RequestView {
  /** @param {string} name */
  constructor(name) {super(); this.dataset = {sessionTab: name};}
  /** @param {string} selector @returns {Element | null} */
  closest(selector) {
    if (selector === '[data-session-tab]') return /** @type {Element} */ (/** @type {unknown} */ (this));
    return null;
  }
  /** @returns {void} */
  scrollIntoView() {}
}

class RequestProcessTarget extends RequestView {
  /** @param {string} group @param {number} pid */
  constructor(group, pid) {super(); this.dataset = {group, pid: String(pid)};}
  /** @param {string} selector @returns {Element | null} */
  closest(selector) {
    if (selector === '.process-pick') return /** @type {Element} */ (/** @type {unknown} */ (this));
    return null;
  }
}

class RequestObserver {
  /** @returns {void} */
  observe() {}
  /** @returns {void} */
  disconnect() {}
}

test('initial controller has no fabricated native model', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  assert.equal(frame.controller.model, undefined);
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  assert.equal(frame.requests.length, 1);
  assert.equal(frame.controller.model, undefined);
  frame.controller.dispose();
  frame.requests[0].resolve(OverviewRequestTest.response('old'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.model, undefined);
});

test('the read selection carries exactly the Rust NativeViewSelection field names', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.openSession({}, 'Claude:session-one');
  await OverviewRequestTest.flush();
  const selection = frame.requests[1].selection;
  assert.deepEqual(Object.keys(selection).sort(), OverviewRequestTest.nativeFields().sort());
  assert.equal(selection.selected_session, 'Claude:session-one');
  assert.equal('selectedSession' in selection, false);
  frame.controller.dispose();
});

test('latest selection replaces queued requests and rejects an older response', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.openSession({}, 'Claude:first');
  frame.controller.openSession({}, 'Claude:latest');
  frame.controller.view.pick('selected-runtime', 42);
  assert.equal(frame.requests.length, 1);
  frame.requests[0].resolve(OverviewRequestTest.response('stale-runtime'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.model, undefined);
  assert.equal(frame.requests.length, 2);
  assert.equal(frame.requests[1].selection.selected_session, 'Claude:latest');
  assert.equal(frame.requests[1].selection.selected, 'selected-runtime');
  frame.requests[1].resolve(OverviewRequestTest.response('selected-runtime'));
  await OverviewRequestTest.flush();
  assert.equal(OverviewRequestTest.acceptedModel(frame).selected, 'selected-runtime');
  assert.equal(frame.controller.selectedSession, 'Claude:latest');
  assert.equal(frame.controller.status, 'Live');
  frame.controller.dispose();
});

test('periodic polling keeps one read and does not invalidate slow accepted data', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  assert.ok(frame.poll);
  frame.poll();
  assert.ok(frame.poll);
  frame.poll();
  assert.equal(frame.requests.length, 1);
  frame.requests[0].resolve(OverviewRequestTest.response('current'));
  await OverviewRequestTest.flush();
  assert.equal(OverviewRequestTest.acceptedModel(frame).selected, 'current');
  assert.equal(frame.requests.length, 2);
  frame.controller.dispose();
  frame.requests[1].resolve(OverviewRequestTest.response('late'));
  await OverviewRequestTest.flush();
  assert.equal(OverviewRequestTest.acceptedModel(frame).selected, 'current');
});

test('a stale bridge rejection does not overwrite the latest request state', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.openSession({}, 'Claude:new');
  frame.requests[0].reject(new Error('outdated failure'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.problem, '');
  assert.equal(frame.controller.view.dataset.nativeFailure, undefined);
  assert.equal(frame.requests.length, 2);
  frame.requests[1].resolve(OverviewRequestTest.response('new-group'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.status, 'Live');
  frame.controller.dispose();
});

test('current bridge failure remains readable and later polling recovers', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.requests[0].reject(new Error('test-only bridge unavailable'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.status, 'Disconnected');
  assert.match(frame.controller.problem, /Retrying automatically/);
  assert.match(OverviewRequestTest.failureText(frame), /test-only bridge unavailable/);
  assert.ok(frame.poll);
  frame.poll();
  await OverviewRequestTest.flush();
  frame.requests[1].resolve(OverviewRequestTest.response('recovered'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.status, 'Live');
  assert.equal(frame.controller.problem, '');
  assert.equal(frame.controller.view.dataset.nativeFailure, undefined);
  frame.controller.dispose();
});

test('session process selection sends exact identity within the open session', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.openSession({}, 'Codex:actual-id');
  frame.controller.selectSessionProcess({}, 42, 'actual-birth');
  frame.requests[0].resolve(OverviewRequestTest.response('ignored'));
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[1].selection.selected, '');
  assert.equal(frame.requests[1].selection.selected_session, 'Codex:actual-id');
  assert.equal(frame.requests[1].selection.session_pid, 42);
  assert.equal(frame.requests[1].selection.session_started, 'actual-birth');
  frame.controller.dispose();
});

test('an unavailable session process is cleared before the next poll', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.openSession({}, 'Codex:actual-id');
  frame.controller.selectSessionProcess({}, 42, 'retired-birth');
  frame.requests[0].resolve(OverviewRequestTest.response('old'));
  await OverviewRequestTest.flush();
  frame.requests[1].resolve({...OverviewRequestTest.response('runtime'), sessionView: 'detail', selectedSession: 'Codex:actual-id'});
  await OverviewRequestTest.flush();
  assert.ok(frame.poll);
  frame.poll();
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[2].selection.selected_session, 'Codex:actual-id');
  assert.equal(frame.requests[2].selection.session_pid, null);
  assert.equal(frame.requests[2].selection.session_started, null);
  frame.controller.dispose();
});

test('series changes preserve the verified selection through the next poll', /** @returns {Promise<void>} */ async () => {
  const frame = await OverviewRequestTest.selectedFrame();
  frame.controller.setSeries({}, 'network');
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.series, 'network');
  assert.equal(frame.controller.sessionTab, 'resources');
  assert.equal(OverviewRequestTest.acceptedModel(frame).selectedSessionProcess?.started, 'actual-birth');
  await OverviewRequestTest.assertRetainedSelection(frame);
  frame.controller.dispose();
});

test('Tab changes preserve the series and exact process identity through the next poll', /** @returns {Promise<void>} */ async () => {
  const frame = await OverviewRequestTest.selectedFrame();
  frame.controller.view.tab('tree');
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.sessionTab, 'tree');
  assert.equal(frame.controller.series, 'cpu');
  assert.equal(OverviewRequestTest.acceptedModel(frame).selectedSessionProcess?.pid, 42);
  await OverviewRequestTest.assertRetainedSelection(frame);
  frame.controller.dispose();
});

test('a bound process click selects runtime and PID without exposing an internal selector', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  assert.equal('select' in frame.controller, false);
  frame.controller.view.pick('runtime:actual-root', 42);
  frame.requests[0].resolve(OverviewRequestTest.response('previous'));
  await OverviewRequestTest.flush();
  assert.equal(frame.requests.length, 2);
  assert.equal(frame.requests[1].selection.selected, 'runtime:actual-root');
  assert.equal(frame.requests[1].selection.pid, 42);
  frame.requests[1].resolve(OverviewRequestTest.response('runtime:actual-root'));
  await OverviewRequestTest.flush();
  assert.equal(OverviewRequestTest.acceptedModel(frame).selected, 'runtime:actual-root');
  frame.controller.dispose();
});

/** @param {RequestFrame} frame @returns {Promise<void>} */
async function started(frame) {
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.requests[0].resolve(OverviewRequestTest.response('runtime'));
  await OverviewRequestTest.flush();
}

/** @param {string} view @returns {TestResponse} */
function sessionResponse(view) {
  return {...OverviewRequestTest.response('runtime'), sessionView: view};
}

test('openSession sends the exact session key and shows loading until that read arrives', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.openSession({}, 'Claude:session-one');
  assert.equal(frame.controller.selectedSession, 'Claude:session-one');
  assert.equal(frame.controller.sessionView, 'loading');
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[1].selection.selected_session, 'Claude:session-one');
  frame.requests[1].resolve(sessionResponse('detail'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.sessionView, 'detail');
  frame.controller.dispose();
});

test('the open session and tab survive polls, including honest inactive and missing states', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.openSession({}, 'Claude:session-a');
  frame.controller.view.tab('activity');
  await OverviewRequestTest.flush();
  frame.requests[1].resolve(sessionResponse('detail'));
  await OverviewRequestTest.flush();
  assert.ok(frame.poll);
  frame.poll();
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[2].selection.selected_session, 'Claude:session-a');
  frame.requests[2].resolve(sessionResponse('inactive'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.sessionView, 'inactive');
  frame.poll();
  await OverviewRequestTest.flush();
  frame.requests[3].resolve(sessionResponse('missing'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.selectedSession, 'Claude:session-a');
  assert.equal(frame.controller.sessionView, 'missing');
  assert.equal(frame.controller.sessionTab, 'activity');
  frame.poll();
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[4].selection.selected_session, 'Claude:session-a');
  frame.controller.dispose();
});

test('switching sessions keeps the current tab and requests the new session', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.openSession({}, 'Claude:session-a');
  frame.controller.view.tab('tree');
  frame.controller.openSession({}, 'Claude:session-b');
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[1].selection.selected_session, 'Claude:session-a');
  frame.requests[1].resolve(sessionResponse('detail'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.sessionView, 'loading');
  assert.equal(frame.controller.sessionTab, 'tree');
  assert.equal(frame.requests[2].selection.selected_session, 'Claude:session-b');
  frame.controller.dispose();
});

test('closeSession clears the selection and the next read omits it', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.openSession({}, 'Claude:session-a');
  await OverviewRequestTest.flush();
  frame.requests[1].resolve(sessionResponse('detail'));
  await OverviewRequestTest.flush();
  frame.controller.closeSession({});
  assert.equal(frame.controller.selectedSession, '');
  assert.equal(frame.controller.sessionView, 'list');
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[2].selection.selected_session, '');
  frame.controller.dispose();
});

test('Escape closes an open session and does nothing when none is open', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.view.press('Escape');
  await OverviewRequestTest.flush();
  assert.equal(frame.requests.length, 1);
  frame.controller.openSession({}, 'Claude:session-a');
  await OverviewRequestTest.flush();
  frame.requests[1].resolve(sessionResponse('detail'));
  await OverviewRequestTest.flush();
  frame.controller.view.press('Escape');
  assert.equal(frame.controller.selectedSession, '');
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[2].selection.selected_session, '');
  frame.controller.dispose();
});

test('session expansion is sent by exact key, survives polls and never opens the panel', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  await started(frame);
  frame.controller.toggleSession({}, 'Claude:session-one');
  assert.equal(frame.controller.selectedSession, '');
  assert.equal(frame.controller.sessionView, 'list');
  await OverviewRequestTest.flush();
  assert.deepEqual([...frame.requests[1].selection.expanded_sessions], ['Claude:session-one']);
  assert.equal(frame.requests[1].selection.selected_session, '');
  frame.requests[1].resolve(sessionResponse('list'));
  await OverviewRequestTest.flush();
  assert.ok(frame.poll);
  frame.poll();
  await OverviewRequestTest.flush();
  assert.deepEqual([...frame.requests[2].selection.expanded_sessions], ['Claude:session-one']);
  frame.requests[2].resolve(sessionResponse('list'));
  await OverviewRequestTest.flush();
  frame.controller.toggleSession({}, 'Claude:session-one');
  await OverviewRequestTest.flush();
  assert.deepEqual([...frame.requests[3].selection.expanded_sessions], []);
  assert.equal(frame.controller.selectedSession, '');
  frame.controller.dispose();
});
