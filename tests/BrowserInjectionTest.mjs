import {QaPathUtils} from './QaPathUtils.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import vm from 'node:vm';

/** @typedef {import('../app/client/models/ObservatoryModel.ts').ObservatoryModel} ObservatoryModel */
/** @typedef {import('../app/client/models/NativeViewSelection.ts').NativeViewSelection} NativeSelection */
/** @typedef {import('../app/client/models/SnapshotTransfer.ts').SnapshotTransfer} SnapshotTransfer */
/** @typedef {import('../app/client/models/SnapshotPage.ts').SnapshotPage} SnapshotPage */
/** @typedef {import('../app/client/services/ObservatoryService.js').default} Observatory */
/** @typedef {import('../app/client/services/native/monitoring/SnapshotService.js').default} Snapshot */
/** @typedef {{ObservatoryService: {default: typeof import('../app/client/services/ObservatoryService.js').default}, SnapshotService: {default: typeof import('../app/client/services/native/monitoring/SnapshotService.js').default}, SnapshotReaderService: {default: typeof import('../app/client/services/SnapshotReaderService.js').default}}} EmittedModules */
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

  /** @returns {Promise<void>} */
  static async run() {
    this.requireCurrentSources();
    const modules = await this.modules();
    const calls = this.nativeBoundary(modules.SnapshotReaderService.default);
    const resolver = this.resolver(modules);
    const observatory = await resolver('observatory');
    try {
      assert.ok(observatory.snapshot instanceof modules.SnapshotService.default,
        'Observatory must receive the paging SnapshotService through the snapshot catalog entry.');
      assert.ok(observatory.snapshot.snapshotReader instanceof modules.SnapshotReaderService.default,
        'SnapshotService must receive the native reader through snapshotReader.');
      observatory.search();
      await this.settled();
      assert.ok(observatory.view);
      assert.equal(observatory.problem, '', observatory.view.dataset.nativeFailure);
      assert.ok(observatory.model);
      assert.equal(observatory.model.ready, true);
      assert.equal(observatory.model.samples, 17);
      assert.equal(calls.open, 1);
      assert.equal(calls.page, 2);
      assert.equal(calls.release, 1);
      await this.concurrent(observatory.snapshot, calls);
      console.log('PASS: accepted compiler-emitted Observatory -> Snapshot -> native Reader resolves through actual browser singleton injection; public search and paged Overview read use a synthetic native boundary; concurrent reads reject without disturbing the pinned transfer.');
    } finally {
      observatory.dispose();
    }
  }

  /** @returns {Promise<EmittedModules>} */
  static async modules() {
    return {
      ObservatoryService: await import(new URL('tachyon-runtime/services/ObservatoryService.js', this.ROOT).href),
      SnapshotService: await import(new URL('tachyon-runtime/services/SnapshotService.js', this.ROOT).href),
      SnapshotReaderService: await import(new URL('tachyon-runtime/services/SnapshotReaderService.js', this.ROOT).href)
    };
  }

  /** @returns {void} */
  static requireCurrentSources() {
    /** @type {{files: Record<string, string>}} */
    const build = JSON.parse(readFileSync(this.BUILD_SOURCE, 'utf8'));
    const project = new URL('../', import.meta.url);
    for (const file of ['services/ObservatoryService.js', 'services/SnapshotReaderService.js',
      'services/native/monitoring/SnapshotService.js']) {
      const relative = 'app/client/' + file;
      const hash = createHash('sha256').update(readFileSync(new URL(relative, project))).digest('hex');
      assert.equal(hash, build.files[relative], 'The accepted native build must match the current browser service source: ' + file);
    }
  }

  /** @returns {NativeSelection} */
  static selection() { return {selected: '', query: '', pid: 0}; }

  /** @returns {ObservatoryModel} */
  static overview() {
    return {sessionGroups: [], linkedSessionCount: 0, harnessCount: 0, eventSourceCount: 0,
      snapshotSourceCount: 0, recoveredSourceCount: 0, sessions: [], runtimes: [], shared: [], logical: [],
      activity: [], logicalCount: 0, logicalFeed: 'No linked task events', logicalNotice: '', detail: null,
      selected: '', errors: [], limitations: [], cpuTotal: 'Unavailable', resident: 'Unavailable', peak: 'Unavailable',
      downloadTotal: 'Unavailable', uploadTotal: 'Unavailable', completed: 0, reported: 0, elapsed: '0s', samples: 17,
      sampled: 'Synthetic test sample', sampleAge: 0, sessionCount: 0, sharedCount: 0, taskFeed: 'No linked task events',
      hasTaskFeed: false, ready: true, processLabel: 'Synthetic boundary fixture', liveStatus: 'Live'};
  }

  /** @param {typeof import('../app/client/services/SnapshotReaderService.js').default} Reader @returns {BoundaryCalls} */
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
    assert.deepEqual(Object.keys(plan.services).sort(), ['observatory', 'snapshot', 'snapshotReader']);
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
        instance.view = /** @type {HTMLElement} */ (/** @type {unknown} */ ({dataset: {}, /** @returns {Element[]} */ querySelectorAll() { return []; }}));
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
