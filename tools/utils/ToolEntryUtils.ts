import path from 'node:path';

export class ToolEntryUtils {
  private static readonly ROOT_FLAG: string = '--root';
  private static readonly MISSING_ARGUMENT_INDEX: number = -1;

  static run(source: string, action: (root: string) => void): void {
    const direct: boolean = ToolEntryUtils.isDirect(source);
    ToolEntryUtils.invoke(direct, source, action);
  }

  static root(source: string): string {
    const fallback: string = path.dirname(path.dirname(ToolEntryUtils.sourcePath(source)));
    const argumentsList: string[] = process.argv.slice(2);
    const index: number = argumentsList.indexOf(ToolEntryUtils.ROOT_FLAG);
    const chosen: string | undefined = ToolEntryUtils.choose(index, argumentsList[index + 1], fallback);
    const required: string = ToolEntryUtils.required(chosen);
    return path.resolve(required);
  }

  private static isDirect(source: string): boolean {
    const command: string = process.argv[1] ?? '';
    return path.resolve(command) === ToolEntryUtils.sourcePath(source);
  }

  private static sourcePath(source: string): string {
    return decodeURIComponent(new URL(source).pathname);
  }

  private static invoke(direct: boolean, source: string, action: (root: string) => void): void {
    if (direct) action(ToolEntryUtils.root(source));
  }

  private static choose(index: number, supplied: string | undefined, fallback: string): string | undefined {
    if (index === ToolEntryUtils.MISSING_ARGUMENT_INDEX) return fallback;
    return supplied;
  }

  private static required(value: string | undefined): string {
    if (!value) throw new Error('--root requires a project path.');
    return value;
  }
}
