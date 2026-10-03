import { spawn } from "node:child_process";
import type { ChildProcess } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";
import type {
  AgentEndEvent,
  AgentStartEvent,
  ExtensionAPI,
  ExtensionContext,
  ExtensionEvent,
  SessionShutdownEvent,
  SessionStartEvent,
} from "@mariozechner/pi-coding-agent";

class SwarxPiExtension {
  static readonly RELAY_PATH: readonly string[] = [".local", "bin", "swarx-hook"];
  static readonly HOOK_FLAG: string = "--pi-hook";
  static readonly UNSAFE_CHARACTERS: RegExp = /[^A-Za-z0-9_.:-]/g;
  static readonly SAFE_REPLACEMENT: string = "-";
  static readonly IDENTIFIER_LIMIT: number = 80;
  static readonly DELIVERY_LIMIT_MILLISECONDS: number = 3000;
  static readonly RUN_PREFIX: string = "run-";
  static readonly RELOAD_REASON: string = "reload";
  static readonly SESSION_START: string = "SessionStart";
  static readonly USER_PROMPT_SUBMIT: string = "UserPromptSubmit";
  static readonly PRE_TOOL_USE: string = "PreToolUse";
  static readonly POST_TOOL_USE: string = "PostToolUse";
  static readonly POST_TOOL_USE_FAILURE: string = "PostToolUseFailure";
  static readonly STOP: string = "Stop";
  static readonly SESSION_END: string = "SessionEnd";

  #relay: string = join(homedir(), ...SwarxPiExtension.RELAY_PATH);
  #queue: Promise<void> = Promise.resolve();
  #run: string | undefined = undefined;

  static install(pi: ExtensionAPI): void {
    const extension: SwarxPiExtension = new SwarxPiExtension();
    pi.on("session_start", (_event: SessionStartEvent, context: ExtensionContext): void =>
      extension.#sessionStarted(context),
    );
    pi.on("agent_start", (_event: AgentStartEvent, context: ExtensionContext): void =>
      extension.#runStarted(context),
    );
    pi.on("tool_execution_start", (event: Extract<ExtensionEvent, { type: "tool_execution_start" }>, context: ExtensionContext): void =>
      extension.#toolStarted(event, context),
    );
    pi.on("tool_execution_end", (event: Extract<ExtensionEvent, { type: "tool_execution_end" }>, context: ExtensionContext): void =>
      extension.#toolEnded(event, context),
    );
    pi.on("agent_end", (_event: AgentEndEvent, context: ExtensionContext): void =>
      extension.#runEnded(context),
    );
    pi.on("session_shutdown", (event: SessionShutdownEvent, context: ExtensionContext): void =>
      extension.#sessionEnded(event, context),
    );
  }

  static #identifier(value: string): string {
    const safe: string = value.replace(SwarxPiExtension.UNSAFE_CHARACTERS, SwarxPiExtension.SAFE_REPLACEMENT);
    return safe.slice(0, SwarxPiExtension.IDENTIFIER_LIMIT);
  }

  #sessionStarted(context: ExtensionContext): void {
    this.#report(context, { hook_event_name: SwarxPiExtension.SESSION_START });
  }

  #runStarted(context: ExtensionContext): void {
    this.#run = `${SwarxPiExtension.RUN_PREFIX}${context.sessionManager.getEntries().length}`;
    this.#report(context, this.#runFields(SwarxPiExtension.USER_PROMPT_SUBMIT));
  }

  #toolStarted(event: Extract<ExtensionEvent, { type: "tool_execution_start" }>, context: ExtensionContext): void {
    this.#report(context, this.#toolFields(SwarxPiExtension.PRE_TOOL_USE, event.toolName, event.toolCallId));
  }

  #toolEnded(event: Extract<ExtensionEvent, { type: "tool_execution_end" }>, context: ExtensionContext): void {
    const name: string = event.isError ? SwarxPiExtension.POST_TOOL_USE_FAILURE : SwarxPiExtension.POST_TOOL_USE;
    this.#report(context, this.#toolFields(name, event.toolName, event.toolCallId));
  }

  #runEnded(context: ExtensionContext): void {
    this.#report(context, this.#runFields(SwarxPiExtension.STOP));
    this.#run = undefined;
  }

  #sessionEnded(event: SessionShutdownEvent, context: ExtensionContext): void {
    if (event.reason === SwarxPiExtension.RELOAD_REASON) {
      return;
    }
    this.#report(context, { hook_event_name: SwarxPiExtension.SESSION_END });
  }

  #runFields(name: string): Record<string, string | undefined> {
    return { hook_event_name: name, prompt_id: this.#run };
  }

  #toolFields(name: string, tool: string, call: string): Record<string, string | undefined> {
    return { ...this.#runFields(name), tool_name: tool, tool_use_id: SwarxPiExtension.#identifier(call) };
  }

  #report(context: ExtensionContext, fields: Record<string, string | undefined>): void {
    const payload: string = JSON.stringify({ ...fields, session_id: context.sessionManager.getSessionId() });
    this.#queue = this.#queue.then((): Promise<void> => this.#deliver(payload));
  }

  #deliver(payload: string): Promise<void> {
    return new Promise<void>((resolve: () => void): void => this.#start(payload, resolve));
  }

  #start(payload: string, resolve: () => void): void {
    const timer: NodeJS.Timeout = setTimeout(resolve, SwarxPiExtension.DELIVERY_LIMIT_MILLISECONDS);
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
    const child: ChildProcess = spawn(this.#relay, [SwarxPiExtension.HOOK_FLAG], {
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

export default SwarxPiExtension.install;
