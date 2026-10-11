import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { crc32, deflateSync } from 'node:zlib';

export class NativePackagerFixture {
  private static readonly pngCache = new Map<string, Buffer>();
  readonly root: string;
  readonly artifacts: string;
  readonly rustCache: string;
  readonly swiftCache: string;
  private readonly previous: Record<string, string | undefined>;
  readonly commands: string[][] = [];
  readonly quiet: boolean[] = [];

  constructor() {
    this.root = fs.mkdtempSync(path.join(os.tmpdir(), 'observatory-packager source-'));
    this.artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'observatory-packager output-'));
    this.rustCache = path.join(this.artifacts, 'rust-cache');
    this.swiftCache = path.join(this.artifacts, 'swift-cache');
    this.previous = {CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR, OBSERVATORY_NATIVE_ARTIFACTS: process.env.OBSERVATORY_NATIVE_ARTIFACTS, OBSERVATORY_RUST_CACHE_PATH: process.env.OBSERVATORY_RUST_CACHE_PATH, OBSERVATORY_SWIFT_CACHE_PATH: process.env.OBSERVATORY_SWIFT_CACHE_PATH};
    process.env.OBSERVATORY_NATIVE_ARTIFACTS = this.artifacts;
    process.env.OBSERVATORY_RUST_CACHE_PATH = this.rustCache;
    process.env.CARGO_TARGET_DIR = path.join(this.rustCache, 'target');
    process.env.OBSERVATORY_SWIFT_CACHE_PATH = this.swiftCache;
  }

  write(relative: string, contents: string | Uint8Array): string {
    const file: string = this.destination(relative);
    this.writeAbsolute(file, contents);
    return file;
  }

  rust(): string {
    return this.write('app/dist/macos/project/companion.rs', 'pub mod TachyonRustServices {}\n');
  }

  buildScript(target: string): SpawnSyncReturns<string> {
    const directory: string = path.join(this.artifacts, 'commands');
    fs.mkdirSync(directory);
    const script: string = this.write('build.sh', fs.readFileSync(path.resolve('build.sh'), 'utf8'));
    this.write('tools/utils/ToolPathUtils.ts', fs.readFileSync(path.resolve('tools/utils/ToolPathUtils.ts'), 'utf8'));
    const compiler: string = path.join(directory, 'ty');
    this.writeAbsolute(compiler, '#!/bin/sh\nprintf "%s\\n" "$@" > "$OBSERVATORY_TEST_COMMANDS/compiler.txt"\n');
    this.writeAbsolute(path.join(directory, 'tsc'), '#!/bin/sh\nprintf "Version 7.0.2\\n"\n');
    this.writeAbsolute(path.join(directory, 'node'), '#!/bin/sh\ncase "$2" in tools/*.ts) printf "%s\\n" "$2" >> "$OBSERVATORY_TEST_COMMANDS/post.txt" ;; *) exec "$OBSERVATORY_TEST_NODE" "$@" ;; esac\n');
    for (const name of ['ty', 'tsc', 'node']) fs.chmodSync(path.join(directory, name), 0o755);
    return spawnSync('/bin/zsh', [script, '--target', target], {encoding: 'utf8', env: {
      ...process.env, PATH: directory + path.delimiter + process.env.PATH,
      TACHYON_COMPILER: compiler, OBSERVATORY_SWIFT_CACHE_PATH: this.artifacts + '-swift', OBSERVATORY_RUST_CACHE_PATH: this.artifacts + '-rust', OBSERVATORY_ROLLBACK_ROOT: this.artifacts + '-rollback',
      OBSERVATORY_TEST_COMMANDS: directory, OBSERVATORY_TEST_NODE: process.execPath,
    }});
  }

  host(): string {
    const source: string = 'window.contentMinSize = NSSize(width: 0, height: 0)\n'
      + '        window.title = "Swarx"\n'
      + '        menus.install(NSApplication.shared, window, "Swarx", menuNavigation, menuActions)\n'
      + '        NSApplication.shared.activate(ignoringOtherApps: true)\n'
      + '        Telemetry.shared.record("controller.active")\n';
    this.write('tools/native/NativePreviewVerifier.swift', 'let script = "__NATIVE_REPORT_SCRIPT__"\n');
    this.write('tools/native/NativePreviewReport.js', 'const note = "$& $$";\n');
    this.write('app/client/shared/native/NativeApplicationIcon.swift', 'final class NativeApplicationIcon {}\n');
    return this.write('app/dist/macos/project/TachyonHost.swift', source);
  }

  icon(): void {
    this.write('app/dist/macos/Swarx.app/Contents/Info.plist', '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>local.fixture.monitor</string></dict></plist>');
    this.write('app/dist/macos/Swarx.app/Contents/Resources/WebBundle/index.html', '<html><head></head><body>Fixture</body></html>');
  }

  png(pixels: number, filter: number = 0): Buffer {
    const key: string = `${pixels}/${filter}`;
    const cached: Buffer | undefined = NativePackagerFixture.pngCache.get(key);
    if (cached !== undefined) return Buffer.from(cached);
    const header: Buffer = Buffer.alloc(13);
    header.writeUInt32BE(pixels, 0);
    header.writeUInt32BE(pixels, 4);
    header[8] = 8;
    header[9] = 6;
    const rgba: Buffer = this.rgba(pixels);
    const stride: number = pixels * 4;
    const scanlines: Buffer = Buffer.alloc(pixels * (stride + 1));
    for (let y: number = 0; y < pixels; y++) {
      scanlines[y * (stride + 1)] = filter;
      for (let x: number = 0; x < stride; x++) {
        const index: number = y * stride + x;
        const left: number = x >= 4 ? rgba[index - 4]! : 0;
        const above: number = y > 0 ? rgba[index - stride]! : 0;
        const diagonal: number = y > 0 && x >= 4 ? rgba[index - stride - 4]! : 0;
        const p: number = left + above - diagonal;
        const distances: number[] = [Math.abs(p - left), Math.abs(p - above), Math.abs(p - diagonal)];
        const paeth: number = [left, above, diagonal][distances.indexOf(Math.min(...distances))]!;
        const predictions: number[] = [0, left, above, Math.floor((left + above) / 2), paeth];
        scanlines[y * (stride + 1) + 1 + x] = (rgba[index]! - (predictions[filter] ?? 0)) & 255;
      }
    }
    const png: Buffer = Buffer.concat([Buffer.from('89504e470d0a1a0a', 'hex'),
      this.pngChunk('IHDR', header), this.pngChunk('IDAT', deflateSync(scanlines)), this.pngChunk('IEND', Buffer.alloc(0))]);
    NativePackagerFixture.pngCache.set(key, png);
    return Buffer.from(png);
  }

  rgba(pixels: number): Buffer {
    const result: Buffer = Buffer.alloc(pixels * pixels * 4);
    for (let y: number = 0; y < pixels; y++) {
      for (let x: number = 0; x < pixels; x++) {
        const index: number = (y * pixels + x) * 4;
        result[index] = (x * 13 + y * 3) & 255;
        result[index + 1] = (x * 7 + y * 11) & 255;
        result[index + 2] = (x + y * 17) & 255;
        result[index + 3] = (x * 19 + y * 23) & 255;
      }
    }
    return result;
  }

  pngChunk(type: string, payload: Buffer): Buffer {
    const result: Buffer = Buffer.alloc(payload.length + 12);
    result.writeUInt32BE(payload.length, 0);
    result.write(type, 4, 'ascii');
    payload.copy(result, 8);
    result.writeUInt32BE(crc32(result.subarray(4, result.length - 4)), result.length - 4);
    return result;
  }

  run: (command: readonly string[], quiet?: boolean) => void = (command: readonly string[], quiet: boolean = false): void => {
    this.commands.push([...command]);
    this.quiet.push(quiet);
    if (command[0] === 'cargo') {
      this.write('tools/rust-cache/target/debug/libobservatory_native.a', 'fixture static library');
      this.write('tools/rust-cache/target/debug/observatory-event', 'fixture emitter');
    } else if (command[0] === '/usr/bin/swiftc') {
      this.writeAbsolute(this.output(command, '-o'), 'fixture executable');
    } else if (command[0]?.endsWith('/native-icon-generator')) {
      for (const pixels of [16, 32, 64, 128, 256, 512, 1024]) this.write(`app/icons/icon-${pixels}.png`, this.png(pixels));
    } else if (command[0] === '/usr/bin/sips') {
      this.writeAbsolute(this.output(command, '--out'), 'fixture resized png');
    } else if (command[0] === '/usr/bin/iconutil') {
      if (command[2] === 'iconset') this.decodeIcon(command[3]!, this.output(command, '-o'));
      else this.writeAbsolute(this.output(command, '-o'), 'icnsfixture');
    } else if (command[0] === '/usr/bin/plutil') {
      const result = spawnSync(command[0], command.slice(1), { encoding: 'utf8' });
      if (result.error) throw result.error;
      if (result.status !== 0) throw new Error(result.stderr);
    }
  };

  close(): void {
    fs.rmSync(this.root, { recursive: true });
    fs.rmSync(this.artifacts, { recursive: true });
    this.restoreEnvironment();
  }

  private decodeIcon(source: string, output: string): void {
    const bytes: Buffer = fs.readFileSync(source);
    if (bytes.toString('ascii', 0, 4) !== 'icns' || bytes.readUInt32BE(4) !== bytes.length) {
      throw new Error('Fixture decoder refuses invalid ICNS framing.');
    }
    fs.mkdirSync(output);
    // Platform pixel fidelity is checked by real Apple decode in the separate admitted packaging lane.
    const originals: string = path.join(this.artifacts, 'icons/Observatory.iconset');
    for (const name of fs.readdirSync(originals)) this.writeAbsolute(path.join(output, name), fs.readFileSync(path.join(originals, name)));
  }

  private destination(relative: string): string {
    if (relative.startsWith('app/dist/')) return path.join(this.artifacts, relative.slice('app/dist/'.length));
    return this.cacheDestination(relative);
  }

  private cacheDestination(relative: string): string {
    if (relative.startsWith('tools/')) return this.toolDestination(relative);
    return this.iconDestination(relative);
  }

  private toolDestination(relative: string): string {
    if (relative.startsWith('tools/rust-cache/')) return path.join(this.rustCache, relative.slice('tools/rust-cache/'.length));
    return path.join(this.root, relative);
  }

  private iconDestination(relative: string): string {
    if (relative.startsWith('app/icons/')) return path.join(this.artifacts, 'icons', relative.slice('app/icons/'.length));
    return path.join(this.root, relative);
  }

  private restoreEnvironment(): void {
    for (const name of Object.keys(this.previous)) this.restoreValue(name);
  }

  private restoreValue(name: string): void {
    const value: string | undefined = this.previous[name];
    if (value === undefined) { delete process.env[name]; return; }
    process.env[name] = value;
  }

  private output(command: readonly string[], flag: string): string {
    const file: string | undefined = command[command.indexOf(flag) + 1];
    if (!file) throw new Error('Fixture command output is missing.');
    return file;
  }

  private writeAbsolute(file: string, contents: string | Uint8Array): void {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, contents);
  }
}
