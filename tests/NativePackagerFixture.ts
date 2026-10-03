import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawnSync } from 'node:child_process';

export class NativePackagerFixture {
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

  write(relative: string, contents: string): string {
    const file: string = this.destination(relative);
    this.writeAbsolute(file, contents);
    return file;
  }

  rust(): string {
    return this.write('app/dist/macos/project/companion.rs', 'pub mod TachyonRustServices {}\n');
  }

  host(): string {
    const source: string = 'window.contentMinSize = NSSize(width: 0, height: 0)\n'
      + '        NSApplication.shared.activate(ignoringOtherApps: true)\n'
      + '        Telemetry.shared.record("controller.active")\n';
    this.write('tools/native/NativePreviewVerifier.swift', 'let script = "__NATIVE_REPORT_SCRIPT__"\n');
    this.write('tools/native/NativePreviewReport.js', 'const note = "$& $$";\n');
    this.write('app/client/shared/native/NativeApplicationIcon.swift', 'final class NativeApplicationIcon {}\n');
    return this.write('app/dist/macos/project/TachyonHost.swift', source);
  }

  icon(): void {
    this.write('app/dist/macos/AgentObservatory.app/Contents/Info.plist', '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>local.fixture.monitor</string></dict></plist>');
    this.write('app/dist/macos/AgentObservatory.app/Contents/Resources/WebBundle/index.html', '<html><head></head><body>Fixture</body></html>');
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
      for (const pixels of [16, 32, 64, 128, 256, 512, 1024]) this.write(`app/icons/icon-${pixels}.png`, 'fixture png');
    } else if (command[0] === '/usr/bin/sips') {
      this.writeAbsolute(this.output(command, '--out'), 'fixture resized png');
    } else if (command[0] === '/usr/bin/iconutil') {
      this.writeAbsolute(this.output(command, '-o'), 'icnsfixture');
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

  private writeAbsolute(file: string, contents: string): void {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, contents);
  }
}
