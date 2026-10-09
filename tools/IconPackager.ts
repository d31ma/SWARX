import fs from 'node:fs';
import path from 'node:path';
import { ToolCommandUtils } from './utils/ToolCommandUtils.ts';
import { ToolEntryUtils } from './utils/ToolEntryUtils.ts';
import { ToolPathUtils } from './utils/ToolPathUtils.ts';

export class IconPackager {
  private static readonly OUTPUT_FLAG: string = '-o';
  private static readonly ICON_NAME: string = 'Observatory.icns';
  private static readonly ICON_SET_NAME: string = 'Observatory.iconset';
  private static readonly APP_PATH: string = 'macos/AgentObservatory.app';
  private static readonly SWIFT_COMPILER: string = '/usr/bin/swiftc';
  private static readonly PNG_SIGNATURE: Buffer = Buffer.from('89504e470d0a1a0a', 'hex');
  private static readonly PNG_HEADER_BYTES: number = 33;
  private static readonly PNG_CHUNK_LENGTH_OFFSET: number = 8;
  private static readonly PNG_CHUNK_TYPE_OFFSET: number = 12;
  private static readonly PNG_WIDTH_OFFSET: number = 16;
  private static readonly PNG_HEIGHT_OFFSET: number = 20;
  private static readonly PNG_HEADER_LENGTH: number = 13;
  private static readonly PNG_HEADER_TYPE: Buffer = Buffer.from('IHDR', 'ascii');
  private static readonly ICON_TOOL: string = '/usr/bin/iconutil';
  private static readonly SIGN_TOOL: string = '/usr/bin/codesign';
  private static readonly PLIST_TOOL: string = '/usr/bin/plutil';
  private static readonly SIZES: readonly number[] = [16, 32, 128, 256, 512];
  private static readonly DOUBLE_SCALE: number = 2;
  private static readonly SCALES: readonly number[] = [1, IconPackager.DOUBLE_SCALE];
  private static readonly HEAD_END: string = '</head>';
  private static readonly FAVICON: string = '<link rel="icon" href="/shared/assets/observatory.svg" type="image/svg+xml">';
  private static readonly PLIST_ICON_KEY: string = 'CFBundleIconFile';
  private static readonly PLIST_DISPLAY_NAME_KEY: string = 'CFBundleDisplayName';
  private static readonly PLIST_NAME_KEY: string = 'CFBundleName';
  private static readonly DISPLAY_NAME: string = 'Swarx';
  private static readonly PERMISSION_MASK: number = 0o7777;
  private readonly root: string;
  private readonly artifacts: string;
  private readonly icons: string;
  private readonly iconset: string;
  private readonly app: string;
  private readonly cache: string;
  private readonly generator: string;
  private readonly run: (command: readonly string[], quiet?: boolean) => void;

  constructor(root: string, run: (command: readonly string[], quiet?: boolean) => void = ToolCommandUtils.run) {
    this.root = path.resolve(root);
    this.artifacts = ToolPathUtils.external(this.root, 'OBSERVATORY_NATIVE_ARTIFACTS');
    this.icons = path.join(this.artifacts, 'icons');
    this.iconset = path.join(this.icons, IconPackager.ICON_SET_NAME);
    this.app = path.join(this.artifacts, IconPackager.APP_PATH);
    this.cache = ToolPathUtils.external(this.root, 'OBSERVATORY_SWIFT_CACHE_PATH');
    this.generator = path.join(this.cache, 'native-icon-generator');
    this.run = run;
  }

  static main(source: string): void {
    ToolEntryUtils.run(source, IconPackager.packageRoot);
  }

  package(): void {
    this.generateArtwork();
    fs.mkdirSync(this.iconset, { recursive: true });
    this.packageSizes();
    this.createIcon();
    this.copyIcon();
    this.updatePlist();
    this.updateFavicons();
    this.run([IconPackager.SIGN_TOOL, '--force', '--sign', '-', this.app]);
    console.log('Original inset artwork, app icon and local favicon packaged.');
  }

  generateArtwork(): void {
    fs.mkdirSync(this.icons, { recursive: true });
    fs.mkdirSync(this.cache, { recursive: true });
    const source: string = path.join(this.root, 'tools/NativeIconGenerator.swift');
    const command: string[] = [IconPackager.SWIFT_COMPILER, '-parse-as-library', '-module-cache-path', this.cache,
      source, '-framework', 'AppKit', IconPackager.OUTPUT_FLAG, this.generator];
    this.run(command);
    this.run([this.generator, this.icons]);
  }

  packageSize(size: number): void {
    this.packageScales(size);
  }

  packageImage(size: number, scale: number): void {
    const pixels: number = size * scale;
    const suffix: string = IconPackager.scaleSuffix(scale);
    const name: string = `icon_${size}x${size}${suffix}.png`;
    const input: string = path.join(this.icons, `icon-${pixels}.png`);
    const output: string = path.join(this.iconset, name);
    const bytes: Buffer = fs.readFileSync(input);
    IconPackager.requireImage(bytes, pixels);
    fs.writeFileSync(output, bytes);
  }

  updateFavicon(file: string): void {
    const source: string = fs.readFileSync(file, 'utf8');
    const updated: string = IconPackager.faviconSource(source);
    fs.writeFileSync(file, updated);
  }

  // Header/dimension validation only; this does not decode pixels or validate PNG CRCs.
  private static requireImage(bytes: Buffer, pixels: number): void {
    if (bytes.length < IconPackager.PNG_HEADER_BYTES) throw new Error('Native icon PNG header is truncated.');
    const valid: boolean = [
      bytes.subarray(0, IconPackager.PNG_SIGNATURE.length).equals(IconPackager.PNG_SIGNATURE),
      bytes.readUInt32BE(IconPackager.PNG_CHUNK_LENGTH_OFFSET) === IconPackager.PNG_HEADER_LENGTH,
      bytes.subarray(IconPackager.PNG_CHUNK_TYPE_OFFSET, IconPackager.PNG_WIDTH_OFFSET).equals(IconPackager.PNG_HEADER_TYPE),
      bytes.readUInt32BE(IconPackager.PNG_WIDTH_OFFSET) === pixels,
      bytes.readUInt32BE(IconPackager.PNG_HEIGHT_OFFSET) === pixels,
    ].every(Boolean);
    if (!valid) throw new Error(`Native icon PNG must start with IHDR and have ${pixels}x${pixels} dimensions.`);
  }

  private static packageRoot(root: string): void {
    new IconPackager(root).package();
  }

  private packageSizes(): void {
    for (const size of IconPackager.SIZES) this.packageSize(size);
  }

  private packageScales(size: number): void {
    for (const scale of IconPackager.SCALES) this.packageImage(size, scale);
  }

  private static scaleSuffix(scale: number): string {
    if (scale === IconPackager.DOUBLE_SCALE) return '@2x';
    return '';
  }

  private createIcon(): void {
    this.run([IconPackager.ICON_TOOL, '-c', 'icns', this.iconset, IconPackager.OUTPUT_FLAG, path.join(this.icons, IconPackager.ICON_NAME)]);
  }

  private copyIcon(): void {
    const source: string = path.join(this.icons, IconPackager.ICON_NAME);
    const destination: string = path.join(this.app, 'Contents/Resources', IconPackager.ICON_NAME);
    fs.copyFileSync(source, destination);
    const metadata: fs.Stats = fs.statSync(source);
    fs.chmodSync(destination, metadata.mode & IconPackager.PERMISSION_MASK);
    fs.utimesSync(destination, metadata.atime, metadata.mtime);
  }

  private updatePlist(): void {
    const plist: string = path.join(this.app, 'Contents/Info.plist');
    this.updatePlistName(plist, IconPackager.PLIST_ICON_KEY, IconPackager.ICON_NAME);
    this.updatePlistName(plist, IconPackager.PLIST_DISPLAY_NAME_KEY, IconPackager.DISPLAY_NAME);
    this.updatePlistName(plist, IconPackager.PLIST_NAME_KEY, IconPackager.DISPLAY_NAME);
  }

  private updatePlistName(plist: string, key: string, value: string): void {
    this.run([IconPackager.PLIST_TOOL, '-replace', key, '-string', value, plist]);
  }

  private updateFavicons(): void {
    this.updateExistingFavicon(path.join(this.app, 'Contents/Resources/WebBundle/index.html'));
  }

  private updateExistingFavicon(file: string): void {
    if (fs.existsSync(file)) this.updateFavicon(file);
  }

  private static faviconSource(source: string): string {
    if (source.includes(IconPackager.FAVICON)) return source;
    return source.replaceAll(IconPackager.HEAD_END, IconPackager.FAVICON + IconPackager.HEAD_END);
  }
}

IconPackager.main(import.meta.url);
