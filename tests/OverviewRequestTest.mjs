import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';
import { BrowserServiceTestUtils } from './BrowserServiceTestUtils.mjs';

/** @typedef {import('../app/client/models/NativeViewSelection.ts').NativeViewSelection} NativeSelection */
/** @typedef {Pick<import('../app/client/models/ObservatoryModel.ts').ObservatoryModel, 'selected' | 'liveStatus' | 'detail' | 'selectedSession' | 'selectedSessionProcess'>} TestResponse */
/** @typedef {{selection: NativeSelection, resolve: (value: TestResponse | PromiseLike<TestResponse>) => void, reject: (reason?: unknown) => void}} PendingRequest */
/** @typedef {{model?: TestResponse, query: string, problem: string, status: string, snapshot: {read: (selection: NativeSelection) => Promise<TestResponse>}, taskScheduler: {render: () => Promise<void>}, lifecycle: AbortSignal, view: RequestView, presentation: import('../app/client/services/browser/PresentationService.js').default, initialize: () => void, dispose: () => void, search: () => void, select: (event: object, identifier: string) => void, selectSessionProcess: (event: object, session: string, pid: number, started: string) => void}} FixtureController */
/** @typedef {{controller: FixtureController, requests: PendingRequest[], poll: (() => void) | null}} RequestFrame */
/** @typedef {Omit<RequestFrame, 'controller'> & {controller: FixtureController | null}} PreparingFrame */

class OverviewRequestTest {
  /** @type {URL} */
  static SOURCE = new URL('../app/client/services/ObservatoryService.js', import.meta.url);
  /** @type {URL} */
  static STAGED_SOURCE = new URL('../client/services/ObservatoryService.js', import.meta.url);

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

  /** @param {vm.Context} context @returns {import('../app/client/services/browser/PresentationService.js').default} */
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

  /** @param {string} selected @returns {TestResponse} */
  static response(selected) {
    return { selected, liveStatus: 'Live', detail: null, selectedSession: '', selectedSessionProcess: null };
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
  /** @returns {Element[]} */
  querySelectorAll() {return [];}
  /** @returns {null} */
  querySelector() {return null;}
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

test('latest search replaces queued requests and rejects an older response', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.query = 'first';
  frame.controller.search();
  frame.controller.query = 'latest';
  frame.controller.select({}, 'selected-runtime');
  assert.equal(frame.requests.length, 1);
  frame.requests[0].resolve(OverviewRequestTest.response('stale-runtime'));
  await OverviewRequestTest.flush();
  assert.equal(frame.controller.model, undefined);
  assert.equal(frame.requests.length, 2);
  assert.equal(frame.requests[1].selection.query, 'latest');
  assert.equal(frame.requests[1].selection.selected, 'selected-runtime');
  frame.requests[1].resolve(OverviewRequestTest.response('selected-runtime'));
  await OverviewRequestTest.flush();
  assert.equal(OverviewRequestTest.acceptedModel(frame).selected, 'selected-runtime');
  assert.equal(frame.controller.query, 'latest');
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
  frame.controller.query = 'new-query';
  frame.controller.search();
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

test('session process selection sends exact identity without changing runtime selection', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.selectSessionProcess({}, 'Codex:actual-id', 42, 'actual-birth');
  frame.requests[0].resolve(OverviewRequestTest.response('ignored'));
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[1].selection.selected, '');
  assert.equal(frame.requests[1].selection.selectedSession, 'Codex:actual-id');
  assert.equal(frame.requests[1].selection.sessionPid, 42);
  assert.equal(frame.requests[1].selection.sessionStarted, 'actual-birth');
  frame.controller.dispose();
});

test('an unavailable session process is cleared before the next poll', /** @returns {Promise<void>} */ async () => {
  const frame = OverviewRequestTest.frame();
  frame.controller.initialize();
  await OverviewRequestTest.flush();
  frame.controller.selectSessionProcess({}, 'Codex:actual-id', 42, 'retired-birth');
  frame.requests[0].resolve(OverviewRequestTest.response('old'));
  await OverviewRequestTest.flush();
  frame.requests[1].resolve({...OverviewRequestTest.response('runtime'), selectedSession: 'Codex:actual-id'});
  await OverviewRequestTest.flush();
  assert.ok(frame.poll);
  frame.poll();
  await OverviewRequestTest.flush();
  assert.equal(frame.requests[2].selection.selectedSession, 'Codex:actual-id');
  assert.equal(frame.requests[2].selection.sessionPid, null);
  assert.equal(frame.requests[2].selection.sessionStarted, null);
  frame.controller.dispose();
});
