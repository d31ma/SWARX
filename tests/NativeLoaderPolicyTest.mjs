import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {test} from 'node:test';
import {BundleInspectionTest} from './BundleInspectionTest.mjs';

/** @typedef {{libraries?: string[], emitterLibraries?: string[], rpaths?: string[], rawLibraries?: string, rawLoads?: string, loadStatus?: number, localLibrary?: boolean, escapedLibrary?: boolean, webAsset?: string, webAssetSymlink?: boolean, webDirectoryAlias?: boolean, webAliasOutside?: boolean, webAliasCycle?: boolean}} LoaderFixture */
/** @typedef {{name: string, passed: boolean, detail: string, evidence?: unknown}} LoaderCheck */

class NativeLoaderPolicyTest {
  /** @param {import('node:test').TestContext} context @param {LoaderFixture} options @returns {{result: number, report: {conditions_passed: boolean, checks: LoaderCheck[]}, bundle: string}} */
  static inspect(context, options = {}) {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'swarx-loader-fixture-'));
    context.after(/** @returns {void} */ () => fs.rmSync(root, {recursive:true, force:true}));
    const bundle = path.join(root, 'Synthetic.app');
    this.files(root, bundle, options);
    const reportFile = path.join(root, 'report.json');
    const inspector = new BundleInspectionTest(bundle, reportFile, 'synthetic-loader-fixture',
      /** @param {string} tool @param {string[]} arguments_ @returns {{status: number, stdout: string, stderr: string, error: null}} */
      (tool, arguments_) => this.tool(tool, arguments_, options));
    const result = inspector.run();
    return {result, report:JSON.parse(fs.readFileSync(reportFile, 'utf8')), bundle};
  }

  /** @param {string} root @param {string} bundle @param {LoaderFixture} options @returns {void} */
  static files(root, bundle, options) {
    const assets = {
      'Contents/Info.plist':'synthetic plist supplied by the injected fixture tool',
      'Contents/MacOS/Main':'synthetic Mach-O metadata fixture; never executed',
      'Contents/MacOS/observatory-event':'synthetic emitter metadata fixture; never executed',
      'Contents/Resources/Icon.icns':'icns-synthetic-fixture',
      'Contents/Resources/NativeIndex.json':JSON.stringify({entry_document:'index.html'}),
      'Contents/Resources/WebBundle/index.html':'<!doctype html><script src="entry.js"></script>',
      'Contents/Resources/WebBundle/entry.js':'export const fixture = true;',
      'Contents/Resources/WebBundle/shared/assets/observatory.svg':'<svg></svg>',
      'Contents/Resources/WebBundle/shared/assets/duvay/duvay-wc.min.js':'/* synthetic vendor fixture */',
      'Contents/Resources/WebBundle/shared/assets/duvay/duvay.min.js':'/* synthetic vendor fixture */',
      'Contents/Resources/WebBundle/shared/assets/duvay/duvay.min.css':'/* synthetic vendor fixture */',
      'Contents/Resources/WebBundle/shared/assets/chartjs/chart.umd.min.js':'/* synthetic vendor fixture */'
    };
    for (const [relative, content] of Object.entries(assets)) {
      const file = path.join(bundle, relative);
      fs.mkdirSync(path.dirname(file), {recursive:true});
      fs.writeFileSync(file, content, {mode:0o755});
    }
    if (options.webAsset) {
      const web = path.join(bundle, 'Contents/Resources/WebBundle');
      const asset = path.join(web, options.webAsset);
      if (options.webDirectoryAlias) {
        const physical = options.webAliasOutside ? path.join(root, 'outside-web-alias') : path.join(web, 'alias-target');
        fs.mkdirSync(physical, {recursive:true});
        fs.writeFileSync(path.join(physical, path.basename(asset)), 'export const syntheticBrowserAsset = true;');
        if (options.webAliasCycle) fs.symlinkSync(web, path.join(physical, 'loop'));
        fs.mkdirSync(path.dirname(path.dirname(asset)), {recursive:true});
        fs.symlinkSync(physical, path.dirname(asset));
      } else {
        fs.mkdirSync(path.dirname(asset), {recursive:true});
        if (options.webAssetSymlink) fs.symlinkSync(path.join(web, 'entry.js'), asset);
        else fs.writeFileSync(asset, 'export const syntheticBrowserAsset = true;');
      }
    }
    if (options.localLibrary) {
      fs.mkdirSync(path.join(bundle, 'Contents/Frameworks'), {recursive:true});
      fs.writeFileSync(path.join(bundle, 'Contents/Frameworks/libFixture.dylib'), 'synthetic local library');
    }
    if (options.escapedLibrary) {
      fs.mkdirSync(path.join(bundle, 'Contents/Frameworks'), {recursive:true});
      fs.writeFileSync(path.join(root, 'outside.dylib'), 'synthetic external library');
      fs.symlinkSync(path.join(root, 'outside.dylib'), path.join(bundle, 'Contents/Frameworks/libFixture.dylib'));
    }
  }

  /** @param {string} tool @param {string[]} arguments_ @param {LoaderFixture} options @returns {{status: number, stdout: string, stderr: string, error: null}} */
  static tool(tool, arguments_, options) {
    const binary = arguments_.at(-1) ?? '';
    if (tool === '/usr/bin/plutil') return this.result(JSON.stringify({CFBundleIdentifier:'fixture.synthetic.loader', CFBundleExecutable:'Main', CFBundleIconFile:'Icon'}));
    if (tool === '/usr/bin/file') return this.result('Mach-O 64-bit executable arm64');
    if (tool === '/usr/bin/codesign') return this.result('Signature=adhoc');
    if (tool !== '/usr/bin/otool') throw new Error('Unexpected fixture tool: '+tool);
    if (arguments_[0] === '-L') {
      const references = path.basename(binary) === 'Main' ? options.libraries ?? ['/usr/lib/libSystem.B.dylib'] : options.emitterLibraries ?? ['/usr/lib/libSystem.B.dylib'];
      const body = references.map(/** @param {string} reference @returns {string} */ reference => '\t'+reference.replaceAll('{bundle}', path.resolve(path.dirname(binary), '../..'))+' (compatibility version 1.0.0, current version 1.0.0)').join('\n');
      return this.result(options.rawLibraries ?? binary+':\n'+body+'\n');
    }
    assert.equal(arguments_[0], '-l');
    const paths = path.basename(binary) === 'Main' ? options.rpaths ?? [] : [];
    const commands = paths.map(/** @param {string} reference @param {number} index @returns {string} */ (reference, index) => 'Load command '+(index+1)+'\n          cmd LC_RPATH\n      cmdsize 64\n         path '+reference+' (offset 12)\n').join('');
    return this.result((options.rawLoads ?? binary+':\nLoad command 0\n          cmd LC_UUID\n      cmdsize 24\n'+commands).replaceAll('{binary}', binary), options.loadStatus ?? 0);
  }

  /** @param {string} stdout @param {number} status @returns {{status: number, stdout: string, stderr: string, error: null}} */
  static result(stdout, status = 0) { return {status, stdout, stderr:'', error:null}; }

  /** @param {{checks: LoaderCheck[]}} report @param {string} role @returns {LoaderCheck} */
  static policy(report, role = 'main') {
    const check = report.checks.find(/** @param {LoaderCheck} item @returns {boolean} */ item => item.name === role+'-loader-path-policy');
    assert.ok(check, 'Public inspection must report the declared loader-path policy');
    return check;
  }

  /** @param {{checks: LoaderCheck[]}} report @returns {LoaderCheck} */
  static retiredPolicy(report) {
    const check = report.checks.find(/** @param {LoaderCheck} item @returns {boolean} */ item => item.name === 'no-retired-browser-adapters');
    assert.ok(check, 'Public inspection must report the retired browser adapter policy');
    return check;
  }

  /** @param {import('node:test').TestContext} context @param {string} asset @param {LoaderFixture} [options] @returns {void} */
  static assertRetired(context, asset, options = {}) {
    const inspected = this.inspect(context, {...options, webAsset:asset});
    assert.equal(inspected.result, 1, 'The actual inspector must reject a retired browser asset: '+asset);
    assert.equal(inspected.report.conditions_passed, false);
    const policy = this.retiredPolicy(inspected.report);
    assert.equal(policy.passed, false);
    assert.deepEqual(policy.evidence, ['Contents/Resources/WebBundle/'+asset]);
    assert.deepEqual(inspected.report.checks.filter(/** @param {LoaderCheck} check @returns {boolean} */ check => !check.passed)
      .map(/** @param {LoaderCheck} check @returns {string} */ check => check.name), ['no-retired-browser-adapters']);
  }
}

test('public bundle inspection accepts declared macOS system libraries without filesystem cache probes', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['/usr/lib/libSystem.B.dylib', '/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit']});
  assert.equal(inspected.result, 0);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, true);
});

test('public bundle inspection rejects an absolute source library', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['/Users/fixture/project/libPrivate.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public bundle inspection rejects a toolchain rpath even with system dependencies', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {rpaths:['/Library/Developer/CommandLineTools/usr/lib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public bundle inspection resolves an existing loader-relative bundled library', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@loader_path/../Frameworks/libFixture.dylib'], localLibrary:true});
  assert.equal(inspected.result, 0);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, true);
});

test('public bundle inspection resolves executable-relative bundled libraries', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@executable_path/../Frameworks/libFixture.dylib'], localLibrary:true});
  assert.equal(inspected.result, 0);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, true);
});

test('public bundle inspection resolves a bundled rpath candidate', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@rpath/libFixture.dylib'], rpaths:['@executable_path/../Frameworks'], localLibrary:true});
  assert.equal(inspected.result, 0);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, true);
});

test('public bundle inspection keeps system rpath declarations distinct from cache existence proof', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@rpath/libFixtureFromSystemCache.dylib'], rpaths:['/usr/lib/swift']});
  assert.equal(inspected.result, 0);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, true);
  assert.match(JSON.stringify(NativeLoaderPolicyTest.policy(inspected.report).evidence), /system-declared/);
});

test('public bundle inspection refuses rpath dependencies without a supported search path', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@rpath/libFixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public bundle inspection refuses missing loader-relative files', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@loader_path/../Frameworks/missing.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses a symlink library that escapes the bundle', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@loader_path/../Frameworks/libFixture.dylib'], escapedLibrary:true});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses directory traversal outside the bundle', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@loader_path/../../../outside.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses unsupported loader macros', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['@unknown/libFixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses bare relative library references', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['libFixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses system-prefix lookalikes', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['/usr/library/libFixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses normalized absolute paths escaping system roots', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['/usr/lib/../../tmp/libFixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses unreadable load-command metadata', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {loadStatus:1});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses malformed dependency metadata instead of ignoring it', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {rawLibraries:'invalid synthetic metadata\nlibUnknown.dylib\n'});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses malformed rpath commands', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {rawLoads:'{binary}:\nLoad command 0\ncmd LC_RPATH\ncmdsize 64\n'});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses malformed command types with a valid executable header', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {rawLoads:'{binary}:\nLoad command 0\ngarbage metadata\n'});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses an absolute path to the current bundle that would break relocation', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['{bundle}/Contents/Frameworks/libFixture.dylib'], localLibrary:true});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public loader policy refuses control characters in declared system dependencies', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {libraries:['/usr/lib/libInvalid\0Fixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report).passed, false);
});

test('public inspection applies the same loader policy to the event emitter', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {emitterLibraries:['/opt/homebrew/lib/libFixture.dylib']});
  assert.equal(inspected.result, 1);
  assert.equal(NativeLoaderPolicyTest.policy(inspected.report, 'emitter').passed, false);
});

for (const asset of ['shared/scripts/ChartRenderer.js', 'shared/scripts/GraphRenderer.js',
  'shared/scripts/GraphGroupRenderer.js', 'shared/scripts/ViewContinuity.js',
  'utils/PresentationUtils.js', 'utils/DomUtils.js']) {
  for (const prefix of ['', 'tachyon-runtime/client/']) {
    test('public inspection rejects retired browser adapter '+prefix+asset, /** @param {import('node:test').TestContext} context @returns {void} */ context => {
      NativeLoaderPolicyTest.assertRetired(context, prefix+asset);
    });
  }
}

test('public inspection rejects a retired browser adapter in a deeper generated subtree', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  NativeLoaderPolicyTest.assertRetired(context, 'generated/runtime/client/shared/scripts/ViewContinuity.js');
});

test('public inspection rejects a retired browser adapter path supplied by an internal symlink', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  NativeLoaderPolicyTest.assertRetired(context, 'tachyon-runtime/client/utils/DomUtils.js', {webAssetSymlink:true});
});

test('public inspection rejects a retired browser adapter behind an internal directory alias', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  NativeLoaderPolicyTest.assertRetired(context, 'tachyon-runtime/client/shared/scripts/GraphRenderer.js', {webDirectoryAlias:true});
});

test('public inspection refuses an external directory alias without traversing retired files outside the bundle', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const asset = 'tachyon-runtime/client/shared/scripts/GraphRenderer.js';
  const inspected = NativeLoaderPolicyTest.inspect(context, {webAsset:asset, webDirectoryAlias:true, webAliasOutside:true});
  assert.equal(inspected.result, 1);
  assert.deepEqual(inspected.report.checks.filter(/** @param {LoaderCheck} check @returns {boolean} */ check => !check.passed)
    .map(/** @param {LoaderCheck} check @returns {string} */ check => check.name), ['symlink:Contents/Resources/WebBundle/'+path.dirname(asset)]);
  assert.equal(NativeLoaderPolicyTest.retiredPolicy(inspected.report).passed, true);
  assert.deepEqual(NativeLoaderPolicyTest.retiredPolicy(inspected.report).evidence, []);
});

test('public inspection permits an active GraphService directory alias with an internal directory cycle', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const inspected = NativeLoaderPolicyTest.inspect(context, {webAsset:'services/browser/GraphService.js', webDirectoryAlias:true, webAliasCycle:true});
  assert.equal(inspected.result, 0);
  assert.equal(inspected.report.conditions_passed, true);
  assert.deepEqual(NativeLoaderPolicyTest.retiredPolicy(inspected.report).evidence, []);
});

for (const asset of ['services/browser/GraphService.js', 'tachyon-runtime/client/services/browser/GraphService.js',
  'generated/runtime/client/shared/scripts/ArchivedGraphRenderer.js']) {
  test('public inspection permits active GraphService or distinct exact asset path '+asset, /** @param {import('node:test').TestContext} context @returns {void} */ context => {
    const inspected = NativeLoaderPolicyTest.inspect(context, {webAsset:asset});
    assert.equal(inspected.result, 0);
    assert.equal(inspected.report.conditions_passed, true);
    assert.equal(NativeLoaderPolicyTest.retiredPolicy(inspected.report).passed, true);
    assert.deepEqual(NativeLoaderPolicyTest.retiredPolicy(inspected.report).evidence, []);
  });
}
