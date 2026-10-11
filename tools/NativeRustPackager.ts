import fs from 'node:fs';
import path from 'node:path';
import { ToolCommandUtils } from './utils/ToolCommandUtils.ts';
import { ToolEntryUtils } from './utils/ToolEntryUtils.ts';
import { ToolPathUtils } from './utils/ToolPathUtils.ts';

export class NativeRustPackager {
  private static readonly DEBUG_OUTPUT: string = 'debug';
  private static readonly BINARY_SECTION: string = '[[bin]]';
  private static readonly SOURCE_PATH: string = 'macos/project/companion.rs';
  private static readonly CLIENT_PATH: string = 'app/client';
  private static readonly CRATE_NAME: string = 'observatory_native';
  private static readonly TRAITS: readonly string[] = ['ObservationClock', 'ProcessReader', 'ProcessIdentityReader', 'NetworkReader', 'MetricSource', 'EventSource', 'HarnessSessionSource'];
  private static readonly SDK_TYPES: readonly string[] = ['NativeMachTimebase'];
  private static readonly HOOK_ADAPTER: string = 'ClaudeHookAdapter';
  private static readonly HOOK_TYPES: readonly string[] = [NativeRustPackager.HOOK_ADAPTER, 'ClaudeHookMapper', 'ClaudeHookEventFactory'];
  private static readonly EMITTER_TYPES: readonly string[] = ['ProcessIdentityReader', NativeRustPackager.HOOK_ADAPTER];
  private static readonly MARKER: string = '\npub mod services {\n';
  private static readonly FRAMEWORK_EXPORTS: string = 'pub use crate::TachyonRustServices::*;';
  private static readonly EMITTER_SERVICES: readonly string[] = ['LogicalEventClientService', 'LogicalEventWireService', 'LogicalSocketService', 'NativePlatformCompositionService'];
  private static readonly LIBRARY_NAME: string = 'libobservatory_native.a';
  private static readonly EMITTER_NAME: string = 'observatory-event';
  private static readonly FIXTURE_NAME: string = 'overview-fixture';
  private static readonly FIXTURE_FEATURE: string = 'native-fixture';
  private static readonly EXECUTABLE_MODE: number = 0o755;
  private static readonly WINDOWS_PLATFORM: NodeJS.Platform = 'win32';
  private static readonly WINDOWS_DEPENDENCIES: string = [`[target.'cfg(windows)'.dependencies]`,
    'windows-sys = { version = "=0.61.2", features = ["Win32_Foundation", "Win32_Security", "Win32_Security_Authorization", "Win32_Security_Cryptography", "Win32_Storage_FileSystem", "Win32_System_Diagnostics_ToolHelp", "Win32_System_IO", "Win32_System_Pipes", "Win32_System_ProcessStatus", "Win32_System_Threading", "Win32_System_Time"] }', ''].join('\n');
  private readonly root: string;
  private readonly artifacts: string;
  private readonly source: string;
  private readonly cache: string;
  private readonly target: string;
  private readonly run: (command: readonly string[], quiet?: boolean) => void;
  private readonly platform: NodeJS.Platform;

  constructor(root: string, run: (command: readonly string[], quiet?: boolean) => void = ToolCommandUtils.run, platform: NodeJS.Platform = process.platform) {
    this.root = path.resolve(root);
    this.artifacts = ToolPathUtils.external(this.root, 'OBSERVATORY_NATIVE_ARTIFACTS');
    this.source = path.join(this.artifacts, NativeRustPackager.SOURCE_PATH);
    this.cache = ToolPathUtils.external(this.root, 'OBSERVATORY_RUST_CACHE_PATH');
    this.target = ToolPathUtils.cargoTarget(this.root, this.cache);
    this.run = run;
    this.platform = platform;
  }

  static main(source: string): void {
    ToolEntryUtils.run(source, NativeRustPackager.buildRoot);
  }

  build(): void {
    fs.mkdirSync(this.cache, { recursive: true });
    const generated: string = fs.readFileSync(this.source, 'utf8');
    NativeRustPackager.requireFresh(generated);
    fs.writeFileSync(this.source, generated + this.boundaryExports() + this.emitterExports() + this.testModule());
    const manifest: string = path.join(this.cache, 'Cargo.toml');
    this.writeEmitter();
    this.writeOverviewFixture();
    fs.writeFileSync(manifest, this.manifest());
    this.run(this.buildCommand(manifest));
    fs.copyFileSync(path.join(this.target, NativeRustPackager.DEBUG_OUTPUT, NativeRustPackager.LIBRARY_NAME), path.join(path.dirname(this.source), 'libtachyoncompanion.a'));
    this.copyEmitter();
    console.log('Rust native core compiled from client services and models; no runtime interpreter.');
  }

  manifest(): string {
    return ['[package]', 'name = "observatory-native"', 'version = "0.1.0"', 'edition = "2024"',
      '[lib]', `name = "${NativeRustPackager.CRATE_NAME}"`, `path = ${JSON.stringify(this.source)}`, 'crate-type = ["staticlib", "rlib"]',
      NativeRustPackager.BINARY_SECTION, `name = "${NativeRustPackager.EMITTER_NAME}"`, 'path = "emitter.rs"', NativeRustPackager.BINARY_SECTION, `name = "${NativeRustPackager.FIXTURE_NAME}"`,
      'path = "overview-fixture/main.rs"', `required-features = ["${NativeRustPackager.FIXTURE_FEATURE}"]`,
      '[features]', `${NativeRustPackager.FIXTURE_FEATURE} = []`, '[dependencies]', 'libc = "=0.2.189"',
      'serde = { version = "=1.0.229", features = ["derive"] }', 'serde_json = "=1.0.151"', 'regex = "=1.13.1"', ''].join('\n') + this.platformDependencies();
  }

  boundaryExports(): string {
    const names: string = [...NativeRustPackager.TRAITS, ...NativeRustPackager.SDK_TYPES, ...NativeRustPackager.HOOK_TYPES, ...NativeRustPackager.EMITTER_SERVICES].join(', ');
    return `\npub use crate::TachyonRustServices::models;\npub use crate::TachyonRustServices::{${names}};\n`
      + `\n#[cfg(any(test, feature = "${NativeRustPackager.FIXTURE_FEATURE}"))]\npub mod fixture_utils {\n`
      + 'pub use crate::TachyonRustServices::{ChartDataUtils, CommandResourceViewUtils, OverviewViewUtils, LogicalViewUtils, ProcessGraphUtils, ProcessOwnerUtils, ProcessTreeUtils, RuntimeViewUtils, SessionViewUtils};\n}\n'
      + '#[cfg(test)] pub mod utils { pub use crate::fixture_utils::*; }\n';
  }

  emitterExports(): string {
    return `${NativeRustPackager.MARKER}pub use crate::TachyonRustServices::{LogicalEventClientService, LogicalEventWireService, LogicalSocketService};\n`
      + 'pub use crate::TachyonRustServices::NativePlatformCompositionService;\n}\n';
  }

  private platformDependencies(): string {
    if (this.platform !== NativeRustPackager.WINDOWS_PLATFORM) return '';
    return NativeRustPackager.WINDOWS_DEPENDENCIES;
  }

  private static buildRoot(root: string): void {
    new NativeRustPackager(root).build();
  }

  private buildCommand(manifest: string): string[] {
    return ['cargo', 'build', '--offline', '--manifest-path', manifest, '--target-dir', this.target,
      '--lib', '--bin', NativeRustPackager.EMITTER_NAME, '--jobs', '2'];
  }

  private writeEmitter(): void {
    const source: string = path.join(this.root, NativeRustPackager.CLIENT_PATH, 'shared/native/EventEmitterMain.rs');
    const body: string = `use observatory_native::{models, ${NativeRustPackager.EMITTER_TYPES.join(', ')}};\n`
      + `use observatory_native::services::{${NativeRustPackager.EMITTER_SERVICES.join(', ')}};\n`
      + `mod entry {include!(${JSON.stringify(source)});\n${this.emitterTests()}\n}\n`
      + 'fn main() {std::process::exit(entry::EventEmitterMain::run());}\n';
    fs.writeFileSync(path.join(this.cache, 'emitter.rs'), body);
  }

  private emitterTests(): string {
    const directory: string = path.join(this.root, 'tests/client/native/emitter');
    const reader: string = path.join(directory, 'EmitterIdentityReader.rs');
    const fixture: string = path.join(directory, 'EmitterIdentityFixture.rs');
    const cases: string = path.join(directory, 'EmitterIdentityTests.rs');
    return `#[cfg(test)] mod identity_reader {include!(${JSON.stringify(reader)});}\n`
      + `#[cfg(test)] mod identity_fixture {include!(${JSON.stringify(fixture)});}\n`
      + `#[cfg(test)] mod identity_tests {include!(${JSON.stringify(cases)});}\n`;
  }

  private writeOverviewFixture(): void {
    const directory: string = path.join(this.cache, NativeRustPackager.FIXTURE_NAME);
    const request: string = path.join(this.root, 'tests/client/native/OverviewFixtureRequest.rs');
    const entry: string = path.join(this.root, 'tests/client/native/OverviewFixtureMain.rs');
    const body: string = 'use observatory_native::{models, fixture_utils as utils};\n'
      + `#[path = ${JSON.stringify(request)}]\nmod request;\nuse request::OverviewFixtureRequest;\n`
      + `#[path = ${JSON.stringify(entry)}]\nmod entry;\n`
      + 'fn main() {std::process::exit(entry::OverviewFixtureMain::entry());}\n';
    fs.mkdirSync(directory, { recursive: true });
    fs.writeFileSync(path.join(directory, 'main.rs'), body);
  }

  private copyEmitter(): void {
    const destination: string = path.join(this.artifacts, 'macos/SWARX.app/Contents/MacOS', NativeRustPackager.EMITTER_NAME);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(this.target, NativeRustPackager.DEBUG_OUTPUT, NativeRustPackager.EMITTER_NAME), destination);
    fs.chmodSync(destination, NativeRustPackager.EXECUTABLE_MODE);
  }

  private testModule(): string {
    const monitor: string = path.join(this.root, 'tests/client/native/MonitorTests.rs');
    const transport: string = path.join(this.root, 'tests/client/native/SnapshotTransportTests.rs');
    const activity: string = path.join(this.root, 'tests/client/native/ActivityHistoryTests.rs');
    const presentation: string = path.join(this.root, 'tests/client/native/ProcessPresentationTests.rs');
    const topology: string = path.join(this.root, 'tests/client/native/ProcessTopologyTests.rs');
    const logical: string = path.join(this.root, 'tests/client/native/LogicalPresentationTests.rs');
    const overview: string = path.join(this.root, 'tests/client/native/OverviewPresentationTests.rs');
    const overviewTransport: string = path.join(this.root, 'tests/client/native/OverviewTransportTests.rs');
    const command: string = path.join(this.root, 'tests/client/native/CommandResourceViewTests.rs');
    const commandFixture: string = path.join(this.root, 'tests/client/native/MonitorCommandFixture.rs');
    const charts: string = path.join(this.root, 'tests/client/native/ChartDataTests.rs');
    const commandMonitor: string = path.join(this.root, 'tests/client/native/MonitorCommandTests.rs');
    const owners: string = path.join(this.root, 'tests/client/native/ProcessOwnerTests.rs');
    return `\n#[cfg(test)]\n#[path = ${JSON.stringify(monitor)}]\nmod monitor_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(transport)}]\nmod snapshot_transport_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(activity)}]\nmod activity_history_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(presentation)}]\nmod process_presentation_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(topology)}]\nmod process_topology_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(logical)}]\nmod logical_presentation_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(overview)}]\nmod overview_presentation_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(overviewTransport)}]\nmod overview_transport_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(command)}]\nmod command_resource_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(commandFixture)}]\nmod monitor_command_fixture;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(commandMonitor)}]\nmod monitor_command_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(charts)}]\nmod chart_data_tests;\n`
      + `\n#[cfg(test)]\n#[path = ${JSON.stringify(owners)}]\nmod process_owner_tests;\n`;
  }

  private static requireFresh(source: string): void {
    if ([NativeRustPackager.MARKER, NativeRustPackager.FRAMEWORK_EXPORTS].some((marker: string): boolean => source.includes(marker))) throw new Error('Stage a fresh Tachyon native companion before compiling the core.');
  }
}

NativeRustPackager.main(import.meta.url);
