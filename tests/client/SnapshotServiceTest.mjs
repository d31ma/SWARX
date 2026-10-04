import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';

/** @typedef {Partial<import('../../app/client/models/NativeViewSelection.ts').NativeViewSelection>} FixtureSelection */
/** @typedef {import('../../app/client/models/SnapshotTransfer.ts').SnapshotTransfer} SnapshotTransfer */
/** @typedef {import('../../app/client/models/SnapshotPage.ts').SnapshotPage} SnapshotPage */
/** @typedef {Pick<import('../../app/client/models/ObservatoryModel.ts').ObservatoryModel, 'selected'>} FixtureResponse */
/** @typedef {{releases: number[], reads: number[], selection?: FixtureSelection, open: (selection: FixtureSelection) => Promise<SnapshotTransfer>, page: (id: number, index: number) => Promise<SnapshotPage>, release: (id: number) => Promise<boolean>}} ReaderFixture */
/** @typedef {{snapshotReader?: ReaderFixture, read: (selection: FixtureSelection) => Promise<FixtureResponse>, constructor: {_MAXIMUM_BYTES: number, _MAXIMUM_PAGES: number}}} SnapshotFixture */

class SnapshotServiceTest {
  /** @type {number} */
  static LONGEST_CHARACTER_BACKOFF = 3;

  /** @param {string} name @returns {number} */
  static rustLimit(name) {
    const url = new URL('../../app/client/services/native/monitoring/NativeSnapshotTransportService.rs', import.meta.url);
    const found = readFileSync(url, 'utf8').match(new RegExp('const ' + name + ': usize = ([0-9_]+);'));
    assert.ok(found, name + ' is declared in the Rust transport');
    return Number(found[1].replaceAll('_', ''));
  }

  /** @returns {SnapshotFixture} */
  static service() {
    const url = new URL('../../app/client/services/native/monitoring/SnapshotService.js', import.meta.url);
    const source = readFileSync(url, 'utf8')
      .replace(/^\s*@(?:Service|Needs)\s*$/gm, /** @param {string} match @returns {string} */ match => match.replace(/[^\r\n]/g, ' '))
      .replace('export default ', ' '.repeat('export default '.length));
    const context = vm.createContext({ Promise, Error, TextEncoder });
    vm.runInContext(source + '\nglobalThis.Snapshot = SnapshotService;', context, { filename: url.href });
    return new (/** @type {new () => SnapshotFixture} */ (context.Snapshot))();
  }

  /** @param {string} [content] @returns {ReaderFixture} */
  static reader(content = '{"selected":"actual-boundary-response"}') {
    return {
      releases: [], reads: [],
      /** @param {FixtureSelection} selection @returns {Promise<SnapshotTransfer>} */
      async open(selection) {
        this.selection = selection;
        return { id: 7, pages: 2, bytes: new TextEncoder().encode(content).length, error: '' };
      },
      /** @param {number} id @param {number} index @returns {Promise<SnapshotPage>} */
      async page(id, index) {
        this.reads.push(index);
        return { id, index, content: [content.slice(0, 10), content.slice(10)][index], error: '' };
      },
      /** @param {number} id @returns {Promise<boolean>} */
      async release(id) { this.releases.push(id); return true; }
    };
  }
}

test('browser transfer limits match the Rust page size and 2 MiB cap', /** @returns {void} */ () => {
  const limits = SnapshotServiceTest.service().constructor;
  const pageBytes = SnapshotServiceTest.rustLimit('PAGE_BYTES');
  const maximumBytes = SnapshotServiceTest.rustLimit('MAXIMUM_BYTES');
  assert.equal(limits._MAXIMUM_BYTES, maximumBytes);
  assert.equal(limits._MAXIMUM_PAGES, Math.ceil(maximumBytes / (pageBytes - SnapshotServiceTest.LONGEST_CHARACTER_BACKOFF)));
});

test('snapshot transfer uses and releases the provider that opened its ticket', /** @returns {Promise<void>} */ async () => {
  const service = SnapshotServiceTest.service();
  const reader = SnapshotServiceTest.reader();
  const replacement = SnapshotServiceTest.reader('{"selected":"wrong-provider"}');
  const open = reader.open.bind(reader);
  reader.open = /** @param {FixtureSelection} selection @returns {Promise<SnapshotTransfer>} */ async selection => {
    const ticket = await open(selection);
    service.snapshotReader = replacement;
    return ticket;
  };
  service.snapshotReader = reader;
  const selection = { query: 'boundary-test' };
  const model = await service.read(selection);
  assert.equal(model.selected, 'actual-boundary-response');
  assert.equal(reader.selection, selection);
  assert.deepEqual(reader.reads, [0, 1]);
  assert.deepEqual(reader.releases, [7]);
  assert.deepEqual(replacement.reads, []);
  assert.deepEqual(replacement.releases, []);
});

test('UTF-8 byte validation accepts Unicode and releases an invalid page', /** @returns {Promise<void>} */ async () => {
  const service = SnapshotServiceTest.service();
  const reader = SnapshotServiceTest.reader('{"selected":"é🐙"}');
  service.snapshotReader = reader;
  assert.equal((await service.read({})).selected, 'é🐙');
  reader.page = /** @param {number} id @param {number} index @returns {Promise<SnapshotPage>} */ async (id, index) => ({ id, index: index + 1, content: '', error: '' });
  await assert.rejects(service.read({}), /Invalid local snapshot page/);
  assert.deepEqual(reader.releases, [7, 7]);
});

test('decode and incomplete-transfer failures release the ticket and clear the busy state', /** @returns {Promise<void>} */ async () => {
  const service = SnapshotServiceTest.service();
  const reader = SnapshotServiceTest.reader('not json');
  service.snapshotReader = reader;
  await assert.rejects(service.read({}), /JSON/);
  assert.deepEqual(reader.releases, [7]);
  const open = reader.open.bind(reader);
  reader.open = /** @param {FixtureSelection} selection @returns {Promise<SnapshotTransfer>} */ async selection => ({ ...await open(selection), bytes: 99 });
  await assert.rejects(service.read({}), /Incomplete local snapshot/);
  assert.deepEqual(reader.releases, [7, 7]);
});

test('missing provider and invalid ticket reject without releasing an unopened transfer', /** @returns {Promise<void>} */ async () => {
  const service = SnapshotServiceTest.service();
  await assert.rejects(service.read({}), /Required native snapshot provider/);
  const reader = SnapshotServiceTest.reader();
  reader.open = /** @returns {Promise<SnapshotTransfer>} */ async () => ({ id: 7, pages: 87, bytes: 10, error: '' });
  service.snapshotReader = reader;
  await assert.rejects(service.read({}), /Invalid local snapshot transfer/);
  assert.deepEqual(reader.reads, []);
  assert.deepEqual(reader.releases, []);
});

test('concurrent reads reject and a release failure still clears the busy state', /** @returns {Promise<void>} */ async () => {
  const service = SnapshotServiceTest.service();
  const reader = SnapshotServiceTest.reader();
  const open = reader.open.bind(reader);
  /** @type {(() => void) | undefined} */
  let finish;
  reader.open = /** @param {FixtureSelection} selection @returns {Promise<SnapshotTransfer>} */ selection => new Promise(/** @param {(value: SnapshotTransfer | PromiseLike<SnapshotTransfer>) => void} resolve @returns {void} */ resolve => { finish = /** @returns {void} */ () => resolve(open(selection)); });
  service.snapshotReader = reader;
  const pending = service.read({});
  await assert.rejects(service.read({}), /already in progress/);
  assert.ok(finish);
  finish();
  await pending;
  reader.open = open;
  reader.release = /** @returns {Promise<boolean>} */ async () => { throw new Error('boundary release failure'); };
  await assert.rejects(service.read({}), /boundary release failure/);
  reader.release = /** @returns {Promise<boolean>} */ async () => true;
  assert.equal((await service.read({})).selected, 'actual-boundary-response');
});
