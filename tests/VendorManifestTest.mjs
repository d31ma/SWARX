import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';

/** @typedef {{package: string, version: string, license: string, sha256: string}} VendorDeclaration */
/** @typedef {{start_url: string, tac: {application: {id: string, version: string}, scripts: string[], scriptSources?: Record<string, VendorDeclaration>}}} VendorManifest */

class VendorManifestTest {
  /** @type {Record<string, VendorDeclaration>} */
  static VENDORS = {
    '/shared/assets/duvay/duvay-wc.min.js': {
      package: 'DuVay', version: '26.35.01', license: 'MIT',
      sha256: 'bd252fed5bb8a90f4c4f9eb1ac8d948e8b5c9dcf88e35ccfe30915ba9d2960de'
    },
    '/shared/assets/duvay/duvay.min.js': {
      package: 'DuVay', version: '26.35.01', license: 'MIT',
      sha256: '50ca0fd49f2232e3520f9e3d1fe8efa1fec1948a69e1076d43ed649a09b80d5b'
    },
    '/shared/assets/chartjs/chart.umd.min.js': {
      package: 'Chart.js', version: '4.5.1', license: 'MIT',
      sha256: '48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a'
    }
  };

  /** @returns {VendorManifest} */
  static manifest() {
    return JSON.parse(readFileSync(new URL('../app/client/manifest.json', import.meta.url), 'utf8'));
  }

  /** @param {string} relative @returns {Buffer} */
  static asset(relative) {
    return readFileSync(new URL('../app/client' + relative, import.meta.url));
  }
}

test('manifest declares only the three captured local vendor scripts in their intended order', /** @returns {void} */ () => {
  const manifest = VendorManifestTest.manifest();
  assert.deepEqual(manifest.tac.scripts, Object.keys(VendorManifestTest.VENDORS));
});

test('each vendor has only the exact supported pinned package version license and digest fields', /** @returns {void} */ () => {
  const manifest = VendorManifestTest.manifest();
  assert.deepEqual(manifest.tac.scriptSources, VendorManifestTest.VENDORS);
  for (const declaration of Object.values(manifest.tac.scriptSources ?? {})) {
    assert.deepEqual(Object.keys(declaration).sort(), ['license', 'package', 'sha256', 'version']);
  }
});

test('declared vendor digests match the genuine captured source bytes', /** @returns {void} */ () => {
  const manifest = VendorManifestTest.manifest();
  assert.ok(manifest.tac.scriptSources);
  for (const [relative, expected] of Object.entries(VendorManifestTest.VENDORS)) {
    const digest = createHash('sha256').update(VendorManifestTest.asset(relative)).digest('hex');
    assert.equal(digest, expected.sha256, relative);
    assert.equal(manifest.tac.scriptSources[relative]?.sha256, digest, relative);
  }
});

test('manifest vendor metadata agrees with retained local version and MIT license evidence', /** @returns {void} */ () => {
  const manifest = VendorManifestTest.manifest();
  assert.ok(manifest.tac.scriptSources);
  const version = VendorManifestTest.asset('/shared/assets/duvay/VERSION').toString('utf8');
  assert.equal(version.split('\n')[0], 'DuVay 26.35.01');
  assert.equal(manifest.tac.scriptSources['/shared/assets/duvay/duvay-wc.min.js']?.version, '26.35.01');
  assert.equal(manifest.tac.scriptSources['/shared/assets/duvay/duvay.min.js']?.version, '26.35.01');
  const chart = VendorManifestTest.asset('/shared/assets/chartjs/chart.umd.min.js').toString('utf8');
  assert.match(chart.slice(0, 180), /Chart\.js v4\.5\.1/);
  assert.match(chart.slice(0, 180), /Released under the MIT License/);
  assert.equal(manifest.tac.scriptSources['/shared/assets/chartjs/chart.umd.min.js']?.version, '4.5.1');
  assert.match(VendorManifestTest.asset('/shared/assets/duvay/LICENSE').toString('utf8'), /^MIT License/);
  assert.match(VendorManifestTest.asset('/shared/assets/chartjs/LICENSE.md').toString('utf8'), /^The MIT License \(MIT\)/);
});

test('vendor declarations contain no authored utility or retired adapter exception', /** @returns {void} */ () => {
  const manifest = VendorManifestTest.manifest();
  assert.ok(manifest.tac.scriptSources);
  const urls = Object.keys(manifest.tac.scriptSources);
  assert.deepEqual(urls.sort(), Object.keys(VendorManifestTest.VENDORS).sort());
  assert.equal(urls.some(/** @param {string} url @returns {boolean} */ url => url.includes('/utils/') || url.includes('/scripts/')), false);
});

test('declarative vendor loading preserves internal application identity and entry route', /** @returns {void} */ () => {
  const manifest = VendorManifestTest.manifest();
  assert.equal(manifest.tac.application.id, 'ma.del.agent-observatory');
  assert.equal(manifest.tac.application.version, '0.1.0');
  assert.equal(manifest.start_url, '/');
});
