import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import assert from 'node:assert/strict';

/** @typedef {{name: string, passed: boolean, detail: string, evidence?: unknown}} InspectionCheck */
/** @typedef {{status: number | null, stdout: string, stderr: string, error: string | null}} InspectionToolResult */
/** @typedef {{reference: string, passed: boolean, kind: string, local?: string}} InspectionAsset */
/** @typedef {{name: string, content: string}} InspectionModuleSource */
/** @typedef {{name: string, references: string[], error: string | null}} InspectionModuleRecord */
/** @typedef {{imports: InspectionAsset[], ignored_nonlocal_literals: number, parse_error: string | null}} InspectionModuleEvidence */
/** @typedef {InspectionCheck & {evidence: InspectionModuleEvidence}} InspectionModuleCheck */

export class BundleInspectionTest {
  /** @type {number} */
  static #TOOL_TIMEOUT = 10000;
  /** @type {number} */
  static #TOOL_OUTPUT_LIMIT = 262144;
  /** @type {string[]} */
  static #LOCAL_ASSETS = ['shared/assets/observatory.svg', 'shared/assets/duvay/duvay-wc.min.js', 'shared/assets/duvay/duvay.min.js', 'shared/assets/duvay/duvay.min.css', 'shared/assets/chartjs/chart.umd.min.js'];
  /** @type {string} */
  static #WEB_PREFIX = 'Contents/Resources/WebBundle/';
  /** @type {string[]} */
  static #VENDOR_PREFIXES = ['shared/assets/duvay/', 'shared/assets/chartjs/', 'vendor/'];
  /** @type {string[]} */
  static #LOCAL_MODULE_PREFIXES = ['./', '../', '/'];
  /** @type {string} */
  static #MODULE_PARSER = `
import {readFileSync} from 'node:fs';
import {SourceTextModule} from 'node:vm';
/** @type {{name: string, content: string}[]} */
const input = JSON.parse(readFileSync(0, 'utf8'));
const records = input.map(/** @param {{name: string, content: string}} record @returns {{name: string, references: string[], error: string | null}} */ record => {
  try {
    return {name: record.name, references: new SourceTextModule(record.content, {identifier: record.name}).dependencySpecifiers, error: null};
  } catch {
    return {name: record.name, references: [], error: 'JavaScript module syntax could not be parsed.'};
  }
});
process.stdout.write(JSON.stringify(records));
`;
  /** @type {string} */
  #bundle;
  /** @type {string} */
  #report;
  /** @type {string} */
  #label;
  /** @type {InspectionCheck[]} */
  #checks = [];
  /** @type {string[]} */
  #files = [];
  /** @type {Record<string, unknown>} */
  #information = {};

  /** @param {string} bundle @param {string} report @param {string} label */
  constructor(bundle, report, label) {
    this.#bundle = fs.realpathSync(bundle);
    this.#report = path.resolve(report);
    this.#label = label;
    if (!this.#bundle.endsWith('.app') || !fs.statSync(this.#bundle).isDirectory()) throw new Error('Supply an existing .app directory.');
    if (!this.#report.endsWith('.json')) throw new Error('Supply an explicit new JSON report path.');
    if (!/^[A-Za-z0-9 ._-]{1,80}$/.test(label)) throw new Error('Use a short plain baseline label.');
    this.#requireExternalReport();
  }

  /** @returns {number} */
  static main() {
    const arguments_ = process.argv.slice(2);
    if (arguments_.length === 2 && arguments_[0] === '--fixtures') return this.fixtures(arguments_[1]);
    if (arguments_.length < 2 || arguments_.length > 3) {
      console.error('Usage: node tests/BundleInspectionTest.mjs /explicit/App.app /explicit/new-report.json [baseline-label]');
      return 2;
    }
    try {
      return new BundleInspectionTest(arguments_[0], arguments_[1], arguments_[2] ?? 'baseline').run();
    } catch (error) {
      console.error(error instanceof Error ? error.message : 'Bundle inspection failed.');
      return 2;
    }
  }

  /** @returns {number} */
  run() {
    this.#walk(this.#bundle);
    this.#inspectInformation();
    this.#inspectBinary('main', this.#executable());
    this.#inspectBinary('emitter', 'observatory-event');
    this.#inspectBundleSignature();
    this.#inspectResources();
    this.#inspectHtmlAssets();
    this.#inspectModuleImports();
    this.#inspectLegacyResources();
    this.#writeReport();
    const failed = this.#checks.filter(/** @param {InspectionCheck} check @returns {boolean} */ check => !check.passed);
    console.log(`${this.#label}: ${this.#checks.length - failed.length}/${this.#checks.length} conditions passed; ${failed.length} failed. Report: ${this.#report}`);
    return failed.length === 0 ? 0 : 1;
  }

  /** @returns {void} */
  #requireExternalReport() {
    const parent = path.dirname(this.#report);
    let ancestor = parent;
    while (!fs.existsSync(ancestor)) ancestor = path.dirname(ancestor);
    const physicalParent = path.resolve(fs.realpathSync(ancestor), path.relative(ancestor, parent));
    if (this.#inside(this.#bundle, this.#report) || this.#inside(this.#bundle, physicalParent)) throw new Error('The report must be outside the inspected bundle.');
    if (fs.existsSync(this.#report)) throw new Error('The report path already exists; choose a new path.');
  }

  /** @param {string} root @param {string} file @returns {boolean} */
  #inside(root, file) {
    const relative = path.relative(root, file);
    return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
  }

  /** @param {string} directory @returns {void} */
  #walk(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      const relative = path.relative(this.#bundle, absolute);
      if (entry.isSymbolicLink()) {
        let contained = false;
        try { contained = this.#inside(this.#bundle, fs.realpathSync(absolute)); } catch {}
        this.#check(`symlink:${relative}`, contained, 'Symlink target exists inside the inspected bundle.');
      } else if (entry.isDirectory()) {
        this.#walk(absolute);
      } else if (entry.isFile()) {
        this.#files.push(relative);
      }
    }
  }

  /** @param {string} name @param {boolean} passed @param {string} detail @param {unknown} [evidence] @returns {void} */
  #check(name, passed, detail, evidence) {
    this.#checks.push({ name, passed, detail, ...evidence === undefined ? {} : { evidence } });
  }

  /** @param {string} tool @param {string[]} arguments_ @returns {InspectionToolResult} */
  #tool(tool, arguments_) {
    const result = spawnSync(tool, arguments_, { encoding: 'utf8', timeout: BundleInspectionTest.#TOOL_TIMEOUT, maxBuffer: BundleInspectionTest.#TOOL_OUTPUT_LIMIT });
    return { status: result.status, stdout: result.stdout ?? '', stderr: result.stderr ?? '', error: result.error?.message ?? null };
  }

  /** @returns {void} */
  #inspectInformation() {
    const information = path.join(this.#bundle, 'Contents/Info.plist');
    const result = this.#tool('/usr/bin/plutil', ['-convert', 'json', '-o', '-', information]);
    let parsed = false;
    try {
      const candidate = JSON.parse(result.stdout);
      parsed = candidate !== null && typeof candidate === 'object' && !Array.isArray(candidate);
      if (parsed) this.#information = candidate;
    } catch {}
    this.#check('info-plist', result.status === 0 && parsed, 'Info.plist parses through read-only plutil stdout conversion.', result.error);
    const identifier = this.#text('CFBundleIdentifier');
    this.#check('bundle-identifier', /^[A-Za-z0-9][A-Za-z0-9.-]+$/.test(identifier), 'CFBundleIdentifier is a nonempty bundle identifier.', identifier);
    const executable = this.#executable();
    this.#check('declared-executable', this.#safeName(executable), 'CFBundleExecutable declares a single file name.', executable);
    const declaredIcon = this.#text('CFBundleIconFile');
    const icon = declaredIcon.endsWith('.icns') ? declaredIcon : `${declaredIcon}.icns`;
    this.#check('declared-local-icon', this.#safeName(declaredIcon) && this.#nonempty(path.join(this.#bundle, 'Contents/Resources', icon)), 'CFBundleIconFile resolves to a nonempty local .icns file.', icon);
    const iconPath = path.join(this.#bundle, 'Contents/Resources', icon);
    const iconHeader = this.#nonempty(iconPath) ? fs.readFileSync(iconPath).subarray(0, 4).toString('ascii') : '';
    this.#check('local-icon-format', iconHeader === 'icns', 'The local icon has the actual icns container header.');
  }

  /** @param {string} key @returns {string} */
  #text(key) {
    return typeof this.#information[key] === 'string' ? this.#information[key] : '';
  }

  /** @returns {string} */
  #executable() { return this.#text('CFBundleExecutable'); }

  /** @param {string} value @returns {boolean} */
  #safeName(value) { return value.length > 0 && value !== '.' && value !== '..' && path.basename(value) === value; }

  /** @param {string} file @returns {boolean} */
  #nonempty(file) {
    try { return fs.statSync(file).isFile() && fs.statSync(file).size > 0; } catch { return false; }
  }

  /** @param {string} role @param {string} name @returns {void} */
  #inspectBinary(role, name) {
    const binary = path.join(this.#bundle, 'Contents/MacOS', this.#safeName(name) ? name : 'invalid-declared-executable');
    const present = this.#nonempty(binary);
    this.#check(`${role}-binary-present`, present, 'Expected nonempty binary is bundled.', path.relative(this.#bundle, binary));
    if (!present) return;
    this.#check(`${role}-executable-mode`, (fs.statSync(binary).mode & 0o111) !== 0, 'Bundled binary has an executable permission bit.');
    const identification = this.#tool('/usr/bin/file', ['-b', binary]);
    this.#check(`${role}-mach-o`, identification.status === 0 && identification.stdout.includes('Mach-O') && identification.stdout.includes('executable'), 'file identifies an actual Mach-O executable.', identification.stdout.trim());
    const libraries = this.#tool('/usr/bin/otool', ['-L', binary]);
    const dependencies = libraries.stdout.split('\n').map(/** @param {string} line @returns {string} */ line => line.trim()).filter(/** @param {string} line @returns {boolean} */ line => /\(compatibility version [^)]*\)$/.test(line));
    const python = dependencies.filter(/** @param {string} line @returns {boolean} */ line => /(?:libpython[^/\s]*\.(?:dylib|so)|Python\.framework\/|\/python(?:[23](?:\.\d+)*)?(?:\s|$))/i.test(line));
    this.#check(`${role}-linked-dependencies`, libraries.status === 0, 'otool reads actual loader dependencies.', { dependencies, error: libraries.error });
    this.#check(`${role}-no-python-link`, libraries.status === 0 && python.length === 0, 'No Python interpreter/library appears in actual linked dependencies; debug symbols are not searched.', python);
    const signature = this.#tool('/usr/bin/codesign', ['-d', '--verbose=4', binary]);
    const metadata = `${signature.stdout}\n${signature.stderr}`;
    this.#check(`${role}-adhoc-signature`, signature.status === 0 && /Signature=adhoc|flags=.*adhoc/.test(metadata), 'codesign display identifies an ad-hoc signature.', { status: signature.status, description: metadata.trim(), error: signature.error });
  }

  /** @returns {void} */
  #inspectBundleSignature() {
    const result = this.#tool('/usr/bin/codesign', ['--verify', '--deep', '--strict', this.#bundle]);
    this.#check('bundle-signature-valid', result.status === 0, 'Read-only codesign --verify --deep --strict succeeds; this does not establish notarization or launch.', { status: result.status, description: result.stderr.trim(), error: result.error });
  }

  /** @returns {void} */
  #inspectResources() {
    const resources = path.join(this.#bundle, 'Contents/Resources');
    const web = path.join(resources, 'WebBundle');
    this.#check('web-bundle', fs.existsSync(web) && fs.statSync(web).isDirectory(), 'WebBundle directory is bundled.');
    const index = path.join(resources, 'NativeIndex.json');
    let parsed = null;
    try { parsed = JSON.parse(fs.readFileSync(index, 'utf8')); } catch {}
    const document = typeof parsed?.entry_document === 'string' ? this.#asset(parsed.entry_document, web, 'index.html') : null;
    this.#check('native-index', parsed !== null && typeof parsed === 'object' && document?.passed === true, 'NativeIndex.json parses and its entry document resolves locally.', document);
    for (const asset of BundleInspectionTest.#LOCAL_ASSETS) this.#check(`local-asset:${asset}`, this.#nonempty(path.join(web, asset)), 'Required local icon/vendor asset is nonempty.');
  }

  /** @param {string} reference @param {string} web @param {string} document @returns {InspectionAsset} */
  #asset(reference, web, document) {
    const clean = reference.split(/[?#]/, 1)[0];
    if (reference.startsWith('data:')) return { reference: 'embedded data asset', passed: true, kind: 'embedded' };
    if (/^(?:[A-Za-z][A-Za-z0-9+.-]*:|\/\/)/.test(reference)) return { reference: clean, passed: false, kind: 'external-or-unsupported-scheme' };
    let decoded;
    try { decoded = decodeURIComponent(clean); } catch { return { reference: clean, passed: false, kind: 'invalid-url-encoding' }; }
    const relative = decoded.startsWith('/') ? decoded.slice(1) : path.join(path.dirname(document), decoded);
    const aliased = relative.replace(/^\.tachyon(?:\/|$)/, 'tachyon-runtime/');
    const target = path.resolve(web, aliased);
    const contained = this.#inside(web, target);
    const present = contained && this.#nonempty(target);
    const physical = present && this.#inside(fs.realpathSync(web), fs.realpathSync(target));
    const kind = !contained ? 'path-escape' : present && !physical ? 'symlink-path-escape' : 'local';
    return { reference: clean, passed: present && physical, kind, local: path.relative(web, target) };
  }

  /** @returns {void} */
  #inspectModuleImports() {
    const web = path.join(this.#bundle, 'Contents/Resources/WebBundle');
    const modules = this.#files.filter(/** @param {string} file @returns {boolean} */ file => this.#moduleAsset(file));
    this.#check('javascript-module-assets-present', modules.length > 0, 'At least one nonvendor JavaScript asset is present.');
    const parsed = this.#parseModules(modules);
    for (const file of modules) this.#inspectModule(web, file, parsed.get(file));
  }

  /** @param {string[]} files @returns {Map<string, InspectionModuleRecord>} */
  #parseModules(files) {
    const input = files.map(/** @param {string} file @returns {InspectionModuleSource} */ file => ({name: file, content: fs.readFileSync(path.join(this.#bundle, file), 'utf8')}));
    const result = spawnSync(process.execPath, ['--experimental-vm-modules', '--input-type=module', '-e', BundleInspectionTest.#MODULE_PARSER],
      {input: JSON.stringify(input), encoding: 'utf8', timeout: BundleInspectionTest.#TOOL_TIMEOUT, maxBuffer: BundleInspectionTest.#TOOL_OUTPUT_LIMIT});
    /** @type {InspectionModuleRecord[]} */
    let records = [];
    try { records = JSON.parse(result.stdout); } catch {}
    const parsed = Array.isArray(records) && records.length === files.length;
    this.#check('javascript-module-parser', result.status === 0 && parsed,
      'One bounded Node/V8 child parses static module requests without linking, evaluating or executing bundled JavaScript.',
      {status: result.status, parsed_assets: parsed ? records.length : 0, error: result.error?.message ?? null});
    return new Map((parsed ? records : []).map(/** @param {InspectionModuleRecord} record @returns {[string, InspectionModuleRecord]} */ record => [record.name, record]));
  }

  /** @param {string} file @returns {boolean} */
  #moduleAsset(file) {
    const relative = file.slice(BundleInspectionTest.#WEB_PREFIX.length);
    return [file.startsWith(BundleInspectionTest.#WEB_PREFIX), /\.(?:js|mjs)$/i.test(file),
      !BundleInspectionTest.#VENDOR_PREFIXES.some(/** @param {string} prefix @returns {boolean} */ prefix => relative.startsWith(prefix))].every(Boolean);
  }

  /** @param {string} web @param {string} file @param {InspectionModuleRecord | undefined} parsed @returns {void} */
  #inspectModule(web, file, parsed) {
    const document = path.relative(web, path.join(this.#bundle, file));
    const literals = parsed?.references ?? [];
    const local = literals.filter(/** @param {string} reference @returns {boolean} */ reference => this.#localModule(reference));
    const references = local.map(/** @param {string} reference @returns {InspectionAsset} */ reference => this.#asset(reference, web, document));
    this.#check(`javascript-local-imports:${document}`, parsed?.error === null && references.every(/** @param {InspectionAsset} reference @returns {boolean} */ reference => reference.passed),
      'Literal local static import/export-from targets exist physically inside WebBundle; comments, strings and dynamic imports are not declarations.',
      { imports: references, ignored_nonlocal_literals: literals.length - local.length, parse_error: parsed?.error ?? null });
  }

  /** @param {string} reference @returns {boolean} */
  #localModule(reference) { return BundleInspectionTest.#LOCAL_MODULE_PREFIXES.some(/** @param {string} prefix @returns {boolean} */ prefix => reference.startsWith(prefix)); }

  /** @param {string} report @returns {number} */
  static fixtures(report) {
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'observatory-bundle-modules-'));
    try {
      this.#exerciseFixtures(temporary, report);
      console.log('PASS: bundle static-import positive/negative fixtures, containment, query redaction and declaration-only parsing.');
      return 0;
    } catch (error) {
      console.error(error instanceof Error ? error.message : 'Bundle module fixtures failed.');
      return 1;
    } finally {
      fs.rmSync(temporary, {recursive: true, force: true});
    }
  }

  /** @param {string} temporary @param {string} report @returns {void} */
  static #exerciseFixtures(temporary, report) {
    const bundle = path.join(temporary, 'Fixture.app');
    const web = path.join(bundle, 'Contents/Resources/WebBundle');
    const assets = {
      'good/entry.mjs': "import './second.js?token=fixture-private-token';\nexport {v} from '../leaf.js';\nimport '/.tachyon/root.js';",
      'good/second.js': "import './nested.js';",
      'good/nested.js': 'export const nested = true;',
      'leaf.js': 'export const v = true;',
      'tachyon-runtime/root.js': 'export const root = true;',
      'ignored.js': "/* import './comment-missing.js'; */\nconst text = \"import './string-missing.js'\";\nconst template = `\nimport './template-missing.js';\n`;\nvoid import('./dynamic-missing.js');",
      'missing.js': "import './not-here.js?token=fixture-private-token';\nexport * from './missing-export.js#fixture-private-fragment';",
      'invalid-url.js': "import './invalid%ZZ.js?token=fixture-private-token';",
      'invalid-syntax.js': 'import {',
      'escape.js': "import '../../outside.js?token=fixture-private-token';",
      'symlink-reader.js': "import './escaped-link.js';",
      'shared/assets/duvay/vendor.js': "import './vendor-missing.js';"
    };
    for (const [name, content] of Object.entries(assets)) {
      const file = path.join(web, name);
      fs.mkdirSync(path.dirname(file), {recursive: true});
      fs.writeFileSync(file, content);
    }
    const outside = path.join(bundle, 'Contents/outside.js');
    fs.writeFileSync(outside, 'export const outside = true;');
    fs.symlinkSync(outside, path.join(web, 'escaped-link.js'));
    const inspection = new BundleInspectionTest(bundle, report, 'synthetic-module-closure-fixtures');
    inspection.#walk(inspection.#bundle);
    inspection.#inspectModuleImports();
    const check = /** @param {string} name @returns {InspectionCheck | undefined} */ name => inspection.#checks.find(
      /** @param {InspectionCheck} item @returns {boolean} */ item => item.name === `javascript-local-imports:${name}`);
    const requiredCheck = /** @param {string} name @returns {InspectionModuleCheck} */ name => {
      const found = check(name);
      assert.ok(found);
      assert.ok(found.evidence);
      return /** @type {InspectionModuleCheck} */ (found);
    };
    assert.ok(check('good/entry.mjs'), 'Expected entry-module check; recorded: ' + inspection.#checks.map(/** @param {InspectionCheck} item @returns {string} */ item => item.name).join(', '));
    assert.equal(requiredCheck('good/entry.mjs').passed, true);
    assert.equal(requiredCheck('good/second.js').passed, true);
    assert.equal(requiredCheck('ignored.js').passed, true);
    assert.deepEqual(requiredCheck('ignored.js').evidence.imports, []);
    assert.equal(requiredCheck('missing.js').passed, false);
    assert.equal(requiredCheck('missing.js').evidence.imports.length, 2);
    assert.equal(requiredCheck('invalid-url.js').evidence.imports[0].kind, 'invalid-url-encoding');
    assert.equal(requiredCheck('invalid-syntax.js').passed, false);
    assert.equal(requiredCheck('invalid-syntax.js').evidence.parse_error, 'JavaScript module syntax could not be parsed.');
    assert.equal(requiredCheck('escape.js').evidence.imports[0].kind, 'path-escape');
    assert.equal(requiredCheck('symlink-reader.js').evidence.imports[0].kind, 'symlink-path-escape');
    assert.equal(check('shared/assets/duvay/vendor.js'), undefined);
    const serialized = JSON.stringify(inspection.#checks);
    assert.ok(!serialized.includes('fixture-private-token') && !serialized.includes('fixture-private-fragment'));
    assert.throws(/** @returns {BundleInspectionTest} */ () => new BundleInspectionTest(bundle, path.join(web, 'inside.json'), 'invalid-fixture'));
    fs.symlinkSync(web, path.join(temporary, 'report-link'));
    assert.throws(/** @returns {BundleInspectionTest} */ () => new BundleInspectionTest(bundle, path.join(temporary, 'report-link/new.json'), 'invalid-fixture'));
    fs.writeFileSync(path.join(temporary, 'existing.json'), '{}');
    assert.throws(/** @returns {BundleInspectionTest} */ () => new BundleInspectionTest(bundle, path.join(temporary, 'existing.json'), 'invalid-fixture'));
    inspection.#writeReport();
  }

  /** @returns {void} */
  #inspectHtmlAssets() {
    const web = path.join(this.#bundle, 'Contents/Resources/WebBundle');
    const documents = this.#files.filter(/** @param {string} file @returns {boolean} */ file => file.startsWith('Contents/Resources/WebBundle/') && /\.html?$/i.test(file));
    this.#check('html-present', documents.length > 0, 'At least one bundled HTML document exists.');
    for (const file of documents) {
      const document = path.relative(web, path.join(this.#bundle, file));
      const html = fs.readFileSync(path.join(this.#bundle, file), 'utf8').replace(/<script\b(?![^>]*\bsrc\s*=)[^>]*>[\s\S]*?<\/script\s*>/gi, '<script></script>');
      const references = [...html.matchAll(/<(script|link)\b[^>]*\b(?:src|href)\s*=\s*(["'])(.*?)\2[^>]*>/gi)].map(/** @param {RegExpMatchArray} match @returns {InspectionAsset & {tag: string}} */ match => ({ tag: match[1].toLowerCase(), ...this.#asset(match[3], web, document) }));
      this.#check(`html-local-assets:${document}`, references.every(/** @param {InspectionAsset} reference @returns {boolean} */ reference => reference.passed), 'Literal script/link references resolve inside WebBundle (including native .tachyon alias) or are embedded; no CDN reference is accepted.', references);
    }
  }

  /** @returns {void} */
  #inspectLegacyResources() {
    const forbidden = this.#files.filter(/** @param {string} file @returns {boolean} */ file => /(?:^|\/)Collector(?:\/|$)|\.(?:py|pyc)$/i.test(file));
    const collector = fs.existsSync(path.join(this.#bundle, 'Contents/Resources/Collector'));
    this.#check('no-collector-or-python-source', !collector && forbidden.length === 0, 'No Collector resource or .py/.pyc file is bundled.', forbidden);
  }

  /** @returns {void} */
  #writeReport() {
    const report = { schema: 2, scope: 'read-only bundle preflight; baseline, not latest-build acceptance', label: this.#label, bundle: this.#bundle, inspected_at: new Date().toISOString(), conditions_passed: this.#checks.every(/** @param {InspectionCheck} check @returns {boolean} */ check => check.passed), checks: this.#checks, limitations: ['Does not launch the app, quit processes, touch the event socket, or certify relocation/lifecycle.', 'Does not verify native bridge resolution, live metrics, UI interactions, icon appearance or current source/compiler provenance.', 'Inspects literal HTML script/link attributes and nonvendor JS/MJS static relative/root import/export-from declarations. Does not resolve bare packages/import maps, dynamic imports, computed script URLs, CSS references or vendor module dependencies.', 'Uses the current Node/V8 module parser without linking or evaluating bundled JavaScript; export symbol compatibility and execution are not certified.', 'Uses the native .tachyon to tachyon-runtime resource alias; its presence does not prove the host serves it correctly.', 'Does not search debug strings or symbols for Python; only actual linked dependencies and bundled Python source resources are checked.', 'Ad-hoc signature verification does not establish notarization, Developer ID signing, Gatekeeper approval or target-host compatibility.'] };
    fs.mkdirSync(path.dirname(this.#report), { recursive: true });
    this.#requireExternalReport();
    fs.writeFileSync(this.#report, `${JSON.stringify(report, null, 2)}\n`, { encoding: 'utf8', flag: 'wx', mode: 0o600 });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) process.exitCode = BundleInspectionTest.main();
