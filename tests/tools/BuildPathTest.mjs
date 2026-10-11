import assert from 'node:assert/strict';
import {test} from 'node:test';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {spawnSync} from 'node:child_process';

/** @typedef {{root: string, project: string, caller: string, compiler: string, capture: string, artifacts: string}} BuildFixture */

class BuildPathTest {
  /** @returns {BuildFixture} */
  static fixture() {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'build-path-check-'));
    const project = path.join(root, 'project with spaces');
    const caller = path.join(root, 'other caller');
    fs.mkdirSync(path.join(project, 'tools/utils'), {recursive:true});
    fs.mkdirSync(caller);
    fs.copyFileSync('build.sh', path.join(project, 'build.sh'));
    fs.copyFileSync('tools/utils/ToolPathUtils.ts', path.join(project, 'tools/utils/ToolPathUtils.ts'));
    fs.writeFileSync(path.join(root, 'tsc'), '#!/bin/sh\nexit 0\n', {mode:0o755});
    const compiler = path.join(root, 'compiler');
    fs.writeFileSync(compiler, '#!/bin/sh\npwd > "$OBSERVATORY_TEST_CAPTURE"\nprintf "%s\\n" "$@" >> "$OBSERVATORY_TEST_CAPTURE"\nexit 37\n', {mode:0o755});
    return {root, project, caller, compiler, capture:path.join(root, 'chosen-project.txt'), artifacts:path.join(root, 'external output with spaces')};
  }

  /** @param {BuildFixture} fixture @param {string} artifacts @param {Record<string, string>} overrides @returns {import('node:child_process').SpawnSyncReturns<string>} */
  static invoke(fixture, artifacts, overrides = {}) {
    return spawnSync('/bin/zsh', [path.join(fixture.project, 'build.sh')], {
      cwd:fixture.caller,
      env:{...process.env, PATH:fixture.root + path.delimiter + process.env.PATH, TACHYON_COMPILER:fixture.compiler, OBSERVATORY_TEST_CAPTURE:fixture.capture,
        OBSERVATORY_NATIVE_ARTIFACTS:artifacts, OBSERVATORY_SWIFT_CACHE_PATH:path.join(fixture.root, 'swift cache'),
        OBSERVATORY_RUST_CACHE_PATH:path.join(fixture.root, 'rust cache'), OBSERVATORY_ROLLBACK_ROOT:path.join(fixture.root, 'rollback'), ...overrides},
      encoding:'utf8'
    });
  }
}

test('build selects script source and explicitly forwards external output containing spaces', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture = BuildPathTest.fixture();
  context.after(/** @returns {void} */ () => fs.rmSync(fixture.root, {recursive:true}));
  const result = BuildPathTest.invoke(fixture, fixture.artifacts);
  assert.equal(result.status, 37, result.stderr);
  assert.deepEqual(fs.readFileSync(fixture.capture, 'utf8').trim().split('\n'), [fixture.project, 'bundle', 'app', '--out-dir', fixture.artifacts, '--target', 'macos', '--native']);
  assert.equal(fs.existsSync(path.join(fixture.project, 'app/dist')), false);
  assert.equal(fs.existsSync(path.join(fixture.project, 'tools/swift-cache')), false);
  assert.equal(fs.existsSync(path.join(fixture.caller, 'tools')), false);
});

test('build rejects a configured source output before compiler invocation', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture = BuildPathTest.fixture();
  context.after(/** @returns {void} */ () => fs.rmSync(fixture.root, {recursive:true}));
  const result = BuildPathTest.invoke(fixture, path.join(fixture.project, 'app/dist'));
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /outside the source project/);
  assert.equal(fs.existsSync(fixture.capture), false);
  assert.equal(fs.existsSync(path.join(fixture.project, 'app/dist')), false);
});


test('build rejects a rollback below the promoted native output before copying or invoking the compiler', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture = BuildPathTest.fixture();
  context.after(/** @returns {void} */ () => fs.rmSync(fixture.root, {recursive:true}));
  const bundle = path.join(fixture.artifacts, 'macos/Swarx.app');
  fs.mkdirSync(bundle, {recursive:true});
  fs.writeFileSync(path.join(bundle, 'preserved.txt'), 'last accepted app');
  const rollback = path.join(fixture.artifacts, 'macos/rollback');
  const result = BuildPathTest.invoke(fixture, fixture.artifacts, {OBSERVATORY_ROLLBACK_ROOT:rollback});
  assert.notEqual(result.status, 37);
  assert.match(result.stderr, /separate/);
  assert.equal(fs.existsSync(fixture.capture), false);
  assert.equal(fs.existsSync(rollback), false);
  assert.equal(fs.readFileSync(path.join(bundle, 'preserved.txt'), 'utf8'), 'last accepted app');
});

test('build rejects cache paths that physically alias each other before invoking the compiler', /** @param {import('node:test').TestContext} context @returns {void} */ context => {
  const fixture = BuildPathTest.fixture();
  context.after(/** @returns {void} */ () => fs.rmSync(fixture.root, {recursive:true}));
  const cache = path.join(fixture.root, 'shared cache');
  const alias = path.join(fixture.root, 'cache alias');
  fs.mkdirSync(cache);
  fs.symlinkSync(cache, alias, 'dir');
  const result = BuildPathTest.invoke(fixture, fixture.artifacts, {OBSERVATORY_RUST_CACHE_PATH:cache, OBSERVATORY_SWIFT_CACHE_PATH:alias});
  assert.notEqual(result.status, 37);
  assert.match(result.stderr, /separate/);
  assert.equal(fs.existsSync(fixture.capture), false);
});
