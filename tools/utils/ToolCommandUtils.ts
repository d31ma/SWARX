import { spawnSync, type StdioOptions } from 'node:child_process';

export class ToolCommandUtils {
  private static readonly SUCCESS_STATUS: number = 0;
  private static readonly INHERITED_OUTPUT: 'inherit' = 'inherit';

  static run(command: readonly string[], quiet: boolean = false): void {
    ToolCommandUtils.execute(command, quiet);
  }

  private static execute(command: readonly string[], quiet: boolean): void {
    const program: string = ToolCommandUtils.program(command[0]);
    const result = spawnSync(program, command.slice(1), { stdio: ToolCommandUtils.output(quiet) });
    ToolCommandUtils.requireStarted(result.error);
    ToolCommandUtils.requireSuccess(result.status, program);
  }

  private static program(value: string | undefined): string {
    if (!value) throw new Error('A packaging command is required.');
    return value;
  }

  private static output(quiet: boolean): StdioOptions {
    if (quiet) return [ToolCommandUtils.INHERITED_OUTPUT, 'ignore', ToolCommandUtils.INHERITED_OUTPUT];
    return ToolCommandUtils.INHERITED_OUTPUT;
  }

  private static requireStarted(failure: Error | undefined): void {
    if (failure) throw failure;
  }

  private static requireSuccess(status: number | null, program: string): void {
    if (status !== ToolCommandUtils.SUCCESS_STATUS) throw new Error(`Packaging command failed: ${program}`);
  }
}
