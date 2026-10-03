import assert from 'node:assert/strict';
import {readFileSync, readdirSync, existsSync} from 'node:fs';
import {createHash} from 'node:crypto';

/** @typedef {{module: string}} BrowserCatalogEntry */
/** @typedef {{services: Record<string, BrowserCatalogEntry>}} BrowserCatalog */

export class BrowserBundleProvenanceUtils {
  /** @type {Record<string, string>} */
  static #SERVICES = {observatory: 'services/ObservatoryService.js', snapshot: 'services/native/monitoring/SnapshotService.js', snapshotReader: 'services/SnapshotReaderService.js', presentation: 'services/browser/PresentationService.js', graph: 'services/browser/GraphService.js', chart: 'services/browser/ChartService.js', viewContinuity: 'services/browser/ViewContinuityService.js'};
  /** @type {string[]} */
  static #RETIRED = ['shared/scripts/ChartRenderer.js', 'shared/scripts/GraphRenderer.js', 'shared/scripts/GraphGroupRenderer.js', 'shared/scripts/ViewContinuity.js', 'utils/PresentationUtils.js', 'utils/DomUtils.js'];
  /** @type {string[]} */
  static #REQUIRED = ['components/observatory/panel/tac.html', 'components/observatory/panel/tac.css', 'components/observatory/help/tac.html', 'components/observatory/help/tac.css', 'shared/styles/app.css', 'utils/ScriptUtils.js', 'manifest.json'];

  /** @type {Record<string, {package: string, version: string, license: string, sha256: string}>} */
  static #VENDORS = {
    "/shared/assets/duvay/duvay-wc.min.js": {
      "package": "DuVay",
      "version": "26.35.01",
      "license": "MIT",
      "sha256": "bd252fed5bb8a90f4c4f9eb1ac8d948e8b5c9dcf88e35ccfe30915ba9d2960de"
    },
    "/shared/assets/duvay/duvay.min.js": {
      "package": "DuVay",
      "version": "26.35.01",
      "license": "MIT",
      "sha256": "50ca0fd49f2232e3520f9e3d1fe8efa1fec1948a69e1076d43ed649a09b80d5b"
    },
    "/shared/assets/chartjs/chart.umd.min.js": {
      "package": "Chart.js",
      "version": "4.5.1",
      "license": "MIT",
      "sha256": "48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a"
    }
  };

  /** @param {URL} project @param {URL} emitted @param {URL} build @returns {Record<string, string>} */
  static inspect(project, emitted, build) {
    this.#requireRetirement(project, emitted);
    this.#requireInputs(project);
    const sources = this.#sources(project, build);
    const modules = this.modules(emitted);
    const assets = this.#emittedHashes(modules, emitted);
    this.#requireLoader(project, emitted);
    const vendors = this.#vendorHashes(project, emitted);
    return {...sources, ...assets, ...vendors, 'build-manifest': this.#hash(build)};
  }

  /** @param {URL} emitted @returns {Record<string, URL>} */
  static modules(emitted) {
    const catalog = this.#catalog(emitted);
    return this.#modulePaths(catalog, emitted);
  }

  /** @param {Record<string, string>} record @param {URL} project @param {URL} emitted @param {URL} build @returns {void} */
  static requireStable(record, project, emitted, build) {
    this.#stableHashes(record, project, emitted, build);
    const fresh = this.inspect(project, emitted, build);
    assert.deepEqual(fresh, record, 'Source or emitted inventory changed during acceptance.');
  }

  /** @param {URL} emitted @returns {BrowserCatalog} */
  static #catalog(emitted) {
    const html = readFileSync(new URL('index.html', emitted), 'utf8');
    const matched = html.match(/<script id="tachyon-view"[^>]*>(.*?)<\/script>/s);
    assert.ok(matched, 'Actual emitted browser service catalog must be present.');
    /** @type {BrowserCatalog} */
    const catalog = JSON.parse(matched[1]);
    assert.deepEqual(Object.keys(catalog.services).sort(), Object.keys(this.#SERVICES).sort(), 'The actual catalog must declare all seven browser Services.');
    return catalog;
  }

  /** @param {BrowserCatalog} catalog @param {URL} emitted @returns {Record<string, URL>} */
  static #modulePaths(catalog, emitted) {
    /** @type {Record<string, URL>} */
    const modules = {};
    for (const [field, source] of Object.entries(this.#SERVICES)) {
      modules[source.slice(source.lastIndexOf('/') + 1, -3)] = this.#modulePath(catalog.services[field], source, emitted);
    }
    return modules;
  }

  /** @param {BrowserCatalogEntry} entry @param {string} source @param {URL} emitted @returns {URL} */
  static #modulePath(entry, source, emitted) {
    assert.equal(typeof entry.module, 'string', 'Each browser provider must have a local catalog module.');
    assert.ok(/^\/?\.tachyon\//.test(entry.module), 'Each browser provider must have a local catalog module.');
    const relative = entry.module.replace(/^\/?\.tachyon\//, 'tachyon-runtime/');
    const url = new URL(relative, emitted);
    assert.ok(url.href.startsWith(emitted.href), 'A local catalog module cannot escape the bundle.');
    assert.equal(url.pathname.slice(url.pathname.lastIndexOf('/') + 1), source.slice(source.lastIndexOf('/') + 1), 'Catalog module must identify the actual marked Service.');
    return url;
  }

  /** @param {URL} project @param {URL} emitted @returns {void} */
  static #requireRetirement(project, emitted) {
    const assets = this.#files(emitted);
    for (const relative of this.#RETIRED) {
      assert.equal(existsSync(new URL('app/client/' + relative, project)), false, 'Retired browser source is still active: ' + relative);
      assert.equal(assets.some(/** @param {string} asset @returns {boolean} */ asset => this.#retiredAsset(asset, relative)), false, 'Retired browser asset is still bundled: ' + relative);
    }
  }

  /** @param {string} asset @param {string} relative @returns {boolean} */
  static #retiredAsset(asset, relative) {
    return asset === relative || asset.endsWith('/' + relative);
  }

  /** @param {URL} project @returns {void} */
  static #requireInputs(project) {
    for (const relative of [...Object.values(this.#SERVICES), ...this.#REQUIRED]) {
      assert.ok(readFileSync(new URL('app/client/' + relative, project)).length > 0, 'Required browser input is nonempty: ' + relative);
    }
    /** @type {{tac: {scripts: string[], scriptSources: Record<string, {package: string, version: string, license: string, sha256: string}>}}} */
    const manifest = JSON.parse(readFileSync(new URL('app/client/manifest.json', project), 'utf8'));
    assert.deepEqual(manifest.tac.scripts, Object.keys(this.#VENDORS), 'Only the three pinned vendor scripts may be declared, in the qualified order.');
    assert.deepEqual(manifest.tac.scriptSources, this.#VENDORS, 'Each declared vendor must retain its exact package, version, license and source SHA-256.');
  }

  /** @param {URL} project @param {URL} build @returns {Record<string, string>} */
  static #sources(project, build) {
    /** @type {{files: Record<string, string>}} */
    const captured = JSON.parse(readFileSync(build, 'utf8'));
    /** @type {Record<string, string>} */
    const record = {};
    for (const file of this.#files(new URL('app/client/', project))) {
      const relative = 'app/client/' + file;
      const hash = this.#hash(new URL(relative, project));
      assert.equal(hash, captured.files[relative], 'Qualified native source is stale: ' + relative);
      record['source:' + relative] = hash;
    }
    return record;
  }

  /** @param {Record<string, URL>} modules @param {URL} emitted @returns {Record<string, string>} */
  static #emittedHashes(modules, emitted) {
    /** @type {Record<string, string>} */
    const record = {};
    for (const url of [...Object.values(modules), new URL('index.html', emitted), new URL('tachyon-runtime/tac-client.js', emitted), new URL('utils/ScriptUtils.js', emitted)]) {
      record['emitted:' + decodeURIComponent(url.href.slice(emitted.href.length))] = this.#hash(url);
    }
    return record;
  }

  /** @param {URL} project @param {URL} emitted @returns {Record<string, string>} */
  static #vendorHashes(project, emitted) {
    /** @type {Record<string, string>} */
    const record = {};
    for (const [relative, vendor] of Object.entries(this.#VENDORS)) {
      const source = this.#hash(new URL('app/client' + relative, project));
      assert.equal(source, vendor.sha256, 'Pinned vendor source bytes differ: ' + relative);
      const published = this.#hash(new URL(relative.slice(1), emitted));
      assert.equal(published, vendor.sha256, 'Published vendor bytes differ: ' + relative);
      record['emitted:' + relative.slice(1)] = published;
    }
    return record;
  }

  /** @param {URL} project @param {URL} emitted @returns {void} */
  static #requireLoader(project, emitted) {
    assert.ok(readFileSync(new URL('app/client/utils/ScriptUtils.js', project)).equals(readFileSync(new URL('utils/ScriptUtils.js', emitted))), 'Emitted ScriptUtils theme utility is stale.');
  }

  /** @param {Record<string, string>} record @param {URL} project @param {URL} emitted @param {URL} build @returns {void} */
  static #stableHashes(record, project, emitted, build) {
    for (const [name, expected] of Object.entries(record)) {
      assert.equal(this.#hash(this.#recordPath(name, project, emitted, build)), expected, 'Source changed during acceptance: ' + name);
    }
  }

  /** @param {string} name @param {URL} project @param {URL} emitted @param {URL} build @returns {URL} */
  static #recordPath(name, project, emitted, build) {
    if (name === 'build-manifest') return build;
    return this.#assetPath(name, project, emitted);
  }

  /** @param {string} name @param {URL} project @param {URL} emitted @returns {URL} */
  static #assetPath(name, project, emitted) {
    if (name.startsWith('source:')) return new URL(name.slice('source:'.length), project);
    assert.ok(name.startsWith('emitted:'), 'Unknown provenance record: ' + name);
    return new URL(name.slice('emitted:'.length), emitted);
  }

  /** @param {URL} file @returns {string} */
  static #hash(file) { return createHash('sha256').update(readFileSync(file)).digest('hex'); }

  /** @param {URL} directory @returns {string[]} */
  static #files(directory) {
    return readdirSync(directory, {withFileTypes: true}).flatMap(/** @param {import('node:fs').Dirent} entry @returns {string[]} */ entry => this.#entry(directory, entry));
  }

  /** @param {URL} directory @param {import('node:fs').Dirent} entry @returns {string[]} */
  static #entry(directory, entry) {
    if (entry.isDirectory()) return this.#files(new URL(entry.name + '/', directory)).map(/** @param {string} relative @returns {string} */ relative => entry.name + '/' + relative);
    assert.ok(entry.isFile(), 'Source and bundle inventory cannot follow symbolic links: ' + entry.name);
    return [entry.name];
  }
}
