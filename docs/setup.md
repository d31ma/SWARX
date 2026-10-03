# Source setup

Use a source checkout on macOS. No Windows or Linux build or runtime result is
claimed for this preparation.

## Prerequisites

- A qualified Tachyon compiler supporting the authored Service roles,
  required-provider injection, application-owned collector lifetime and absolute
  `--out-dir` output routing.
- Node.js with the type-stripping support used by the build tools.
- TypeScript `7.0.2`, as declared in `app/package.json`, available as `tsc`.
- Rust and Cargo, plus Apple's Swift compiler and macOS command-line tools.

Compiler binaries, installed dependency trees and build outputs are external
prerequisites. They are not included in the public source inventory. Do not
substitute an arbitrary Tachyon release for the compiler contract required by
this source.

There is no JavaScript dependency lockfile in the migrated source. The TypeScript
version pin alone does not establish a frozen reproducible dependency installation.
Matching dependency metadata must be recovered or reviewed before a reproducible
installation procedure can be published. This preparation installed nothing.

## External build and launch paths

Set these environment variables to absolute paths outside the source checkout
before running `./build.sh` from the project root. The path check also resolves
existing ancestors to reject a symlink route back into the source checkout.
There are no machine-specific fallback paths. Choose designated nonsynced
build and cache directories: the guard checks that a path is outside this
checkout, not whether it is cloud-synchronized.

| Variable | Required value |
| --- | --- |
| `TACHYON_COMPILER` | An executable qualified Tachyon compiler. |
| `OBSERVATORY_NATIVE_ARTIFACTS` | External native output root, supplied to the compiler with `--out-dir`. |
| `OBSERVATORY_RUST_CACHE_PATH` | External Cargo build/cache root. |
| `OBSERVATORY_SWIFT_CACHE_PATH` | External Swift and Clang module-cache root. |
| `OBSERVATORY_ROLLBACK_ROOT` | External directory for preserving an existing app before rebuilding it. |

`CARGO_TARGET_DIR`, when supplied, is also honored and checked to remain outside
source. A relative value resolves from the project root; an absolute value is
used directly. When unset, Cargo output uses `target/` under the explicitly
configured `OBSERVATORY_RUST_CACHE_PATH`.

The build composes the Tachyon output, Rust companion, thin native host and
application icon. Its current native bundle is
`macos/AgentObservatory.app` under `OBSERVATORY_NATIVE_ARTIFACTS`; native Swarx
branding is pending. Generated project inputs also remain under that external
root. The output route has source and fixture checks; this preparation did not
perform a new native build or launch.

For `./run-local.sh`, set `OBSERVATORY_APP_BUNDLE` to an existing external `.app`
directory. The launch entrypoint opens that bundle; it does not build one.

Installed dependencies, compiler binaries, caches, native bundles and all build,
test and preview artifacts must remain outside the source checkout permanently
and outside the public source inventory. Keep application state and opt-in event
records separate from build artifacts. A local artifact-routing launcher is a
machine setup detail; no personal launcher path belongs in portable source.

## Validation boundary

Source checks and native acceptance have separate prerequisites. Live QA requires
an external `OBSERVATORY_QA_ROOT` with a pre-created `evidence/` directory and an
existing `OBSERVATORY_APP_BUNDLE`. Explicit report fields are
`OBSERVATORY_QA_SESSION_ID`, `OBSERVATORY_QA_AGENT_ID`, `OBSERVATORY_QA_FAMILY`,
`OBSERVATORY_QA_SESSION_LABEL`, `OBSERVATORY_QA_TASK_ID`,
`OBSERVATORY_QA_TASK_NAME` and `OBSERVATORY_QA_ACTION_NAME`. Supply only real,
caller-owned context for a live check. The native lifetime check additionally
requires `OBSERVATORY_LIFETIME_EVENT_FILE`, an external JSON file containing a
real caller-owned report.

Offline browser QA requires explicitly selected external dependencies and
fixtures: `OBSERVATORY_PLAYWRIGHT_MODULE` (ESM entry),
`OBSERVATORY_BROWSER_EXECUTABLE`, `OBSERVATORY_WEB_BUNDLE`,
`OBSERVATORY_BUILD_SOURCE` (qualified source-hash manifest),
`OBSERVATORY_OVERVIEW_FIXTURE` (Rust fixture executable) and
`OBSERVATORY_QA_ROOT`. These generated artifacts are not included with public
source. No dependency installation or new native/browser runtime result is
claimed by this documentation.

A future native acceptance must check the renamed bundle, ordinary and small
window layouts, actual browser dependency resolution, polling focus/scroll,
charts, Help navigation and normal Quit. A source typecheck or fixture result
does not replace that acceptance. No web/HTTP target is part of this desktop
setup.

## Local data

Task events are explicit opt-in reports. Process links require a verified
process identity; resources without a proven session link remain separate.
Event adapters must send safe, already-sanitized action labels through the
supported private local boundary and retain source, stale and unavailable labels.
Do not send credentials, prompts, transcripts or terminal output. Supported
redaction is bounded; it does not guarantee removal of every secret format.
This document changes no storage path, IPC identifier or application-owned
collector lifecycle.
