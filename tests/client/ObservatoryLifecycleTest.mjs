import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {test} from 'node:test';

/** @typedef {import('../../app/client/services/ObservatoryService.js').default} Observatory */
/** @typedef {{service: Observatory, owner: AbortController, finishRender: () => void, calls: {render: number, focus: number, dispose: number}}} LifecycleFixture */

class ObservatoryLifecycleTest {
  /** @returns {LifecycleFixture} */
  static fixture() {
    const sourceUrl = new URL('../../app/client/services/ObservatoryService.js', import.meta.url);
    const source = readFileSync(sourceUrl, 'utf8')
      .replace(/^import .*ScriptUtils.*;$/m, '')
      .replace(/^\s*@(?:Service|Needs|OnStart|OnStop)\s*$/gm, '')
      .replace('export default ', '');
    const context = vm.createContext({ScriptUtils: {load() {}}, setInterval: () => 1, clearInterval() {}, Promise});
    vm.runInContext(source + '\nglobalThis.ServiceClass = ObservatoryService;', context, {filename: sourceUrl.href});
    const service = /** @type {Observatory} */ (new context.ServiceClass());
    const owner = new AbortController();
    const calls = {render: 0, focus: 0, dispose: 0};
    /** @type {() => void} */
    let finishRender = () => {};
    const pending = new Promise(/** @param {(value?: void) => void} resolve @returns {void} */ resolve => { finishRender = resolve; });
    service.lifecycle = owner.signal;
    service.view = /** @type {HTMLElement} */ (/** @type {unknown} */ ({dataset: {}}));
    service.windowControls = /** @type {TacWindowControls} */ (/** @type {unknown} */ ({setTitle() {}}));
    service.taskScheduler = /** @type {TacTaskScheduler} */ (/** @type {unknown} */ ({render() { return pending; }}));
    service.snapshot = /** @type {Observatory['snapshot']} */ (/** @type {unknown} */ ({read() { return new Promise(() => {}); }}));
    service.presentation = /** @type {Observatory['presentation']} */ (/** @type {unknown} */ ({
      watch() {}, render() { calls.render += 1; }, focusSession() { calls.focus += 1; }, dispose() { calls.dispose += 1; }
    }));
    service.initialize();
    service.model = /** @type {NonNullable<Observatory['model']>} */ (/** @type {unknown} */ ({detail: null}));
    return {service, owner, finishRender, calls};
  }

  /** @returns {Promise<void>} */
  static async settled() {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  }
}

test('a delayed chart render cannot resume after page disposal', /** @returns {Promise<void>} */ async () => {
  const fixture = ObservatoryLifecycleTest.fixture();
  fixture.service.setSeries(new Event('click'), 'cpu');
  fixture.service.dispose();
  fixture.finishRender();
  await ObservatoryLifecycleTest.settled();
  assert.equal(fixture.calls.dispose, 1);
  assert.equal(fixture.calls.render, 0);
});

test('a delayed chart render cannot resume after its lifecycle aborts', /** @returns {Promise<void>} */ async () => {
  const fixture = ObservatoryLifecycleTest.fixture();
  fixture.service.setSeries(new Event('click'), 'cpu');
  fixture.owner.abort();
  fixture.finishRender();
  await ObservatoryLifecycleTest.settled();
  assert.equal(fixture.calls.render, 0);
  fixture.service.dispose();
});

test('a delayed session focus cannot resume after page disposal', /** @returns {Promise<void>} */ async () => {
  const fixture = ObservatoryLifecycleTest.fixture();
  fixture.service.openSession(new Event('click'), 'Claude:fixture-session');
  fixture.service.dispose();
  fixture.finishRender();
  await ObservatoryLifecycleTest.settled();
  assert.equal(fixture.calls.focus, 0);
});

test('an active page still completes delayed chart rendering and session focus', /** @returns {Promise<void>} */ async () => {
  const chart = ObservatoryLifecycleTest.fixture();
  chart.service.setSeries(new Event('click'), 'resident');
  chart.finishRender();
  await ObservatoryLifecycleTest.settled();
  assert.equal(chart.calls.render, 1);
  assert.equal(chart.service.series, 'resident');
  chart.service.dispose();
  const session = ObservatoryLifecycleTest.fixture();
  session.service.openSession(new Event('click'), 'Claude:fixture-session');
  session.finishRender();
  await ObservatoryLifecycleTest.settled();
  assert.equal(session.calls.focus, 1);
  assert.equal(session.service.selectedSession, 'Claude:fixture-session');
  session.service.dispose();
});
