import fs from 'node:fs';
import path from 'node:path';
import { crc32, inflateSync } from 'node:zlib';
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
  private static readonly MAX_PNG_BYTES: number = 16 * 1024 * 1024;
  private static readonly MAX_ICNS_BYTES: number = 0xffffffff;
  private static readonly RESOURCES: readonly (readonly [string, string, number])[] = [
    ['is32', 'icon_16x16.png', 16], ['s8mk', 'icon_16x16.png', 16],
    ['ic11', 'icon_16x16@2x.png', 32],
    ['il32', 'icon_32x32.png', 32], ['l8mk', 'icon_32x32.png', 32],
    ['ic12', 'icon_32x32@2x.png', 64],
    ['ic07', 'icon_128x128.png', 128], ['ic13', 'icon_128x128@2x.png', 256],
    ['ic08', 'icon_256x256.png', 256], ['ic14', 'icon_256x256@2x.png', 512],
    ['ic09', 'icon_512x512.png', 512], ['ic10', 'icon_512x512@2x.png', 1024],
  ];
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
    const images = new Map<string, { bytes: Buffer; compressed: Buffer; pixels: number }>();
    const entries: Buffer[] = [];
    for (const [type, name, pixels] of IconPackager.RESOURCES) {
      let image = images.get(name);
      if (image === undefined) {
        const bytes: Buffer = fs.readFileSync(path.join(this.iconset, name));
        IconPackager.requireImage(bytes, pixels);
        const compressed: Buffer = IconPackager.pngData(bytes);
        image = { bytes, compressed, pixels };
        images.set(name, image);
      }
      let payload: Buffer = image.bytes;
      if (type === 'is32' || type === 'il32' || type === 's8mk' || type === 'l8mk') {
        const rgba: Buffer = IconPackager.smallPixels(image.compressed, pixels);
        const channels: Buffer[] = [0, 1, 2, 3].map((channel: number): Buffer =>
          Buffer.from(rgba.filter((_value: number, index: number): boolean => index % 4 === channel)));
        payload = type === 's8mk' || type === 'l8mk' ? channels[3]!
          : Buffer.concat(channels.slice(0, 3).map(IconPackager.literalChannel));
      }
      entries.push(IconPackager.frame(type, payload));
    }
    const toc: Buffer = IconPackager.frame('TOC ', Buffer.concat(entries.map((entry: Buffer): Buffer => entry.subarray(0, 8))));
    const bytes: Buffer = IconPackager.frame('icns', Buffer.concat([toc, ...entries]));
    const validation: string = fs.mkdtempSync(path.join(this.icons, 'icns-validation-'));
    const candidate: string = path.join(validation, IconPackager.ICON_NAME);
    const decoded: string = path.join(validation, 'decoded.iconset');
    fs.writeFileSync(candidate, bytes);
    this.run([IconPackager.ICON_TOOL, '-c', 'iconset', candidate, IconPackager.OUTPUT_FLAG, decoded]);
    if (!fs.statSync(decoded).isDirectory() || fs.lstatSync(decoded).isSymbolicLink()
      || JSON.stringify(fs.readdirSync(decoded).sort()) !== JSON.stringify([...images.keys()].sort())) {
      throw new Error('ICNS decoder returned an unexpected iconset.');
    }
    for (const [name, image] of images) {
      const delivered: string = path.join(decoded, name);
      if (!fs.lstatSync(delivered).isFile() || fs.lstatSync(delivered).isSymbolicLink()) {
        throw new Error('ICNS decoder returned a non-regular PNG.');
      }
      const decodedBytes: Buffer = fs.readFileSync(delivered);
      IconPackager.requireImage(decodedBytes, image.pixels);
      const decodedData: Buffer = IconPackager.pngData(decodedBytes);
      if (image.pixels <= 32 && !IconPackager.smallPixels(decodedData, image.pixels)
        .equals(IconPackager.smallPixels(image.compressed, image.pixels))) {
        throw new Error('ICNS decoder changed original small PNG pixels.');
      }
      if (!fs.readFileSync(path.join(this.iconset, name)).equals(image.bytes)
        || !fs.readFileSync(path.join(this.icons, `icon-${image.pixels}.png`)).equals(image.bytes)) {
        throw new Error('ICNS validation changed original PNG bytes.');
      }
    }
    if (!fs.readFileSync(candidate).equals(bytes)) throw new Error('ICNS decoder changed the candidate.');
    fs.renameSync(candidate, path.join(this.icons, IconPackager.ICON_NAME));
  }

  private static frame(type: string, payload: Buffer): Buffer {
    const length: number = payload.length + 8;
    if (!/^[A-Za-z0-9 ]{4}$/.test(type) || !Number.isSafeInteger(length) || length > IconPackager.MAX_ICNS_BYTES) {
      throw new Error('Invalid ICNS resource or 32-bit length.');
    }
    const header: Buffer = Buffer.alloc(8);
    header.write(type, 0, 'ascii');
    header.writeUInt32BE(length, 4);
    return Buffer.concat([header, payload]);
  }

  // Only the generated 8-bit RGBA, noninterlaced PNG subset is supported.
  private static pngData(bytes: Buffer): Buffer {
    if (bytes.length > IconPackager.MAX_PNG_BYTES || !bytes.subarray(24, 29).equals(Buffer.from([8, 6, 0, 0, 0]))) {
      throw new Error('Unsupported or oversized native icon PNG.');
    }
    const data: Buffer[] = [];
    let offset: number = 8;
    let ended: boolean = false;
    let afterData: boolean = false;
    while (offset < bytes.length) {
      if (bytes.length - offset < 12) throw new Error('Truncated PNG chunk.');
      const length: number = bytes.readUInt32BE(offset);
      const end: number = offset + 12 + length;
      if (end > bytes.length) throw new Error('Truncated PNG chunk payload.');
      const rawType: Buffer = bytes.subarray(offset + 4, offset + 8);
      const type: string = rawType.toString('latin1');
      if (!/^[A-Za-z]{2}[A-Z][A-Za-z]$/.test(type)
        || crc32(bytes.subarray(offset + 4, end - 4)) !== bytes.readUInt32BE(end - 4)) {
        throw new Error('Invalid PNG chunk type or CRC.');
      }
      if (offset === 8) {
        if (type !== 'IHDR' || length !== 13) throw new Error('PNG must start with IHDR.');
      } else if (type === 'IDAT') {
        if (afterData) throw new Error('PNG IDAT chunks must be contiguous.');
        data.push(bytes.subarray(offset + 8, end - 4));
      } else if (type === 'IEND') {
        if (length !== 0 || end !== bytes.length || data.length === 0) throw new Error('Invalid PNG IEND.');
        ended = true;
      } else {
        if ((rawType[0]! & 32) === 0) throw new Error('Unsupported critical PNG chunk.');
        if (data.length > 0) afterData = true;
      }
      offset = end;
    }
    if (!ended || data.length === 0) throw new Error('PNG IDAT/IEND is missing.');
    const compressed: Buffer = Buffer.concat(data);
    if (compressed.length === 0) throw new Error('PNG IDAT is empty.');
    return compressed;
  }

  private static smallPixels(compressed: Buffer, pixels: number): Buffer {
    if (pixels !== 16 && pixels !== 32) throw new Error('Legacy ICNS pixels must be 16 or 32.');
    const stride: number = pixels * 4;
    const length: number = pixels * (stride + 1);
    // Node's info option returns this runtime shape; its declarations retain the Buffer return type.
    const result = inflateSync(compressed, { maxOutputLength: length, info: true }) as unknown as
      { buffer: Buffer; engine: { bytesWritten: number } };
    if (result.buffer.length !== length || result.engine.bytesWritten !== compressed.length) {
      throw new Error('Invalid PNG IDAT inflated length or trailing compressed bytes.');
    }
    const rgba: Buffer = Buffer.alloc(pixels * stride);
    for (let y: number = 0; y < pixels; y++) {
      const filter: number = result.buffer[y * (stride + 1)]!;
      if (filter > 4) throw new Error('Unsupported PNG scanline filter.');
      for (let x: number = 0; x < stride; x++) {
        const index: number = y * stride + x;
        const left: number = x >= 4 ? rgba[index - 4]! : 0;
        const above: number = y > 0 ? rgba[index - stride]! : 0;
        const upperLeft: number = y > 0 && x >= 4 ? rgba[index - stride - 4]! : 0;
        const prediction: number = filter === 0 ? 0 : filter === 1 ? left : filter === 2 ? above
          : filter === 3 ? Math.floor((left + above) / 2) : IconPackager.paeth(left, above, upperLeft);
        rgba[index] = (result.buffer[y * (stride + 1) + 1 + x]! + prediction) & 255;
      }
    }
    return rgba;
  }

  private static paeth(left: number, above: number, upperLeft: number): number {
    const prediction: number = left + above - upperLeft;
    const a: number = Math.abs(prediction - left);
    const b: number = Math.abs(prediction - above);
    const c: number = Math.abs(prediction - upperLeft);
    return a <= b && a <= c ? left : b <= c ? above : upperLeft;
  }

  private static literalChannel(channel: Buffer): Buffer {
    const packets: Buffer[] = [];
    for (let offset: number = 0; offset < channel.length; offset += 128) {
      const packet: Buffer = channel.subarray(offset, offset + 128);
      packets.push(Buffer.from([packet.length - 1]), packet);
    }
    return Buffer.concat(packets);
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
