import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import {BrowserBundleProvenanceUtils} from './utils/BrowserBundleProvenanceUtils.mjs';

/** @typedef {import('../../app/client/models/ObservatoryModel.ts').ObservatoryModel} ObservatoryModel */
/** @typedef {import('../../app/client/models/NativeViewSelection.ts').NativeViewSelection} NativeSelection */
/** @typedef {import('../../app/client/models/SnapshotTransfer.ts').SnapshotTransfer} SnapshotTransfer */
/** @typedef {import('../../app/client/models/SnapshotPage.ts').SnapshotPage} SnapshotPage */
/** @typedef {import('../../app/client/services/ObservatoryService.js').default} Observatory */
/** @typedef {import('../../app/client/services/native/monitoring/SnapshotService.js').default} Snapshot */
/** @typedef {{ObservatoryService: {default: typeof import('../../app/client/services/ObservatoryService.js').default}, SnapshotService: {default: typeof import('../../app/client/services/native/monitoring/SnapshotService.js').default}, SnapshotReaderService: {default: typeof import('../../app/client/services/SnapshotReaderService.js').default}, PresentationService: {default: typeof import('../../app/client/services/browser/PresentationService.js').default}, GraphService: {default: typeof import('../../app/client/services/browser/GraphService.js').default}, ChartService: {default: typeof import('../../app/client/services/browser/ChartService.js').default}, ViewContinuityService: {default: typeof import('../../app/client/services/browser/ViewContinuityService.js').default}}} EmittedModules */
/** @typedef {{default: new () => object}} BrowserModule */
/** @typedef {{open: number, page: number, release: number}} BoundaryCalls */
/** @typedef {{view?: HTMLElement, taskScheduler?: TacTaskScheduler, lifecycle?: AbortSignal}} RuntimeOwner */
/** @typedef {(name: string) => Promise<Observatory>} Resolver */
/** @typedef {{services: Record<string, unknown>}} ClientPlan */
/** @typedef {{ClientView: {plan: ClientPlan}, AbortController: typeof AbortController, location: {pathname: string}, Owners: {modules: WeakMap<object, BrowserModule>, managed: WeakMap<object, AbortController>, stored: WeakMap<object, unknown>}, Runtime: {graphs: WeakMap<object, object[]>}, importAsset: (path: string) => Promise<BrowserModule>, nativeHost: () => boolean, attachRuntime: (instance: RuntimeOwner) => void, resolve?: Resolver}} ResolverContext */

class BrowserInjectionTest {
  /** @type {URL} */
  static ROOT = QaPathUtils.directory('OBSERVATORY_WEB_BUNDLE');
  /** @type {URL} */
  static BUILD_SOURCE = QaPathUtils.file('OBSERVATORY_BUILD_SOURCE');
  /** @type {{observed: number, disconnected: number}} */
  static observerCalls = {observed: 0, disconnected: 0};

  /** @returns {Promise<void>} */
  static async run() {
    const sources = BrowserBundleProvenanceUtils.inspect(new URL('../../', import.meta.url), this.ROOT, this.BUILD_SOURCE);
    const restore = this.observers();
    try {
      await this.check();
      BrowserBundleProvenanceUtils.requireStable(sources, new URL('../../', import.meta.url), this.ROOT, this.BUILD_SOURCE);
    } finally {
      restore();
    }
  }

  /** @returns {Promise<void>} */
  static async check() {
    const modules = await this.modules();
    const calls = this.nativeBoundary(modules.SnapshotReaderService.default);
    const resolver = this.resolver(modules);
    const observatory = await resolver('observatory');
    try {
      assert.ok(observatory.snapshot instanceof modules.SnapshotService.default);
      assert.ok(observatory.snapshot.snapshotReader instanceof modules.SnapshotReaderService.default);
      const presentation = observatory.presentation;
      assert.ok(presentation instanceof modules.PresentationService.default);
      assert.ok(presentation.graph instanceof modules.GraphService.default);
      assert.ok(presentation.chart instanceof modules.ChartService.default);
      assert.ok(presentation.viewContinuity instanceof modules.ViewContinuityService.default);
      assert.equal(presentation.viewContinuity.graph, presentation.graph);
      assert.equal(presentation.viewContinuity.chart, presentation.chart);
      assert.ok(observatory.view);
      assert.ok(observatory.lifecycle);
      presentation.watch(observatory.view, observatory.lifecycle, this.viewSelection);
      assert.equal(this.observerCalls.observed, 2);
      await this.settled();
      assert.equal(observatory.problem, '', observatory.view.dataset.nativeFailure);
      assert.ok(observatory.model);
      assert.equal(observatory.model.ready, true);
      assert.equal(observatory.model.samples, 17);
      assert.deepEqual(calls, {open: 1, page: 2, release: 1});
      await this.concurrent(observatory.snapshot, calls);
    } finally {
      observatory.dispose();
      assert.equal(this.observerCalls.disconnected, 2);
    }
    console.log('PASS: seven actual emitted Services resolve through the compiler catalog, including shared transitive graph/chart identity. Public search, paging, empty-DOM render, watch and disposal pass with explicit synthetic native/observer boundaries. This is not actual WKWebView or startup acceptance.');
  }

  /** @returns {Promise<EmittedModules>} */
  static async modules() {
    const paths = BrowserBundleProvenanceUtils.modules(this.ROOT);
    return {
      ObservatoryService: await import(paths.ObservatoryService.href),
      SnapshotService: await import(paths.SnapshotService.href),
      SnapshotReaderService: await import(paths.SnapshotReaderService.href),
      PresentationService: await import(paths.PresentationService.href),
      GraphService: await import(paths.GraphService.href),
      ChartService: await import(paths.ChartService.href),
      ViewContinuityService: await import(paths.ViewContinuityService.href)
    };
  }

  /** @returns {() => void} */
  static observers() {
    const descriptors = Object.getOwnPropertyDescriptors(globalThis);
    /** @param {ResizeObserverCallback | MutationCallback} _callback @returns {ResizeObserver | MutationObserver} */
    function observer(_callback) {
      return /** @type {ResizeObserver | MutationObserver} */ (/** @type {unknown} */ ({
        /** @returns {void} */ observe() { BrowserInjectionTest.observerCalls.observed += 1; },
        /** @returns {void} */ disconnect() { BrowserInjectionTest.observerCalls.disconnected += 1; }
      }));
    }
    Object.defineProperty(globalThis, 'ResizeObserver', {value: observer, configurable: true});
    Object.defineProperty(globalThis, 'MutationObserver', {value: observer, configurable: true});
    return /** @returns {void} */ () => {
      for (const name of ['ResizeObserver', 'MutationObserver']) {
        if (descriptors[name]) Object.defineProperty(globalThis, name, descriptors[name]);
        else Reflect.deleteProperty(globalThis, name);
      }
    };
  }

  /** @returns {import('../../app/client/models/ViewSelection.ts').ViewSelection} */
  static viewSelection() {
    return {group: null, mode: 'cpu', model: undefined, pid: 0, session: '', sessionPid: null, sessionStarted: null,
      /** @returns {void} */ showTab() {}, /** @returns {void} */ close() {}, /** @returns {void} */ pick() {}};
  }

  /** @returns {HTMLElement} */
  static browserView() {
    return /** @type {HTMLElement} */ (/** @type {unknown} */ (Object.assign(new EventTarget(), {
      dataset: {}, /** @returns {Element[]} */ querySelectorAll() { return []; }
    })));
  }

  /** @returns {NativeSelection} */
  static selection() { return {selected: '', query: '', pid: 0, selectedSession: '', sessionPid: null, sessionStarted: null, selectedAgent: '', expandedSessions: []}; }

  /** @returns {ObservatoryModel} */
  static overview() {
    return {sessionGroups: [], resourceSessions: [], sharedResources: [], chartResources: [], selectedSession: '', selectedSessionProcess: null, activeSessions: [],
      activeSummary: {current: true, agents: 0, subAgents: 0, sessions: 0, tools: 0, harnesses: [], cards: []}, agentDetail: null, agentView: 'list', sessions: [], runtimes: [], shared: [], logical: [],
      activity: [], logicalCount: 0, logicalFeed: 'No linked task events', logicalNotice: '', detail: null,
      selected: '', errors: [], limitations: [], cpuTotal: 'Unavailable', resident: 'Unavailable', peak: 'Unavailable',
      downloadTotal: 'Unavailable', uploadTotal: 'Unavailable', completed: 0, reported: 0, elapsed: '0s', samples: 17,
      sampled: 'Synthetic test sample', sampleAge: 0, sessionCount: 0, sharedCount: 0, taskFeed: 'No linked task events',
      hasTaskFeed: false, ready: true, processLabel: 'Synthetic boundary fixture', liveStatus: 'Live'};
  }

  /** @param {typeof import('../../app/client/services/SnapshotReaderService.js').default} Reader @returns {BoundaryCalls} */
  static nativeBoundary(Reader) {
    const content = JSON.stringify(this.overview());
    const cut = Math.floor(content.length / 2);
    const parts = [content.slice(0, cut), content.slice(cut)];
    /** @type {BoundaryCalls} */
    const calls = {open: 0, page: 0, release: 0};
    Reader.prototype.open = /** @param {NativeSelection} selection @returns {Promise<SnapshotTransfer>} */ async selection => {
      assert.deepEqual(selection, BrowserInjectionTest.selection());
      calls.open += 1;
      return {id: 1, pages: parts.length, bytes: new TextEncoder().encode(content).length, error: ''};
    };
    Reader.prototype.page = /** @param {number} id @param {number} index @returns {Promise<SnapshotPage>} */ async (id, index) => {
      calls.page += 1;
      return {id, index, content: parts[index], error: ''};
    };
    Reader.prototype.release = /** @param {number} id @returns {Promise<boolean>} */ async id => { assert.equal(id, 1); calls.release += 1; return true; };
    return calls;
  }

  /** @param {EmittedModules} modules @returns {Resolver} */
  static resolver(modules) {
    const runtime = readFileSync(new URL('tachyon-runtime/tac-client.js', this.ROOT), 'utf8');
    const start = runtime.indexOf('const createOwner = async ');
    const end = runtime.indexOf('const composeOwner = ', start);
    assert.ok(start >= 0 && end > start, 'Actual compiler browser resolver must be present.');
    const html = readFileSync(new URL('index.html', this.ROOT), 'utf8');
    const matched = html.match(/<script id="tachyon-view"[^>]*>(.*?)<\/script>/s);
    assert.ok(matched);
    /** @type {ClientPlan} */
    const plan = JSON.parse(matched[1]);
    assert.deepEqual(Object.keys(plan.services).sort(), ['chart', 'graph', 'observatory', 'presentation', 'snapshot', 'snapshotReader', 'viewContinuity']);
    /** @type {ResolverContext} */
    const context = {
      ClientView: {plan}, AbortController, location: {pathname: '/'},
      Owners: {modules: new WeakMap(), managed: new WeakMap(), stored: new WeakMap()},
      Runtime: {graphs: new WeakMap()},
      importAsset: /** @param {string} path @returns {Promise<BrowserModule>} */ async path => {
        const file = path.split('/').pop();
        assert.ok(file);
        const name = file.replace('.js', '');
        const module = (/** @type {Record<string, BrowserModule>} */ (modules))[name];
        assert.ok(module);
        return module;
      },
      nativeHost: /** @returns {boolean} */ () => false,
      attachRuntime: /** @param {RuntimeOwner} instance @returns {void} */ instance => {
        instance.view = BrowserInjectionTest.browserView();
        instance.taskScheduler = /** @type {TacTaskScheduler} */ (/** @type {unknown} */ ({render: /** @returns {Promise<void>} */ async () => {}}));
        instance.lifecycle = new AbortController().signal;
      }
    };
    vm.runInNewContext(runtime.slice(start, end) + '\nglobalThis.resolve = serviceSingleton;', context);
    assert.ok(context.resolve);
    return context.resolve;
  }

  /** @returns {Promise<void>} */
  static settled() {
    return new Promise(/** @param {() => void} resolve @returns {void} */ resolve => { setImmediate(resolve); });
  }

  /** @param {Snapshot} snapshot @param {BoundaryCalls} calls @returns {Promise<void>} */
  static async concurrent(snapshot, calls) {
    const first = snapshot.read(this.selection());
    await assert.rejects(snapshot.read(this.selection()), /A local Overview read is already in progress\./);
    assert.deepEqual(await first, this.overview(), 'The first pinned transfer still returns the complete Overview.');
    assert.deepEqual(calls, {open: 2, page: 4, release: 2});
  }
}

await BrowserInjectionTest.run();
