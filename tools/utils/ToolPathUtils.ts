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

  static cargoTarget(root: string, cache: string): string {
    const configured: string | undefined = process.env.CARGO_TARGET_DIR;
    const supplied: string = ToolPathUtils.targetValue(configured, cache);
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
    return ToolPathUtils.requireValue(configured, 'CARGO_TARGET_DIR');
  }

  private static requireValue(value: string | undefined, name: string): string {
    if (!value?.trim()) throw new Error(name + ' requires an explicitly supplied value.');
    return value;
  }

  private static requireAbsolute(value: string, name: string): void {
    if (!path.isAbsolute(value)) throw new Error(name + ' requires an absolute external path.');
  }

  private static requireOutside(root: string, chosen: string, name: string): void {
    const relative: string = path.relative(root, chosen);
    const outside: boolean = relative === '..' || relative.startsWith('..' + path.sep) || path.isAbsolute(relative);
    ToolPathUtils.requireExternal(outside, name);
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
