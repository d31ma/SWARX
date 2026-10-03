import fs from 'node:fs';
import path from 'node:path';

export class ToolPathUtils {
  static external(root: string, name: string): string {
    const supplied: string = ToolPathUtils.required(name);
    ToolPathUtils.requireAbsolute(supplied, name);
    const chosen: string = path.resolve(supplied);
    ToolPathUtils.requireOutside(root, chosen, name);
    ToolPathUtils.requireOutside(fs.realpathSync(root), ToolPathUtils.physical(chosen), name);
    return chosen;
  }

  static separate(paths: readonly string[]): void {
    const physical: readonly string[] = paths.map((chosen: string): string => ToolPathUtils.physical(chosen));
    ToolPathUtils.requireSeparated(physical);
  }

  static cargoTarget(root: string, cache: string): string {
    const configured: string | undefined = process.env.CARGO_TARGET_DIR;
    const value: string = ToolPathUtils.targetValue(configured, cache);
    const supplied: string = ToolPathUtils.requireValue(value, 'CARGO_TARGET_DIR');
    const chosen: string = path.resolve(root, supplied);
    ToolPathUtils.requireOutside(root, chosen, 'CARGO_TARGET_DIR');
    ToolPathUtils.requireOutside(fs.realpathSync(root), ToolPathUtils.physical(chosen), 'CARGO_TARGET_DIR');
    return chosen;
  }

  static required(name: string): string {
    const value: string | undefined = process.env[name];
    return ToolPathUtils.requireValue(value, name);
  }

  private static targetValue(configured: string | undefined, cache: string): string {
    if (configured === undefined) return path.join(cache, 'target');
    return configured;
  }

  private static requireValue(value: string | undefined, name: string): string {
    if (!value?.trim()) throw new Error(name + ' requires an explicitly supplied value.');
    return value;
  }

  private static requireAbsolute(value: string, name: string): void {
    if (!path.isAbsolute(value)) throw new Error(name + ' requires an absolute external path.');
  }

  private static requireOutside(root: string, chosen: string, name: string): void {
    const outside: boolean = ToolPathUtils.outside(root, chosen);
    ToolPathUtils.requireExternal(outside, name);
  }

  private static outside(root: string, chosen: string): boolean {
    const relative: string = path.relative(root, chosen);
    const outside: readonly boolean[] = [relative === '..', relative.startsWith('..' + path.sep), path.isAbsolute(relative)];
    return outside.some(Boolean);
  }

  private static requireSeparated(paths: readonly string[]): void {
    for (let index: number = 0; index < paths.length; index += 1) ToolPathUtils.requireRemaining(paths, index);
  }

  private static requireRemaining(paths: readonly string[], index: number): void {
    for (const chosen of paths.slice(index + 1)) ToolPathUtils.requirePair(paths[index] as string, chosen);
  }

  private static requirePair(first: string, second: string): void {
    const separate: readonly boolean[] = [ToolPathUtils.outside(first, second), ToolPathUtils.outside(second, first)];
    ToolPathUtils.requireSeparation(separate.every(Boolean));
  }

  private static requireSeparation(separate: boolean): void {
    if (!separate) throw new Error('Build output, caches and rollback must be separate directories.');
  }

  private static requireExternal(outside: boolean, name: string): void {
    if (!outside) throw new Error(name + ' must be outside the source project.');
  }

  private static physical(chosen: string): string {
    const ancestor: string = ToolPathUtils.ancestor(chosen);
    return path.resolve(fs.realpathSync(ancestor), path.relative(ancestor, chosen));
  }

  private static ancestor(chosen: string): string {
    let ancestor: string = chosen;
    while (!fs.existsSync(ancestor)) ancestor = path.dirname(ancestor);
    return ancestor;
  }
}
