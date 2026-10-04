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

The application keeps measurements and opt-in task events local. The collectors
do not read prompts, session transcripts or terminal output. Event adapters must
provide safe, already-sanitized action labels. Supported redaction covers bounded
patterns and does not guarantee removal of every secret format. Task visibility
requires an explicit supported event source; this source does not establish an
automatic API for every desktop harness.

The source folder is `SWARX`, and the chosen display name is Swarx. Native
application labels, bundle names and internal identifiers still use Agent
Observatory. A native rename and acceptance check are pending; publication
preparation does not establish a renamed executable release.

See [setup](docs/setup.md) for prerequisites and the current build boundary, and
[third-party notices](docs/third-party-notices.md) for bundled dependencies.
Application code is under `app/client/`; build and test tooling is separate.
Only an explicit reviewed source inventory is eligible for publication. Runtime
records, screenshots, caches, compiler binaries and internal validation history
are excluded.

No project-wide source license has been selected or granted by this preparation.
Bundled third-party license texts retain their own terms.
