import test, { type TestContext } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { NativeRustPackager } from '../../tools/NativeRustPackager.ts';
import { NativeHostCustomizer } from '../../tools/NativeHostCustomizer.ts';
import { IconPackager } from '../../tools/IconPackager.ts';
import { ToolCommandUtils } from '../../tools/utils/ToolCommandUtils.ts';
import { ToolPathUtils } from '../../tools/utils/ToolPathUtils.ts';
import { NativePackagerFixture } from './NativePackagerFixture.ts';

test('Rust packaging appends current exports, builds offline and copies library/emitter', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source = fixture.rust();
  const packager = new NativeRustPackager(fixture.root, fixture.run);
  packager.build();
  const body = fs.readFileSync(source, 'utf8');
  assert.ok(body.includes('pub use crate::TachyonRustServices::models;'));
  assert.ok(body.includes('ObservationClock, ProcessReader, ProcessIdentityReader, NetworkReader, MetricSource, EventSource, HarnessSessionSource, NativeMachTimebase, ClaudeHookAdapter, ClaudeHookMapper, ClaudeHookEventFactory'));
  assert.ok(body.includes('LogicalEventClientService, LogicalEventWireService, LogicalSocketService'));
  assert.ok(body.includes('SnapshotTransportTests.rs'));
  assert.deepEqual(fixture.commands[0], ['cargo', 'build', '--offline', '--manifest-path', path.join(fixture.rustCache, 'Cargo.toml'), '--target-dir', path.join(fixture.rustCache, 'target'), '--lib', '--bin', 'observatory-event', '--jobs', '2']);
  const manifest = fs.readFileSync(path.join(fixture.rustCache, 'Cargo.toml'), 'utf8');
  assert.ok(manifest.includes(`path = ${JSON.stringify(source)}`));
  assert.ok(manifest.includes('crate-type = ["staticlib", "rlib"]'));
  assert.ok(manifest.includes('libc = "=0.2.189"'));
  assert.ok(fs.readFileSync(path.join(fixture.rustCache, 'emitter.rs'), 'utf8').includes('EventEmitterMain::run()'));
  assert.ok(fs.readFileSync(path.join(fixture.rustCache, 'emitter.rs'), 'utf8').includes('use observatory_native::{models, services, ProcessIdentityReader, ClaudeHookAdapter};'));
  assert.equal(fs.readFileSync(path.join(fixture.artifacts, 'macos/project/libtachyoncompanion.a'), 'utf8'), 'fixture static library');
  const emitter = path.join(fixture.artifacts, 'macos/AgentObservatory.app/Contents/MacOS/observatory-event');
  assert.equal(fs.statSync(emitter).mode & 0o777, 0o755);
});

test('Rust second packaging rejects old companion before running or changing it', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source = fixture.rust();
  const packager = new NativeRustPackager(fixture.root, fixture.run);
  packager.build();
  const previous = fs.readFileSync(source, 'utf8');
  assert.throws((): void => packager.build(), /fresh Tachyon/);
  assert.equal(fs.readFileSync(source, 'utf8'), previous);
  assert.equal(fixture.commands.length, 1);
});

test('Host uses runtime icon companion with QA sources kept in tooling', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source: string = fixture.host();
  const customizer: NativeHostCustomizer = new NativeHostCustomizer(fixture.root, fixture.run);
  const customized: string = customizer.customize(fs.readFileSync(source, 'utf8'));
  assert.ok(customized.includes('NativeApplicationIcon.applyIcon()'));
  assert.equal(customized.includes('NativePreviewVerifier.applyIcon()'), false);
  customizer.build();
  assert.ok(fixture.commands[0]?.includes(path.join(fixture.root, 'app/client/shared/native/NativeApplicationIcon.swift')));
  assert.ok(fixture.commands[0]?.includes(path.join(fixture.swiftCache, 'NativePreviewVerifier.swift')));
  assert.ok(fixture.commands[0]?.includes(path.join(fixture.root, 'tools/native/NativeApplicationLifetimeVerifier.swift')));
  assert.ok(customized.includes('NativeApplicationLifetimeVerifier.begin(view: webView, window: window)'));
  assert.equal(fs.existsSync(path.join(fixture.root, 'app/client/shared/native/NativePreviewVerifier.swift')), false);
  assert.equal(fs.existsSync(path.join(fixture.root, 'app/client/shared/native/NativePreviewReport.js')), false);
  assert.ok(fs.readFileSync(path.join(fixture.swiftCache, 'NativePreviewVerifier.swift'), 'utf8').includes(JSON.stringify('const note = "$& $$";\n')));
});

test('Host customization preserves lifecycle code and is idempotent', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source = fixture.host();
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  const original = fs.readFileSync(source, 'utf8');
  const customized = customizer.customize(original);
  assert.ok(customized.includes('NSSize(width: 900, height: 640)'));
  assert.ok(customized.includes('NativeApplicationIcon.applyIcon()'));
  assert.ok(customized.includes('NativePreviewVerifier.begin(view: webView, window: window)'));
  assert.equal(customizer.customize(customized), customized);
  assert.ok(customized.includes('Telemetry.shared.record("controller.active")'));
});

test('Host rejects stale and missing native anchors', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  assert.throws((): string => customizer.customize('private func verifyPreview()'), /Rebuild/);
  assert.throws((): string => customizer.customize('missing'), /minimum/);
  assert.throws((): string => customizer.customize('window.contentMinSize = NSSize(width: 0, height: 0)'), /anchor/);
});

test('Host builds with static companion and preserves literal embedded report characters', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  fixture.host();
  const collector = fixture.write('app/dist/macos/AgentObservatory.app/Contents/Resources/Collector/old.txt', 'legacy fixture');
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  customizer.build();
  assert.equal(fs.existsSync(collector), false);
  const verifier = fs.readFileSync(path.join(fixture.swiftCache, 'NativePreviewVerifier.swift'), 'utf8');
  assert.ok(verifier.includes(JSON.stringify('const note = "$& $$";\n')));
  assert.deepEqual(fixture.commands[0], customizer.compilerCommand());
  assert.ok(fixture.commands[0]?.includes(path.join(fixture.artifacts, 'macos/project/libtachyoncompanion.a')));
  assert.ok(fixture.commands[0]?.includes('-module-cache-path'));
  assert.ok(fixture.commands[0]?.includes('-lsqlite3'), 'The host links the system SQLite library used by the local Codex title reader.');
});

test('Host refuses non-directory Collector resource without deleting it', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const collector = fixture.write('app/dist/macos/AgentObservatory.app/Contents/Resources/Collector', 'keep');
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  assert.throws((): void => customizer.removeLegacyCollector(), /real directory/);
  assert.equal(fs.readFileSync(collector, 'utf8'), 'keep');
});

test('Icon packaging preserves dimensions, plist keys, local favicon and signing order', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  fixture.icon();
  new IconPackager(fixture.root, fixture.run).package();
  const images = fixture.commands.filter((command: string[]): boolean => command[0] === '/usr/bin/sips');
  assert.equal(images.length, 10);
  assert.ok(images.some((command: string[]): boolean => command.includes(path.join(fixture.artifacts, 'icons/Observatory.iconset/icon_512x512@2x.png'))));
  assert.ok(images.some((command: string[]): boolean => command.includes(path.join(fixture.artifacts, 'icons/icon-1024.png'))));
  assert.deepEqual(fixture.commands.at(-1), ['/usr/bin/codesign', '--force', '--sign', '-', path.join(fixture.artifacts, 'macos/AgentObservatory.app')]);
  const plist = path.join(fixture.artifacts, 'macos/AgentObservatory.app/Contents/Info.plist');
  const icon = spawnSync('/usr/bin/plutil', ['-extract', 'CFBundleIconFile', 'raw', '-o', '-', plist], {encoding: 'utf8'});
  const identifier = spawnSync('/usr/bin/plutil', ['-extract', 'CFBundleIdentifier', 'raw', '-o', '-', plist], {encoding: 'utf8'});
  assert.equal(icon.status, 0);
  assert.equal(icon.stdout.trim(), 'Observatory.icns');
  assert.equal(identifier.stdout.trim(), 'local.fixture.monitor');
  assert.ok(fs.readFileSync(path.join(fixture.artifacts, 'macos/AgentObservatory.app/Contents/Resources/WebBundle/index.html'), 'utf8').includes('/shared/assets/observatory.svg'));
});

test('Favicon replacement is idempotent', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const file = fixture.write('index.html', '<head></head>');
  const packager = new IconPackager(fixture.root, fixture.run);
  packager.updateFavicon(file);
  const first = fs.readFileSync(file, 'utf8');
  packager.updateFavicon(file);
  assert.equal(fs.readFileSync(file, 'utf8'), first);
});

test('All tool imports have no production build side effects', (): void => {
  assert.equal(typeof NativeRustPackager, 'function');
  assert.equal(typeof NativeHostCustomizer, 'function');
  assert.equal(typeof IconPackager, 'function');
});

test('Command runner fails on unsuccessful child process', (): void => {
  assert.throws((): void => ToolCommandUtils.run([process.execPath, '-e', 'process.exit(7)'], true), /failed/);
});

test('Real CLI selects explicit fixture root and keeps fresh-companion guard', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source: string = fixture.write('app/dist/macos/project/companion.rs', '\npub mod services {\n');
  const tool: string = path.resolve('tools/NativeRustPackager.ts');
  const result = spawnSync(process.execPath, ['--experimental-strip-types', tool, '--root', fixture.root], { encoding: 'utf8' });
  assert.equal(result.status, 1);
  assert.ok(result.stderr.includes('fresh Tachyon'));
  assert.equal(fs.readFileSync(source, 'utf8'), '\npub mod services {\n');
});

test('Real CLI rejects missing root argument without building', (): void => {
  const tool: string = path.resolve('tools/NativeHostCustomizer.ts');
  const result = spawnSync(process.execPath, ['--experimental-strip-types', tool, '--root'], { encoding: 'utf8' });
  assert.equal(result.status, 1);
  assert.ok(result.stderr.includes('--root requires a project path'));
});

test('Packagers reject missing explicit output configuration before creating caches', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  delete process.env.OBSERVATORY_NATIVE_ARTIFACTS;
  assert.throws((): NativeRustPackager => new NativeRustPackager(fixture.root, fixture.run), /explicitly supplied/);
  assert.equal(fixture.commands.length, 0);
  assert.equal(fs.existsSync(fixture.rustCache), false);
});

test('Packagers reject source output before creating caches or running native tools', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  process.env.OBSERVATORY_NATIVE_ARTIFACTS = path.join(fixture.root, 'app/dist');
  assert.throws((): NativeHostCustomizer => new NativeHostCustomizer(fixture.root, fixture.run), /outside the source project/);
  assert.equal(fixture.commands.length, 0);
  assert.equal(fs.existsSync(path.join(fixture.root, 'app/dist')), false);
});

test('Packagers reject an external-looking symlink into source before writing', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const link: string = path.join(fixture.artifacts, 'source-link');
  fs.symlinkSync(fixture.root, link);
  process.env.OBSERVATORY_NATIVE_ARTIFACTS = path.join(link, 'generated');
  assert.throws((): IconPackager => new IconPackager(fixture.root, fixture.run), /outside the source project/);
  assert.equal(fixture.commands.length, 0);
  assert.equal(fs.existsSync(path.join(fixture.root, 'generated')), false);
});

test('Cargo artifacts honor a configured absolute external target', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const target: string = path.join(fixture.artifacts, 'existing-target');
  process.env.CARGO_TARGET_DIR = target;
  assert.equal(ToolPathUtils.cargoTarget(fixture.root, fixture.rustCache), target);
});

test('Cargo artifacts honor a configured relative external target', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const target: string = path.join(fixture.artifacts, 'relative-target');
  process.env.CARGO_TARGET_DIR = path.relative(fixture.root, target);
  assert.equal(ToolPathUtils.cargoTarget(fixture.root, fixture.rustCache), target);
});

test('Cargo artifacts reject a relative source target', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  process.env.CARGO_TARGET_DIR = 'target';
  assert.throws((): string => ToolPathUtils.cargoTarget(fixture.root, fixture.rustCache), /outside the source project/);
  assert.equal(fs.existsSync(path.join(fixture.root, 'target')), false);
});


test('Unset Cargo target uses the explicit external cache without creating output', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  delete process.env.CARGO_TARGET_DIR;
  const target: string = path.join(fixture.rustCache, 'target');
  assert.equal(ToolPathUtils.cargoTarget(fixture.root, fixture.rustCache), target);
  assert.equal(fs.existsSync(target), false);
});

test('An explicitly blank Cargo target is rejected instead of selecting a fallback', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  process.env.CARGO_TARGET_DIR = ' \t';
  assert.throws((): string => ToolPathUtils.cargoTarget(fixture.root, fixture.rustCache), /CARGO_TARGET_DIR requires an explicitly supplied value/);
  assert.equal(fs.existsSync(fixture.rustCache), false);
});
