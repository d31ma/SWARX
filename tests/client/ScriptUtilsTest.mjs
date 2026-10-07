import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';

/** @typedef {{dataset: {observatory?: string}, src?: string, defer?: boolean}} FixtureScript */
/** @typedef {{load: () => void, scripts: FixtureScript[], attributes: Map<string, string>, writes: [string, string][], created: string[]}} BrowserFixture */

class ScriptUtilsTest {
  /** @param {string | null} [stored] @param {string} [failure] @returns {BrowserFixture} */
  static browser(stored = null, failure = '') {
    /** @type {FixtureScript[]} */
    const scripts = [];
    /** @type {Map<string, string>} */
    const attributes = new Map();
    /** @type {[string, string][]} */
    const writes = [];
    /** @type {string[]} */
    const created = [];
    const context = vm.createContext({
      URL,
      document: {
        /** @param {string} selector @returns {FixtureScript | undefined} */
        querySelector(selector) { return scripts.find(/** @param {FixtureScript} script @returns {boolean} */ script => selector.includes('"' + script.dataset.observatory + '"')); },
        /** @param {string} tag @returns {FixtureScript} */
        createElement(tag) { created.push(tag); return { dataset: {} }; },
        head: {
          /** @param {FixtureScript} script @returns {void} */
          append(script) { scripts.push(script); }
        },
        documentElement: {
          /** @param {string} key @param {string} value @returns {void} */
          setAttribute(key, value) { attributes.set(key, value); }
        }
      },
      localStorage: {
        /** @returns {string | null} */
        getItem() { if (failure === 'read') throw new Error('storage blocked'); return stored; },
        /** @param {string} key @param {string} value @returns {void} */
        setItem(key, value) { if (failure === 'write') throw new Error('storage full'); writes.push([key, value]); }
      }
    });
    const url = new URL('../../app/client/shared/utils/ScriptUtils.js', import.meta.url);
    context.assetModuleUrl = url.href;
    const source = readFileSync(url, 'utf8').replace('export ', ' '.repeat('export '.length))
      .replaceAll('import.meta.url', 'assetModuleUrl'.padEnd('import.meta.url'.length));
    vm.runInContext(source + '\nglobalThis.Scripts = ScriptUtils;', context, { filename: url.href });
    return { load: /** @returns {void} */ () => (/** @type {{load: () => void}} */ (context.Scripts)).load(), scripts, attributes, writes, created };
  }

}

test('repeated load preserves the saved theme without manually injecting vendor scripts', /** @returns {void} */ () => {
  const browser = ScriptUtilsTest.browser('light');
  browser.load();
  browser.load();
  assert.equal(browser.attributes.get('w-theme'), 'light');
  assert.deepEqual(browser.created, []);
  assert.deepEqual(browser.scripts, []);
});

test('absent theme defaults to dark while an explicit empty theme is preserved', /** @returns {void} */ () => {
  const absent = ScriptUtilsTest.browser();
  absent.load();
  assert.equal(absent.attributes.get('w-theme'), 'dark');
  assert.deepEqual(absent.writes, [['w-theme', 'dark']]);
  const empty = ScriptUtilsTest.browser('');
  empty.load();
  assert.equal(empty.attributes.get('w-theme'), '');
});

test('unavailable theme storage read still applies the dark fallback', /** @returns {void} */ () => {
  const browser = ScriptUtilsTest.browser('light', 'read');
  browser.load();
  assert.equal(browser.attributes.get('w-theme'), 'dark');
  assert.deepEqual(browser.scripts, []);
  assert.deepEqual(browser.created, []);
  assert.deepEqual(browser.writes, []);
});

test('unavailable theme storage write still applies the dark fallback', /** @returns {void} */ () => {
  const browser = ScriptUtilsTest.browser('light', 'write');
  browser.load();
  assert.equal(browser.attributes.get('w-theme'), 'dark');
  assert.deepEqual(browser.scripts, []);
  assert.deepEqual(browser.created, []);
  assert.deepEqual(browser.writes, []);
});


test('an explicit automatic theme stays selected and stored', /** @returns {void} */ () => {
  const browser = ScriptUtilsTest.browser('auto');
  browser.load();
  assert.equal(browser.attributes.get('w-theme'), 'auto');
  assert.deepEqual(browser.writes, [['w-theme', 'auto']]);
});

test('fresh repeated load leaves the same dark theme without adding script nodes', /** @returns {void} */ () => {
  const browser = ScriptUtilsTest.browser();
  browser.load();
  const first = browser.attributes.get('w-theme');
  browser.load();
  assert.equal(first, 'dark');
  assert.equal(browser.attributes.get('w-theme'), first);
  assert.deepEqual(browser.created, []);
  assert.deepEqual(browser.scripts, []);
});
