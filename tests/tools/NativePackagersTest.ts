import test, { type TestContext } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { deflateSync } from 'node:zlib';
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
  assert.ok(fs.readFileSync(path.join(fixture.rustCache, 'emitter.rs'), 'utf8').includes('use observatory_native::{models, ProcessIdentityReader, ClaudeHookAdapter};\nuse observatory_native::services::{LogicalEventClientService, LogicalEventWireService, LogicalSocketService, NativePlatformCompositionService};'));
  assert.equal(fs.readFileSync(path.join(fixture.artifacts, 'macos/project/libtachyoncompanion.a'), 'utf8'), 'fixture static library');
  const emitter = path.join(fixture.artifacts, 'macos/Swarx.app/Contents/MacOS/observatory-event');
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

test('Rust manifest adds windows-sys only for Windows builds and keeps macOS bytes unchanged', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const macos: string = new NativeRustPackager(fixture.root, fixture.run, 'darwin').manifest();
  const windows: string = new NativeRustPackager(fixture.root, fixture.run, 'win32').manifest();
  assert.equal(macos.includes('windows-sys'), false);
  assert.ok(macos.endsWith('regex = "=1.13.1"\n'));
  assert.ok(windows.startsWith(macos));
  assert.equal(windows.slice(macos.length).split('\n')[0], "[target.'cfg(windows)'.dependencies]");
  assert.ok(windows.includes('windows-sys = { version = "=0.61.2", features = ["Win32_Foundation", "Win32_Security", "Win32_Security_Authorization", "Win32_Security_Cryptography", "Win32_Storage_FileSystem", "Win32_System_Diagnostics_ToolHelp", "Win32_System_IO", "Win32_System_Pipes", "Win32_System_ProcessStatus", "Win32_System_Threading", "Win32_System_Time"] }'));
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
  const collector = fixture.write('app/dist/macos/Swarx.app/Contents/Resources/Collector/old.txt', 'legacy fixture');
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
  const collector = fixture.write('app/dist/macos/Swarx.app/Contents/Resources/Collector', 'keep');
  const customizer = new NativeHostCustomizer(fixture.root, fixture.run);
  assert.throws((): void => customizer.removeLegacyCollector(), /real directory/);
  assert.equal(fs.readFileSync(collector, 'utf8'), 'keep');
});

test('Icon packaging preserves dimensions, plist keys, local favicon and signing order', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  fixture.icon();
  new IconPackager(fixture.root, fixture.run).package();
  assert.deepEqual(fixture.commands.map((command: string[]): string | undefined => command[0]),
    ['/usr/bin/swiftc', path.join(fixture.swiftCache, 'native-icon-generator'), '/usr/bin/iconutil',
      '/usr/bin/plutil', '/usr/bin/plutil', '/usr/bin/plutil', '/usr/bin/codesign']);
  const decode: string[] = fixture.commands[2]!;
  assert.deepEqual(decode.slice(0, 3), ['/usr/bin/iconutil', '-c', 'iconset']);
  assert.equal(decode[4], '-o');
  assert.equal(path.basename(decode[5]!), 'decoded.iconset');
  assert.deepEqual(fs.readFileSync(path.join(fixture.artifacts, 'icons/Observatory.icns')),
    fs.readFileSync(path.join(fixture.artifacts, 'macos/Swarx.app/Contents/Resources/Observatory.icns')));
  const iconset: string = path.join(fixture.artifacts, 'icons/Observatory.iconset');
  const expected: readonly (readonly [string, number])[] = [
    ['icon_16x16.png', 16], ['icon_16x16@2x.png', 32],
    ['icon_32x32.png', 32], ['icon_32x32@2x.png', 64],
    ['icon_128x128.png', 128], ['icon_128x128@2x.png', 256],
    ['icon_256x256.png', 256], ['icon_256x256@2x.png', 512],
    ['icon_512x512.png', 512], ['icon_512x512@2x.png', 1024],
  ];
  assert.deepEqual(fs.readdirSync(iconset).sort(), expected.map(([name]): string => name).sort());
  for (const [name, pixels] of expected) {
    const original: Buffer = fs.readFileSync(path.join(fixture.artifacts, `icons/icon-${pixels}.png`));
    const packaged: Buffer = fs.readFileSync(path.join(iconset, name));
    assert.deepEqual(original, fixture.png(pixels), 'Generated artwork must remain unchanged.');
    assert.deepEqual(packaged, original, 'Iconset entry must preserve exact generated PNG bytes.');
    assert.equal(packaged.readUInt32BE(16), pixels);
    assert.equal(packaged.readUInt32BE(20), pixels);
  }
  assert.deepEqual(fixture.commands.at(-1), ['/usr/bin/codesign', '--force', '--sign', '-', path.join(fixture.artifacts, 'macos/Swarx.app')]);
  const plist = path.join(fixture.artifacts, 'macos/Swarx.app/Contents/Info.plist');
  const icon = spawnSync('/usr/bin/plutil', ['-extract', 'CFBundleIconFile', 'raw', '-o', '-', plist], {encoding: 'utf8'});
  const identifier = spawnSync('/usr/bin/plutil', ['-extract', 'CFBundleIdentifier', 'raw', '-o', '-', plist], {encoding: 'utf8'});
  assert.equal(icon.status, 0);
  assert.equal(icon.stdout.trim(), 'Observatory.icns');
  assert.equal(identifier.stdout.trim(), 'local.fixture.monitor');
  assert.ok(fs.readFileSync(path.join(fixture.artifacts, 'macos/Swarx.app/Contents/Resources/WebBundle/index.html'), 'utf8').includes('/shared/assets/observatory.svg'));
});

const invalidIconInputs: readonly (readonly [string, (fixture: NativePackagerFixture) => Buffer | undefined])[] = [
  ['missing PNG', (): undefined => undefined],
  ['truncated PNG header', (fixture): Buffer => fixture.png(16).subarray(0, 32)],
  ['non-PNG signature', (): Buffer => Buffer.alloc(33)],
  ['wrong first chunk length', (fixture): Buffer => {
    const bytes: Buffer = fixture.png(16); bytes.writeUInt32BE(12, 8); return bytes;
  }],
  ['wrong first chunk type', (fixture): Buffer => {
    const bytes: Buffer = fixture.png(16); bytes.write('IDAT', 12, 'ascii'); return bytes;
  }],
  ['non-ASCII first chunk type', (fixture): Buffer => {
    const bytes: Buffer = fixture.png(16); Buffer.from([0xc9, 0xc8, 0xc4, 0xd2]).copy(bytes, 12); return bytes;
  }],
  ['wrong PNG width', (fixture): Buffer => {
    const bytes: Buffer = fixture.png(16); bytes.writeUInt32BE(32, 16); return bytes;
  }],
  ['wrong PNG height', (fixture): Buffer => {
    const bytes: Buffer = fixture.png(16); bytes.writeUInt32BE(32, 20); return bytes;
  }],
];

for (const [name, contents] of invalidIconInputs) {
  test(`Icon packaging refuses ${name} without output or input changes`, (context: TestContext): void => {
    const fixture: NativePackagerFixture = new NativePackagerFixture();
    context.after((): void => fixture.close());
    const iconset: string = path.join(fixture.artifacts, 'icons/Observatory.iconset');
    fs.mkdirSync(iconset, { recursive: true });
    const input: string = path.join(fixture.artifacts, 'icons/icon-16.png');
    const output: string = path.join(iconset, 'icon_16x16.png');
    const bytes: Buffer | undefined = contents(fixture);
    if (bytes !== undefined) fixture.write('app/icons/icon-16.png', bytes);
    const packager: IconPackager = new IconPackager(fixture.root, fixture.run);
    assert.throws((): void => packager.packageImage(16, 1), /PNG|IHDR|dimensions|ENOENT/);
    assert.equal(fs.existsSync(output), false);
    assert.equal(fixture.commands.length, 0, 'Rejected images must not invoke any tool.');
    if (bytes === undefined) assert.equal(fs.existsSync(input), false);
    else assert.deepEqual(fs.readFileSync(input), bytes);
    const previous: Buffer = fixture.png(32);
    fs.writeFileSync(output, previous);
    assert.throws((): void => packager.packageImage(16, 1), /PNG|IHDR|dimensions|ENOENT/);
    assert.deepEqual(fs.readFileSync(output), previous, 'Reject before touching an existing output.');
    assert.equal(fixture.commands.length, 0);
  });
}

function icnsEntries(bytes: Buffer): Map<string, Buffer> {
  assert.equal(bytes.toString('ascii', 0, 4), 'icns');
  assert.equal(bytes.readUInt32BE(4), bytes.length);
  const types: string[] = ['is32', 's8mk', 'ic11', 'il32', 'l8mk', 'ic12', 'ic07', 'ic13', 'ic08', 'ic14', 'ic09', 'ic10'];
  assert.equal(bytes.toString('ascii', 8, 12), 'TOC ');
  assert.equal(bytes.readUInt32BE(12), 8 + types.length * 8);
  let offset: number = 16 + types.length * 8;
  const result = new Map<string, Buffer>();
  for (const [index, type] of types.entries()) {
    const length: number = bytes.readUInt32BE(offset + 4);
    assert.ok(length >= 8 && offset + length <= bytes.length);
    assert.equal(bytes.toString('ascii', offset, offset + 4), type);
    assert.deepEqual(bytes.subarray(16 + index * 8, 24 + index * 8), bytes.subarray(offset, offset + 8));
    result.set(type, bytes.subarray(offset + 8, offset + length));
    offset += length;
  }
  assert.equal(offset, bytes.length);
  return result;
}

function legacyPixels(rgb: Buffer, alpha: Buffer): Buffer {
  const channels: number[][] = [[], [], []];
  let offset: number = 0;
  for (const channel of channels) {
    while (channel.length < alpha.length) {
      const prefix: number = rgb[offset++]!;
      assert.ok(prefix <= 127, 'Assembler must use literal RLE packets.');
      const count: number = prefix + 1;
      assert.ok(channel.length + count <= alpha.length && offset + count <= rgb.length);
      channel.push(...rgb.subarray(offset, offset + count));
      offset += count;
    }
  }
  assert.equal(offset, rgb.length);
  const rgba: Buffer = Buffer.alloc(alpha.length * 4);
  for (let index: number = 0; index < alpha.length; index++) {
    for (let channel: number = 0; channel < 3; channel++) rgba[index * 4 + channel] = channels[channel]![index]!;
    rgba[index * 4 + 3] = alpha[index]!;
  }
  return rgba;
}

for (const filter of [0, 1, 2, 3, 4]) {
  test(`ICNS TOC, original PNG payloads and legacy RGB/alpha preserve filter ${filter} pixels`, (context: TestContext): void => {
    const fixture = new NativePackagerFixture();
    context.after((): void => fixture.close());
    fixture.icon();
    const run = (command: readonly string[], quiet?: boolean): void => {
      fixture.run(command, quiet);
      if (command[0]?.endsWith('/native-icon-generator')) {
        for (const pixels of [16, 32]) fixture.write(`app/icons/icon-${pixels}.png`, fixture.png(pixels, filter));
      }
    };
    const packager = new IconPackager(fixture.root, run);
    packager.package();
    const file: string = path.join(fixture.artifacts, 'icons/Observatory.icns');
    const first: Buffer = fs.readFileSync(file);
    const entries: Map<string, Buffer> = icnsEntries(first);
    for (const [rgb, mask, pixels] of [['is32', 's8mk', 16], ['il32', 'l8mk', 32]] as const) {
      assert.equal(entries.get(mask)!.length, pixels * pixels);
      assert.deepEqual(legacyPixels(entries.get(rgb)!, entries.get(mask)!), fixture.rgba(pixels));
    }
    for (const [type, pixels] of [['ic11', 32], ['ic12', 64], ['ic07', 128], ['ic13', 256],
      ['ic08', 256], ['ic14', 512], ['ic09', 512], ['ic10', 1024]] as const) {
      assert.deepEqual(entries.get(type), fs.readFileSync(path.join(fixture.artifacts, `icons/icon-${pixels}.png`)));
    }
    packager.package();
    assert.deepEqual(fs.readFileSync(file), first, 'Assembly is deterministic across fresh decoder directories.');
    const decodes: string[][] = fixture.commands.filter((command): boolean => command[0] === '/usr/bin/iconutil');
    assert.equal(decodes.length, 2);
    assert.ok(decodes.every((command): boolean => command[2] === 'iconset'));
    assert.notEqual(decodes[0]![5], decodes[1]![5]);
  });
}

function pngWithData(fixture: NativePackagerFixture, compressed: Buffer): Buffer {
  const valid: Buffer = fixture.png(16);
  return Buffer.concat([valid.subarray(0, 33), fixture.pngChunk('IDAT', compressed), fixture.pngChunk('IEND', Buffer.alloc(0))]);
}

const invalidAssemblyInputs: readonly (readonly [string, (fixture: NativePackagerFixture) => Buffer])[] = [
  ['bad chunk CRC', (f): Buffer => { const b = f.png(16); b[32] = b[32]! ^ 1; return b; }],
  ['truncated chunk', (f): Buffer => f.png(16).subarray(0, -1)],
  ['oversized chunk length', (f): Buffer => { const b = f.png(16); b.writeUInt32BE(0xffffffff, 33); return b; }],
  ['duplicate IHDR', (f): Buffer => { const b = f.png(16); return Buffer.concat([b.subarray(0, 33), b.subarray(8)]); }],
  ['missing IEND', (f): Buffer => f.png(16).subarray(0, -12)],
  ['trailing PNG bytes', (f): Buffer => Buffer.concat([f.png(16), Buffer.from([0])])],
  ['missing IDAT', (f): Buffer => Buffer.concat([f.png(16).subarray(0, 33), f.pngChunk('IEND', Buffer.alloc(0))])],
  ['empty IDAT', (f): Buffer => pngWithData(f, Buffer.alloc(0))],
  ['unknown critical chunk', (f): Buffer => { const b = f.png(16); return Buffer.concat([b.subarray(0, 33), f.pngChunk('ABCD', Buffer.alloc(0)), b.subarray(33)]); }],
  ['noncontiguous IDAT', (f): Buffer => { const b = f.png(16); const n = b.readUInt32BE(33); const data = b.subarray(41, 41 + n); return Buffer.concat([b.subarray(0, 33), f.pngChunk('IDAT', data.subarray(0, 2)), f.pngChunk('tEXt', Buffer.alloc(0)), f.pngChunk('IDAT', data.subarray(2)), b.subarray(-12)]); }],
  ['invalid reserved chunk letter', (f): Buffer => { const b = f.png(16); return Buffer.concat([b.subarray(0, 33), f.pngChunk('abca', Buffer.alloc(0)), b.subarray(33)]); }],
  ['invalid IDAT compression', (f): Buffer => pngWithData(f, Buffer.from([0, 1, 2]))],
  ['short inflated length', (f): Buffer => pngWithData(f, deflateSync(Buffer.alloc(16 * 65 - 1)))],
  ['long inflated length', (f): Buffer => pngWithData(f, deflateSync(Buffer.alloc(16 * 65 + 1)))],
  ['inflation bomb', (f): Buffer => pngWithData(f, deflateSync(Buffer.alloc(1024 * 1024)))],
  ['trailing compressed bytes', (f): Buffer => pngWithData(f, Buffer.concat([deflateSync(Buffer.alloc(16 * 65)), Buffer.from([1, 2, 3])]))],
  ['unsupported scanline filter', (f): Buffer => f.png(16, 5)],
  ...([['bit depth', 8, 16], ['color type', 9, 2], ['compression', 10, 1], ['filter method', 11, 1], ['interlace', 12, 1]] as const)
    .map(([name, offset, value]): readonly [string, (fixture: NativePackagerFixture) => Buffer] =>
      [`unsupported ${name}`, (f): Buffer => { const b = f.png(16); const header = Buffer.from(b.subarray(16, 29)); header[offset] = value; return Buffer.concat([b.subarray(0, 8), f.pngChunk('IHDR', header), b.subarray(33)]); }]),
];

for (const [name, corrupt] of invalidAssemblyInputs) {
  test(`ICNS assembly refuses ${name} and preserves previous valid ICNS`, (context: TestContext): void => {
    const fixture = new NativePackagerFixture();
    context.after((): void => fixture.close());
    fixture.icon();
    new IconPackager(fixture.root, fixture.run).package();
    const file: string = path.join(fixture.artifacts, 'icons/Observatory.icns');
    const delivered: string = path.join(fixture.artifacts, 'macos/Swarx.app/Contents/Resources/Observatory.icns');
    const previous: Buffer = fs.readFileSync(file);
    const previousDelivered: Buffer = fs.readFileSync(delivered);
    const bad: Buffer = corrupt(fixture);
    fixture.commands.length = 0;
    const run = (command: readonly string[], quiet?: boolean): void => {
      fixture.run(command, quiet);
      if (command[0]?.endsWith('/native-icon-generator')) fixture.write('app/icons/icon-16.png', bad);
    };
    assert.throws((): void => new IconPackager(fixture.root, run).package());
    assert.deepEqual(fs.readFileSync(file), previous);
    assert.deepEqual(fs.readFileSync(delivered), previousDelivered);
    assert.deepEqual(fs.readFileSync(path.join(fixture.artifacts, 'icons/icon-16.png')), bad);
    assert.equal(fixture.commands.some((command): boolean => command[0] === '/usr/bin/iconutil' || command[0] === '/usr/bin/codesign'), false);
  });
}

test('ICNS assembly validates large PNG CRCs before replacing the previous icon', (context: TestContext): void => {
  const fixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  fixture.icon();
  new IconPackager(fixture.root, fixture.run).package();
  const file: string = path.join(fixture.artifacts, 'icons/Observatory.icns');
  const previous: Buffer = fs.readFileSync(file);
  const bad: Buffer = fixture.png(1024);
  bad[bad.length - 1] = bad[bad.length - 1]! ^ 1;
  fixture.commands.length = 0;
  const run = (command: readonly string[], quiet?: boolean): void => {
    fixture.run(command, quiet);
    if (command[0]?.endsWith('/native-icon-generator')) fixture.write('app/icons/icon-1024.png', bad);
  };
  assert.throws((): void => new IconPackager(fixture.root, run).package(), /CRC/);
  assert.deepEqual(fs.readFileSync(file), previous);
  assert.deepEqual(fs.readFileSync(path.join(fixture.artifacts, 'icons/icon-1024.png')), bad);
  assert.equal(fixture.commands.some((command): boolean => command[0] === '/usr/bin/iconutil' || command[0] === '/usr/bin/codesign'), false);
});

for (const mode of ['decoder failure', 'missing image', 'extra image', 'wrong dimensions', 'symlink image', 'bad decoded CRC', 'changed decoded pixels', 'changed original', 'changed candidate']) {
  test(`ICNS mandatory readback rejects ${mode} before final replacement/signing`, (context: TestContext): void => {
    const fixture = new NativePackagerFixture();
    context.after((): void => fixture.close());
    fixture.icon();
    new IconPackager(fixture.root, fixture.run).package();
    const file: string = path.join(fixture.artifacts, 'icons/Observatory.icns');
    const previous: Buffer = fs.readFileSync(file);
    fixture.commands.length = 0;
    const run = (command: readonly string[], quiet?: boolean): void => {
      fixture.run(command, quiet);
      if (command[0] !== '/usr/bin/iconutil' || command[2] !== 'iconset') return;
      const directory: string = command[5]!;
      const png: string = path.join(directory, 'icon_16x16.png');
      if (mode === 'decoder failure') throw new Error('Fixture decoder failure');
      if (mode === 'missing image') fs.unlinkSync(png);
      if (mode === 'extra image') fs.writeFileSync(path.join(directory, 'extra.png'), fixture.png(16));
      if (mode === 'wrong dimensions') fs.writeFileSync(png, fixture.png(32));
      if (mode === 'symlink image') { fs.unlinkSync(png); fs.symlinkSync(path.join(fixture.artifacts, 'icons/icon-16.png'), png); }
      if (mode === 'bad decoded CRC') { const bad = fixture.png(16); bad[32] = bad[32]! ^ 1; fs.writeFileSync(png, bad); }
      if (mode === 'changed decoded pixels') fs.writeFileSync(png, pngWithData(fixture, deflateSync(Buffer.alloc(16 * 65))));
      if (mode === 'changed original') fs.writeFileSync(path.join(fixture.artifacts, 'icons/icon-16.png'), fixture.png(32));
      if (mode === 'changed candidate') fs.appendFileSync(command[3]!, Buffer.from([0]));
    };
    assert.throws((): void => new IconPackager(fixture.root, run).package());
    assert.deepEqual(fs.readFileSync(file), previous);
    assert.equal(fixture.commands.some((command): boolean => command[0] === '/usr/bin/codesign'), false);
  });
}

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

for (const target of ['macos', 'linux', 'windows']) {
  test(`Production build routes ${target} through one framework native build`, (context: TestContext): void => {
    const fixture: NativePackagerFixture = new NativePackagerFixture();
    context.after((): void => fixture.close());
    const result = fixture.buildScript(target);
    assert.equal(result.status, 0, result.stderr);
    const directory: string = path.join(fixture.artifacts, 'commands');
    assert.deepEqual(fs.readFileSync(path.join(directory, 'compiler.txt'), 'utf8').trim().split('\n'),
      ['bundle', 'app', '--out-dir', fixture.artifacts, '--target', target]);
    const post: string = path.join(directory, 'post.txt');
    const commands: string[] = fs.existsSync(post) ? fs.readFileSync(post, 'utf8').trim().split('\n') : [];
    assert.deepEqual(commands, target === 'macos' ? ['tools/NativeHostCustomizer.ts', 'tools/IconPackager.ts'] : []);
  });
}

test('Production build rejects an unsupported target before invoking native tools', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const result = fixture.buildScript('web');
  assert.notEqual(result.status, 0);
  assert.equal(fs.existsSync(path.join(fixture.artifacts, 'commands/compiler.txt')), false);
});

test('Legacy QA packager refuses an already framework-exported Cargo companion', (context: TestContext): void => {
  const fixture: NativePackagerFixture = new NativePackagerFixture();
  context.after((): void => fixture.close());
  const source: string = fixture.write('app/dist/macos/project/companion.rs', 'pub use crate::TachyonRustServices::*;\n');
  const packager: NativeRustPackager = new NativeRustPackager(fixture.root, fixture.run);
  assert.throws((): void => packager.build(), /fresh Tachyon/);
  assert.equal(fs.readFileSync(source, 'utf8'), 'pub use crate::TachyonRustServices::*;\n');
  assert.equal(fixture.commands.length, 0);
});
