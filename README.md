# Swarx

Swarx is a local desktop monitor for explicitly reported agents, verified sessions,
assigned tasks and measured process resources. It uses Tachyon, a Rust native
core and a JavaScript browser interface. macOS is the first target; Windows and
Linux runtime validation is pending.

Reported activity and local measurements are separate. Unknown ownership stays
unknown. Shared desktop processes and remote model work are not divided into
invented per-agent resource totals. Missing, denied, stale and reset measurements
keep their availability labels.

Swarx monitors agent work that runs on this machine. Cloud agent sessions, such as Codex
desktop cloud threads, run remotely and send no local events, so they do not appear; an open
desktop client by itself is not counted as active work.

## Local Codex sessions

Local Codex CLI sessions report through `observatory-event --codex-hook`. The
Codex thread ID becomes the agent and session ID unchanged, and the turn ID
becomes the task. With only the `SessionStart`, `PreToolUse` and `Stop` hooks
trusted in Codex:

- Each tool call marks the thread active and shows as a current action until
  the turn's `Stop`. Without `PostToolUse`, every call in a turn stays running
  until `Stop`, so a turn with several commands shows all of them as current.
  No finish time, duration or exit code is invented; `Stop` closes the calls as
  "no finish".
- A call longer than 120 seconds with no further hook goes stale.
- Codex sends no background flag and Swarx has no Codex harness process link,
  so long-running or yielded commands are never shown as background tasks.
  Swarx does not infer them from the shared Codex process.
- A sub-agent tool call that carries `agent_id` appears as its own agent under
  the `session_id` Codex reports (not yet probed live). Sub-agent start and stop are not reported until the user trusts
  `SubagentStart` and `SubagentStop` in Codex. If `PostToolUse` is trusted and
  Codex sends it when a long command yields, that command shows as completed
  while its process still runs; this has not been probed.
- A live local Codex CLI run (2026-10-11) confirmed that `PreToolUse` reports
  the shell tool as `Bash` with the command in `tool_input.command`, which
  becomes the action detail. In that run (one `codex app-server` process, two
  turns) `SessionStart` fired only at the first turn, so the later turn was
  first seen at its first tool call.
- Without `UserPromptSubmit`, nothing marks a turn start, so a turn with no tool
  call should not appear (inferred from the hook set, not observed).
- Without `SessionEnd`, Swarx gets no event when a Codex thread's process exits;
  the card simply stays idle after the last `Stop`. Without a process link, its
  Resources and Process Tree stay empty. The session is named by its thread ID
  because Codex hooks send no title.

The application keeps measurements and opt-in task events local. The collectors
do not read prompts, session transcripts or terminal output. Event adapters must
provide safe, already-sanitized action labels. Supported redaction covers bounded
patterns and does not guarantee removal of every secret format. Task visibility
requires an explicit supported event source; this source does not establish an
automatic API for every desktop harness.

The application name is Swarx. The native bundle is `Swarx.app` and its
executable is `Swarx`. The bundle ID `ma.del.agent-observatory`, the private
event socket under `/tmp/agent-observatory-<uid>`, the `agent-observatory`
storage name and the `observatory-event` helper are unchanged, so existing app
data carries over.

After installing this build, point every installed hook command at
`.../Swarx.app/Contents/MacOS/observatory-event`, including any
`~/.local/bin/swarx-hook` relay used by the OpenCode and Pi harnesses. Commands
that still name `AgentObservatory.app` stop working once the old bundle is
removed.

See [setup](docs/setup.md) for prerequisites and the current build boundary, and
[third-party notices](docs/third-party-notices.md) for bundled dependencies.
Application code is under `app/client/`; build and test tooling is separate.
Only an explicit reviewed source inventory is eligible for publication. Runtime
records, screenshots, caches, compiler binaries and internal validation history
are excluded.

No project-wide source license has been selected or granted by this preparation.
Bundled third-party license texts retain their own terms.
