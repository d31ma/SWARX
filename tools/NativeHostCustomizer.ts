import fs from 'node:fs';
import path from 'node:path';
import { ToolCommandUtils } from './utils/ToolCommandUtils.ts';
import { ToolEntryUtils } from './utils/ToolEntryUtils.ts';
import { ToolPathUtils } from './utils/ToolPathUtils.ts';

export class NativeHostCustomizer {
  private static readonly FILE_ENCODING: BufferEncoding = 'utf8';
  private static readonly FRAMEWORK_FLAG: string = '-framework';
  private static readonly SQLITE_LIBRARY_FLAG: string = '-lsqlite3';
  private static readonly SOURCE_PATH: string = 'macos/project/TachyonHost.swift';
  private static readonly COMPANION_PATH: string = 'macos/project/libtachyoncompanion.a';
  private static readonly EXECUTABLE_PATH: string = 'macos/AgentObservatory.app/Contents/MacOS/AgentObservatory';
  private static readonly VERIFY_SOURCE: string = 'NativePreviewVerifier.swift';
  private static readonly LIFETIME_SOURCE: string = 'NativeApplicationLifetimeVerifier.swift';
  private static readonly NATIVE_SOURCE: string = 'app/client/shared/native';
  private static readonly ICON_SOURCE: string = 'NativeApplicationIcon.swift';
  private static readonly QA_SOURCE: string = 'tools/native';
  private static readonly COLLECTOR_RESOURCE: string = 'macos/AgentObservatory.app/Contents/Resources/Collector';
  private static readonly REPORT_SOURCE: string = 'NativePreviewReport.js';
  private static readonly REPORT_MARKER: string = '"__NATIVE_REPORT_SCRIPT__"';
  private static readonly MINIMUM_OLD: string = 'window.contentMinSize = NSSize(width: 0, height: 0)';
  private static readonly MINIMUM_NEW: string = 'window.contentMinSize = NSSize(width: 900, height: 640)';
  private static readonly WINDOW_TITLE_OLD: string = 'window.title = "Agent Observatory"';
  private static readonly WINDOW_TITLE_NEW: string = 'window.title = "Swarx Agent Observatory"';
  private static readonly MENU_NAME_OLD: string = 'menus.install(NSApplication.shared, window, "Agent Observatory", menuNavigation)';
  private static readonly MENU_NAME_NEW: string = 'menus.install(NSApplication.shared, window, "Swarx", menuNavigation)';
  private static readonly MENU_ACTIONS_OLD: string = 'menus.install(NSApplication.shared, window, "Agent Observatory", menuNavigation, menuActions)';
  private static readonly MENU_ACTIONS_NEW: string = 'menus.install(NSApplication.shared, window, "Swarx", menuNavigation, menuActions)';
  private static readonly ACTIVATION: string = '        NSApplication.shared.activate(ignoringOtherApps: true)';
  private static readonly ACTIVE: string = '        Telemetry.shared.record("controller.active")';
  private static readonly ICON_CALL: string = '        NativeApplicationIcon.applyIcon()\n';
  private static readonly QA_CALL: string = '\n        NativePreviewVerifier.begin(view: webView, window: window)';
  private static readonly LIFETIME_CALL: string = '\n        NativeApplicationLifetimeVerifier.begin(view: webView, window: window)';
  private static readonly LEGACY_METHOD: string = 'private func verifyPreview()';
  private static readonly SWIFT_COMPILER: string = '/usr/bin/swiftc';
  private readonly root: string;
  private readonly artifacts: string;
  private readonly source: string;
  private readonly cache: string;
  private readonly verifier: string;
  private readonly run: (command: readonly string[], quiet?: boolean) => void;

  constructor(root: string, run: (command: readonly string[], quiet?: boolean) => void = ToolCommandUtils.run) {
    this.root = path.resolve(root);
    this.artifacts = ToolPathUtils.external(this.root, 'OBSERVATORY_NATIVE_ARTIFACTS');
    this.source = path.join(this.artifacts, NativeHostCustomizer.SOURCE_PATH);
    this.cache = ToolPathUtils.external(this.root, 'OBSERVATORY_SWIFT_CACHE_PATH');
    this.verifier = path.join(this.cache, NativeHostCustomizer.VERIFY_SOURCE);
    this.run = run;
  }

  static main(source: string): void {
    ToolEntryUtils.run(source, NativeHostCustomizer.buildRoot);
  }

  build(): void {
    const source: string = fs.readFileSync(this.source, NativeHostCustomizer.FILE_ENCODING);
    const customized: string = this.customize(source);
    fs.writeFileSync(this.source, customized);
    fs.mkdirSync(this.cache, { recursive: true });
    fs.writeFileSync(this.verifier, this.renderVerifier());
    this.removeLegacyCollector();
    fs.mkdirSync(path.dirname(path.join(this.artifacts, NativeHostCustomizer.EXECUTABLE_PATH)), { recursive: true });
    this.run(this.compilerCommand());
    console.log('Project native icon, minimum size and optional WebView QA helper compiled.');
  }

  customize(source: string): string {
    NativeHostCustomizer.requireNoLegacy(source);
    const minimumPresent: boolean = NativeHostCustomizer.minimumPresent(source);
    NativeHostCustomizer.requireMinimum(minimumPresent);
    NativeHostCustomizer.requireAnchor(source, NativeHostCustomizer.ACTIVATION);
    NativeHostCustomizer.requireAnchor(source, NativeHostCustomizer.ACTIVE);
    const minimum: string = source.replaceAll(NativeHostCustomizer.MINIMUM_OLD, NativeHostCustomizer.MINIMUM_NEW);
    const branded: string = NativeHostCustomizer.applyBranding(minimum);
    const icon: string = NativeHostCustomizer.insertIcon(branded);
    const qa: string = NativeHostCustomizer.insertQa(icon);
    return NativeHostCustomizer.insertLifetime(qa);
  }

  renderVerifier(): string {
    const template: string = fs.readFileSync(path.join(this.root, NativeHostCustomizer.QA_SOURCE, NativeHostCustomizer.VERIFY_SOURCE), NativeHostCustomizer.FILE_ENCODING);
    const script: string = fs.readFileSync(path.join(this.root, NativeHostCustomizer.QA_SOURCE, NativeHostCustomizer.REPORT_SOURCE), NativeHostCustomizer.FILE_ENCODING);
    NativeHostCustomizer.requireReportMarker(template);
    const literal: string = JSON.stringify(script);
    return NativeHostCustomizer.embedScript(template, literal);
  }

  compilerCommand(): string[] {
    return [NativeHostCustomizer.SWIFT_COMPILER, '-parse-as-library', '-module-cache-path', this.cache,
      this.source, path.join(this.artifacts, NativeHostCustomizer.COMPANION_PATH),
      path.join(this.root, NativeHostCustomizer.NATIVE_SOURCE, NativeHostCustomizer.ICON_SOURCE), this.verifier,
      path.join(this.root, NativeHostCustomizer.QA_SOURCE, NativeHostCustomizer.LIFETIME_SOURCE),
      NativeHostCustomizer.FRAMEWORK_FLAG, 'AppKit', NativeHostCustomizer.FRAMEWORK_FLAG, 'WebKit', NativeHostCustomizer.SQLITE_LIBRARY_FLAG, '-o', path.join(this.artifacts, NativeHostCustomizer.EXECUTABLE_PATH)];
  }

  removeLegacyCollector(): void {
    const destination: string = path.join(this.artifacts, NativeHostCustomizer.COLLECTOR_RESOURCE);
    const metadata: fs.Stats | undefined = NativeHostCustomizer.optionalMetadata(destination);
    const directory: boolean = NativeHostCustomizer.safeDirectory(metadata);
    NativeHostCustomizer.requireDirectory(directory);
    NativeHostCustomizer.clearCollector(destination, metadata);
  }

  private static buildRoot(root: string): void {
    new NativeHostCustomizer(root).build();
  }

  private static requireNoLegacy(source: string): void {
    if (source.includes(NativeHostCustomizer.LEGACY_METHOD)) throw new Error('Rebuild the Tachyon native bundle before applying the current generator.');
  }

  private static minimumPresent(source: string): boolean {
    return source.includes(NativeHostCustomizer.MINIMUM_OLD) || source.includes(NativeHostCustomizer.MINIMUM_NEW);
  }

  private static requireMinimum(found: boolean): void {
    if (!found) throw new Error('Native minimum window anchor is missing.');
  }

  private static requireAnchor(source: string, anchor: string): void {
    if (!source.includes(anchor)) throw new Error('Required native generation anchor is missing.');
  }

  private static requireReportMarker(source: string): void {
    if (!source.includes(NativeHostCustomizer.REPORT_MARKER)) throw new Error('Native report script marker is missing.');
  }

  private static embedScript(template: string, literal: string): string {
    return template.replaceAll(NativeHostCustomizer.REPORT_MARKER, (): string => literal);
  }

  private static applyBranding(source: string): string {
    const windowPresent: boolean = NativeHostCustomizer.brandingPresent(source, NativeHostCustomizer.WINDOW_TITLE_OLD, NativeHostCustomizer.WINDOW_TITLE_NEW);
    NativeHostCustomizer.requireBranding(windowPresent);
    const menuPresent: boolean = NativeHostCustomizer.brandingPresent(source, NativeHostCustomizer.MENU_NAME_OLD, NativeHostCustomizer.MENU_NAME_NEW)
      || NativeHostCustomizer.brandingPresent(source, NativeHostCustomizer.MENU_ACTIONS_OLD, NativeHostCustomizer.MENU_ACTIONS_NEW);
    NativeHostCustomizer.requireBranding(menuPresent);
    const window: string = source.replaceAll(NativeHostCustomizer.WINDOW_TITLE_OLD, NativeHostCustomizer.WINDOW_TITLE_NEW);
    return window.replaceAll(NativeHostCustomizer.MENU_NAME_OLD, NativeHostCustomizer.MENU_NAME_NEW)
      .replaceAll(NativeHostCustomizer.MENU_ACTIONS_OLD, NativeHostCustomizer.MENU_ACTIONS_NEW);
  }

  private static brandingPresent(source: string, previous: string, current: string): boolean {
    return source.includes(previous) || source.includes(current);
  }

  private static requireBranding(found: boolean): void {
    if (!found) throw new Error('Native application branding anchor is missing. Review the generated host before customizing it.');
  }

  private static insertIcon(source: string): string {
    if (source.includes(NativeHostCustomizer.ICON_CALL.trim())) return source;
    return source.replaceAll(NativeHostCustomizer.ACTIVATION, NativeHostCustomizer.ICON_CALL + NativeHostCustomizer.ACTIVATION);
  }

  private static insertQa(source: string): string {
    if (source.includes(NativeHostCustomizer.QA_CALL.trim())) return source;
    return source.replaceAll(NativeHostCustomizer.ACTIVE, NativeHostCustomizer.ACTIVE + NativeHostCustomizer.QA_CALL);
  }

  private static insertLifetime(source: string): string {
    if (source.includes(NativeHostCustomizer.LIFETIME_CALL.trim())) return source;
    return source.replaceAll(NativeHostCustomizer.QA_CALL, NativeHostCustomizer.QA_CALL + NativeHostCustomizer.LIFETIME_CALL);
  }

  private static optionalMetadata(destination: string): fs.Stats | undefined {
    if (!fs.existsSync(destination)) return undefined;
    return fs.lstatSync(destination);
  }

  private static safeDirectory(metadata: fs.Stats | undefined): boolean {
    return !metadata || metadata.isDirectory();
  }

  private static requireDirectory(valid: boolean): void {
    if (!valid) throw new Error('Legacy Collector resource must be a real directory.');
  }

  private static clearCollector(destination: string, metadata: fs.Stats | undefined): void {
    if (metadata) fs.rmSync(destination, { recursive: true });
  }
}

NativeHostCustomizer.main(import.meta.url);
