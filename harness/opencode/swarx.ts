import { spawn } from "node:child_process";
import type { ChildProcess } from "node:child_process";
import { homedir } from "node:os";
import { basename, join } from "node:path";

type SwarxOpenCodeEvent = { type: string; properties?: { sessionID?: string; status?: { type?: string } } };
type SwarxOpenCodeMessage = { sessionID: string; messageID?: string };
type SwarxOpenCodeTool = { tool: string; sessionID: string; callID: string };
type SwarxOpenCodeHooks = {
  event?: (input: { event: SwarxOpenCodeEvent }) => Promise<void>;
  "chat.message"?: (input: SwarxOpenCodeMessage) => Promise<void>;
  "tool.execute.before"?: (input: SwarxOpenCodeTool) => Promise<void>;
  "tool.execute.after"?: (input: SwarxOpenCodeTool) => Promise<void>;
};
type SwarxOpenCodeRun = { message: string | undefined; calls: Set<string> };

class SwarxOpenCodePlugin {
  static readonly RELAY_PATH: readonly string[] = [".local", "bin", "swarx-hook"];
  static readonly HOOK_FLAG: string = "--opencode-hook";
  static readonly HARNESSES: ReadonlyMap<string, string> = new Map([
    ["opencode", "OpenCode"],
    ["kilo", "Kilo"],
  ]);
  static readonly UNSAFE_CHARACTERS: RegExp = /[^A-Za-z0-9_.:-]/g;
  static readonly SAFE_REPLACEMENT: string = "-";
  static readonly IDENTIFIER_LIMIT: number = 80;
  static readonly DELIVERY_LIMIT_MILLISECONDS: number = 3000;
  static readonly STATUS_EVENT: string = "session.status";
  static readonly IDLE_EVENT: string = "session.idle";
  static readonly BUSY_STATUS: string = "busy";
  static readonly USER_PROMPT_SUBMIT: string = "UserPromptSubmit";
  static readonly PRE_TOOL_USE: string = "PreToolUse";
  static readonly POST_TOOL_USE: string = "PostToolUse";
  static readonly POST_TOOL_USE_FAILURE: string = "PostToolUseFailure";
  static readonly STOP: string = "Stop";

  #relay: string = join(homedir(), ...SwarxOpenCodePlugin.RELAY_PATH);
  #queue: Promise<void> = Promise.resolve();
  #runs: Map<string, SwarxOpenCodeRun> = new Map();
  #harness: string;

  constructor(harness: string) {
    this.#harness = harness;
  }

  static async install(): Promise<SwarxOpenCodeHooks> {
    const harness: string | undefined = SwarxOpenCodePlugin.HARNESSES.get(basename(process.execPath));
    return SwarxOpenCodePlugin.#hooksFor(harness);
  }

  static #hooksFor(harness: string | undefined): SwarxOpenCodeHooks {
    if (harness === undefined) {
      return {};
    }
    return new SwarxOpenCodePlugin(harness).#hooks();
  }

  static #identifier(value: string): string {
    const safe: string = value.replace(SwarxOpenCodePlugin.UNSAFE_CHARACTERS, SwarxOpenCodePlugin.SAFE_REPLACEMENT);
    return safe.slice(0, SwarxOpenCodePlugin.IDENTIFIER_LIMIT);
  }

  #hooks(): SwarxOpenCodeHooks {
    return {
      event: async (input: { event: SwarxOpenCodeEvent }): Promise<void> => this.#event(input.event),
      "chat.message": async (input: SwarxOpenCodeMessage): Promise<void> => this.#message(input),
      "tool.execute.before": async (input: SwarxOpenCodeTool): Promise<void> => this.#toolStarted(input),
      "tool.execute.after": async (input: SwarxOpenCodeTool): Promise<void> => this.#toolEnded(input),
    };
  }

  #event(event: SwarxOpenCodeEvent): void {
    const session: string | undefined = event.properties?.sessionID;
    switch (event.type) {
      case SwarxOpenCodePlugin.STATUS_EVENT:
        return this.#status(session, event.properties?.status?.type);
      case SwarxOpenCodePlugin.IDLE_EVENT:
        return this.#idle(session);
    }
  }

  #status(session: string | undefined, status: string | undefined): void {
    if (session === undefined || status !== SwarxOpenCodePlugin.BUSY_STATUS) {
      return;
    }
    this.#report(session, { hook_event_name: SwarxOpenCodePlugin.USER_PROMPT_SUBMIT, prompt_id: this.#run(session).message });
  }

  #idle(session: string | undefined): void {
    if (session === undefined) {
      return;
    }
    this.#finishRun(session, this.#run(session));
  }

  #finishRun(session: string, run: SwarxOpenCodeRun): void {
    this.#runs.delete(session);
    run.calls.forEach((call: string): void => this.#report(session, this.#unfinishedFields(run, call)));
    this.#report(session, { hook_event_name: SwarxOpenCodePlugin.STOP, prompt_id: run.message });
  }

  #unfinishedFields(run: SwarxOpenCodeRun, call: string): Record<string, string | undefined> {
    return this.#callFields(SwarxOpenCodePlugin.POST_TOOL_USE_FAILURE, run, call);
  }

  #message(input: SwarxOpenCodeMessage): void {
    this.#run(input.sessionID).message = input.messageID && SwarxOpenCodePlugin.#identifier(input.messageID);
  }

  #toolStarted(input: SwarxOpenCodeTool): void {
    const run: SwarxOpenCodeRun = this.#run(input.sessionID);
    const call: string = SwarxOpenCodePlugin.#identifier(input.callID);
    run.calls.add(call);
    this.#report(input.sessionID, { ...this.#callFields(SwarxOpenCodePlugin.PRE_TOOL_USE, run, call), tool_name: input.tool });
  }

  #toolEnded(input: SwarxOpenCodeTool): void {
    const run: SwarxOpenCodeRun = this.#run(input.sessionID);
    const call: string = SwarxOpenCodePlugin.#identifier(input.callID);
    run.calls.delete(call);
    this.#report(input.sessionID, { ...this.#callFields(SwarxOpenCodePlugin.POST_TOOL_USE, run, call), tool_name: input.tool });
  }

  #run(session: string): SwarxOpenCodeRun {
    const run: SwarxOpenCodeRun = this.#runs.get(session) ?? { message: undefined, calls: new Set() };
    this.#runs.set(session, run);
    return run;
  }

  #callFields(name: string, run: SwarxOpenCodeRun, call: string): Record<string, string | undefined> {
    return { hook_event_name: name, prompt_id: run.message, tool_use_id: call };
  }

  #report(session: string, fields: Record<string, string | undefined>): void {
    const payload: string = JSON.stringify({ ...fields, session_id: session, harness: this.#harness });
    this.#queue = this.#queue.then((): Promise<void> => this.#deliver(payload));
  }

  #deliver(payload: string): Promise<void> {
    return new Promise<void>((resolve: () => void): void => this.#start(payload, resolve));
  }

  #start(payload: string, resolve: () => void): void {
    const timer: NodeJS.Timeout = setTimeout(resolve, SwarxOpenCodePlugin.DELIVERY_LIMIT_MILLISECONDS);
    timer.unref();
    const finish: () => void = (): void => {
      clearTimeout(timer);
      resolve();
    };
    try {
      this.#spawn(payload, finish);
    } catch {
      finish();
    }
  }

  #spawn(payload: string, finish: () => void): void {
    const child: ChildProcess = spawn(this.#relay, [SwarxOpenCodePlugin.HOOK_FLAG], {
      detached: true,
      stdio: ["pipe", "ignore", "ignore"],
    });
    child.once("error", finish);
    child.once("close", finish);
    child.stdin?.once("error", finish);
    child.stdin?.end(payload);
    child.unref();
  }
}

export default SwarxOpenCodePlugin.install;
