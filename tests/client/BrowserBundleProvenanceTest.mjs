import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, readdirSync, rmSync, unlinkSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {BrowserBundleProvenanceUtils} from './utils/BrowserBundleProvenanceUtils.mjs';
import {ProvenanceVendorFixture} from './provenance/ProvenanceVendorFixture.mjs';

/** @typedef {import('node:test').TestContext} TestContext */
/** @typedef {{project: URL, emitted: URL, build: URL}} ProvenanceFixture */
/** @type {Record<string, string>} */
const SERVICES = {observatory: 'services/ObservatoryService.js', snapshot: 'services/native/monitoring/SnapshotService.js', presentation: 'services/browser/PresentationService.js', graph: 'services/browser/GraphService.js', chart: 'services/browser/ChartService.js', viewContinuity: 'services/browser/ViewContinuityService.js'};

/** @param {URL} root @param {string} relative @param {string} content @returns {void} */
function put(root, relative, content) {
  const file = new URL(relative, root);
  mkdirSync(new URL('./', file), {recursive: true});
  writeFileSync(file, content);
}

/** @param {URL} directory @param {string} prefix @returns {Record<string, string>} */
function hashes(directory, prefix) {
  /** @type {Record<string, string>} */
  const files = {};
  for (const entry of readdirSync(directory, {withFileTypes: true})) {
    const relative = prefix + entry.name;
    if (entry.isDirectory()) Object.assign(files, hashes(new URL(entry.name + '/', directory), relative + '/'));
    else files[relative] = createHash('sha256').update(readFileSync(new URL(entry.name, directory))).digest('hex');
  }
  return files;
}

/** @param {TestContext} context @returns {ProvenanceFixture} */
function fixture(context) {
  const root = mkdtempSync(join('/tmp', 'swarx-provenance-'));
  context.after(/** @returns {void} */ () => rmSync(root, {recursive: true, force: true}));
  const project = pathToFileURL(root + '/project/');
  const emitted = pathToFileURL(root + '/emitted/');
  const build = pathToFileURL(root + '/build.json');
  /** @type {Record<string, {module: string | null, native: boolean}>} */
  const services = {snapshotReader: {module: null, native: true}};
  put(project, 'app/client/services/SnapshotReaderService.rs', '#[Service]\nstruct SnapshotReaderService;\n');
  for (const [field, source] of Object.entries(SERVICES)) {
    const name = source.split('/').pop();
    services[field] = {module: '/.tachyon/services/' + name, native: false};
    put(project, 'app/client/' + source, '@Service\nexport default class ' + field + ' {}\n');
    put(emitted, 'tachyon-runtime/services/' + name, 'export default class ' + field + ' {}\n');
  }
  for (const component of ['panel', 'help']) {
    put(project, 'app/client/components/observatory/' + component + '/tac.html', '<section>' + component + '</section>');
    put(project, 'app/client/components/observatory/' + component + '/tac.css', ':host {color: blue;}');
  }
  put(project, 'app/client/shared/styles/app.css', 'body {margin: 0;}');
  put(project, 'app/client/shared/utils/ScriptUtils.js', 'export class ScriptUtils {}');
  put(project, 'app/client/models/Fixture.ts', 'export interface Fixture {value: string}');
  put(project, 'app/client/manifest.json', JSON.stringify(ProvenanceVendorFixture.manifest()));
  put(emitted, 'shared/utils/ScriptUtils.js', 'export class ScriptUtils {}');
  put(emitted, 'index.html', '<script id="tachyon-view" type="application/json">' + JSON.stringify({services}) + '</script>');
  put(emitted, 'tachyon-runtime/tac-client.js', 'export const runtime = "synthetic fixture";');
  ProvenanceVendorFixture.copy(project, emitted);
  writeFileSync(build, JSON.stringify({files: hashes(new URL('app/client/', project), 'app/client/')}));
  return {project, emitted, build};
}

test('provenance captures six actual catalog modules and scoped component inputs', context => {
  const data = fixture(context);
  const record = BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build);
  assert.equal(Object.keys(record).filter(name => name.startsWith('emitted:tachyon-runtime/services/')).length, 6);
  assert.ok(record['source:app/client/services/SnapshotReaderService.rs']);
  assert.ok(record['source:app/client/components/observatory/panel/tac.css']);
  assert.ok(record['source:app/client/components/observatory/help/tac.css']);
  assert.ok(record['source:app/client/models/Fixture.ts']);
  BrowserBundleProvenanceUtils.requireStable(record, data.project, data.emitted, data.build);
});

test('stale scoped CSS is refused even when Services are unchanged', context => {
  const data = fixture(context);
  put(data.project, 'app/client/components/observatory/panel/tac.css', ':host {color: red;}');
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /source is stale.*panel\/tac.css/);
});

test('missing component CSS is refused', context => {
  const data = fixture(context);
  unlinkSync(new URL('app/client/components/observatory/help/tac.css', data.project));
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /ENOENT/);
});

test('missing emitted provider is refused', context => {
  const data = fixture(context);
  unlinkSync(new URL('tachyon-runtime/services/ViewContinuityService.js', data.emitted));
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /ENOENT/);
});

test('restored legacy source is refused', context => {
  const data = fixture(context);
  put(data.project, 'app/client/shared/scripts/ViewContinuity.js', 'export class ViewContinuity {}');
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Retired browser source/);
});

test('retired asset in a compiler runtime alias is refused', context => {
  const data = fixture(context);
  put(data.emitted, 'tachyon-runtime/utils/PresentationUtils.js', 'export class PresentationUtils {}');
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Retired browser asset/);
});

test('missing catalog provider is refused', context => {
  const data = fixture(context);
  put(data.emitted, 'index.html', '<script id="tachyon-view">{"services":{}}</script>');
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /six browser Services and the native snapshotReader/);
});

test('external catalog module paths are refused', context => {
  const data = fixture(context);
  const html = readFileSync(new URL('index.html', data.emitted), 'utf8').replace('/.tachyon/services/GraphService.js', 'https://external.invalid/GraphService.js');
  put(data.emitted, 'index.html', html);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /local catalog module/);
});

test('source changes after capture are refused', context => {
  const data = fixture(context);
  const record = BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build);
  put(data.project, 'app/client/models/Fixture.ts', 'export interface Fixture {value: number}');
  assert.throws(() => BrowserBundleProvenanceUtils.requireStable(record, data.project, data.emitted, data.build), /changed during acceptance/);
});

test('stale live theme utility copy is refused', context => {
  const data = fixture(context);
  put(data.emitted, 'shared/utils/ScriptUtils.js', 'export class StaleLoader {}');
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /ScriptUtils.*stale/);
});

/** @param {ProvenanceFixture} data @returns {void} */
function capture(data) {
  writeFileSync(data.build, JSON.stringify({files: hashes(new URL('app/client/', data.project), 'app/client/')}));
}

/** @param {ProvenanceFixture} data @param {ReturnType<typeof ProvenanceVendorFixture.manifest>} manifest @returns {void} */
function declare(data, manifest) {
  put(data.project, 'app/client/manifest.json', JSON.stringify(manifest));
  capture(data);
}

/** @param {ProvenanceFixture} data @param {string} script @returns {void} */
function corruptSource(data, script) {
  const file = new URL('app/client' + script, data.project);
  writeFileSync(file, Buffer.concat([readFileSync(file), Buffer.from('\n')]));
}

/** @param {ProvenanceFixture} data @param {string} script @returns {void} */
function corruptPublished(data, script) {
  const file = new URL(script.slice(1), data.emitted);
  writeFileSync(file, Buffer.concat([readFileSync(file), Buffer.from('\n')]));
}

test('public modules resolves exactly the six browser providers to their local emitted modules', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const modules = BrowserBundleProvenanceUtils.modules(data.emitted);
  assert.deepEqual(Object.keys(modules).sort(), ['ChartService', 'GraphService', 'ObservatoryService', 'PresentationService', 'SnapshotService', 'ViewContinuityService']);
  assert.equal(modules.GraphService.href, new URL('tachyon-runtime/services/GraphService.js', data.emitted).href);
});

test('three real vendors are captured byte-for-byte and remain stable', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const record = BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build);
  assert.equal(record['emitted:shared/assets/duvay/duvay-wc.min.js'], 'bd252fed5bb8a90f4c4f9eb1ac8d948e8b5c9dcf88e35ccfe30915ba9d2960de');
  assert.equal(record['emitted:shared/assets/duvay/duvay.min.js'], '50ca0fd49f2232e3520f9e3d1fe8efa1fec1948a69e1076d43ed649a09b80d5b');
  assert.equal(record['emitted:shared/assets/chartjs/chart.umd.min.js'], '48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a');
  BrowserBundleProvenanceUtils.requireStable(record, data.project, data.emitted, data.build);
});

test('missing script declaration is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scripts.pop();
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Only the three pinned vendor scripts/);
});

test('an extra authored script cannot obtain vendor status', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scripts.push('/shared/utils/ScriptUtils.js');
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Only the three pinned vendor scripts/);
});

test('renamed vendor script declaration is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scripts[0] = '/shared/assets/duvay/renamed.js';
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Only the three pinned vendor scripts/);
});

test('vendor script order must remain exact', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scripts.reverse();
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /qualified order/);
});

test('all vendor metadata is required', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scriptSources = {};
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /exact package, version, license and source SHA-256/);
});

test('extra vendor metadata is refused even without an extra script', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scriptSources['/shared/assets/extra.js'] = {...manifest.tac.scriptSources[manifest.tac.scripts[0]]};
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /exact package, version, license and source SHA-256/);
});

test('renamed vendor metadata key is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  manifest.tac.scriptSources['/shared/assets/renamed.js'] = manifest.tac.scriptSources[manifest.tac.scripts[0]];
  delete manifest.tac.scriptSources[manifest.tac.scripts[0]];
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /exact package, version, license and source SHA-256/);
});

test('unsupported vendor loading metadata cannot bypass the exact schema', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const manifest = ProvenanceVendorFixture.manifest();
  Object.assign(manifest.tac.scriptSources[manifest.tac.scripts[0]], {kind: 'classic'});
  declare(data, manifest);
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /exact package, version, license and source SHA-256/);
});

for (const field of /** @type {('package' | 'version' | 'license' | 'sha256')[]} */ (['package', 'version', 'license', 'sha256'])) {
  test('changed vendor ' + field + ' is refused', /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    const manifest = ProvenanceVendorFixture.manifest();
    manifest.tac.scriptSources[manifest.tac.scripts[0]][field] = 'unqualified';
    declare(data, manifest);
    assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /exact package, version, license and source SHA-256/);
  });
}

for (const script of ProvenanceVendorFixture.manifest().tac.scripts) {
  test('changed captured source bytes are refused despite refreshed build capture: ' + script, /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    corruptSource(data, script);
    capture(data);
    assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Pinned vendor source bytes differ/);
  });
  test('changed published bytes are refused: ' + script, /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    corruptPublished(data, script);
    assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Published vendor bytes differ/);
  });
  test('missing vendor metadata entry is refused: ' + script, /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    const manifest = ProvenanceVendorFixture.manifest();
    delete manifest.tac.scriptSources[script];
    declare(data, manifest);
    assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /exact package, version, license and source SHA-256/);
  });
}

test('missing emitted real vendor is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  unlinkSync(new URL('shared/assets/chartjs/chart.umd.min.js', data.emitted));
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /ENOENT/);
});

test('vendor source drift after acceptance capture is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const record = BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build);
  corruptSource(data, '/shared/assets/chartjs/chart.umd.min.js');
  assert.throws(() => BrowserBundleProvenanceUtils.requireStable(record, data.project, data.emitted, data.build), /changed during acceptance/);
});

test('published vendor drift after acceptance capture is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const record = BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build);
  corruptPublished(data, '/shared/assets/chartjs/chart.umd.min.js');
  assert.throws(() => BrowserBundleProvenanceUtils.requireStable(record, data.project, data.emitted, data.build), /changed during acceptance/);
});

test('a browser module for the native snapshot reader is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const html = readFileSync(new URL('index.html', data.emitted), 'utf8').replace('"snapshotReader":{"module":null', '"snapshotReader":{"module":"/.tachyon/services/SnapshotReaderService.js"');
  put(data.emitted, 'index.html', html);
  assert.throws(() => BrowserBundleProvenanceUtils.modules(data.emitted), /no browser module/);
});

test('a snapshot reader that is not served by Rust is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  const html = readFileSync(new URL('index.html', data.emitted), 'utf8').replace('"module":null,"native":true', '"module":null,"native":false');
  put(data.emitted, 'index.html', html);
  assert.throws(() => BrowserBundleProvenanceUtils.modules(data.emitted), /served by Rust/);
});

test('missing Rust snapshot reader source is refused', /** @param {TestContext} context @returns {void} */ context => {
  const data = fixture(context);
  unlinkSync(new URL('app/client/services/SnapshotReaderService.rs', data.project));
  assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /ENOENT/);
});

for (const field of [...Object.keys(SERVICES), 'snapshotReader']) {
  test('public modules refuses missing catalog provider: ' + field, /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    const html = readFileSync(new URL('index.html', data.emitted), 'utf8');
    const matched = html.match(/<script id="tachyon-view"[^>]*>(.*?)<\/script>/s);
    assert.ok(matched);
    const catalog = JSON.parse(matched[1]);
    delete catalog.services[field];
    put(data.emitted, 'index.html', '<script id="tachyon-view">' + JSON.stringify(catalog) + '</script>');
    assert.throws(() => BrowserBundleProvenanceUtils.modules(data.emitted), /six browser Services and the native snapshotReader/);
  });
}

for (const retired of ['services/SnapshotReaderService.js', 'shared/scripts/ChartRenderer.js', 'shared/scripts/GraphRenderer.js', 'shared/scripts/GraphGroupRenderer.js', 'shared/scripts/ViewContinuity.js', 'utils/PresentationUtils.js', 'utils/DomUtils.js']) {
  test('each retired source remains refused: ' + retired, /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    put(data.project, 'app/client/' + retired, 'retired fixture');
    assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Retired browser source/);
  });
  test('each retired runtime alias remains refused: ' + retired, /** @param {TestContext} context @returns {void} */ context => {
    const data = fixture(context);
    put(data.emitted, 'tachyon-runtime/' + retired, 'retired fixture');
    assert.throws(() => BrowserBundleProvenanceUtils.inspect(data.project, data.emitted, data.build), /Retired browser asset/);
  });
}
