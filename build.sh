#!/bin/zsh
set -euo pipefail

_external() {
  node --experimental-strip-types --input-type=module -e 'import {ToolPathUtils} from "./tools/utils/ToolPathUtils.ts"; process.stdout.write(ToolPathUtils.external(process.argv[1], process.argv[2]));' "$1" "$2"
}

_preserve_verified_app() {
  local app_path="$1/macos/AgentObservatory.app"
  local backup_root="$2"
  local backup_stamp
  backup_stamp="$(/bin/date -u +%Y%m%dT%H%M%SZ)-$$"
  if [[ -d "$app_path" ]]; then
    mkdir -p "$backup_root/$backup_stamp"
    /bin/cp -R "$app_path" "$backup_root/$backup_stamp/AgentObservatory.app"
    print "Previous app preserved at $backup_root/$backup_stamp/AgentObservatory.app"
  fi
}

main() {
  local project_root="$1"
  cd "$project_root"
  local compiler_path artifact_path cache_path rust_cache backup_root
  compiler_path="$(_external "$project_root" TACHYON_COMPILER)"
  artifact_path="$(_external "$project_root" OBSERVATORY_NATIVE_ARTIFACTS)"
  cache_path="$(_external "$project_root" OBSERVATORY_SWIFT_CACHE_PATH)"
  rust_cache="$(_external "$project_root" OBSERVATORY_RUST_CACHE_PATH)"
  backup_root="$(_external "$project_root" OBSERVATORY_ROLLBACK_ROOT)"
  [[ -x "$compiler_path" ]] || { print -u2 'TACHYON_COMPILER must name an executable qualified compiler.'; return 1; }
  command -v tsc >/dev/null 2>&1 || { print -u2 'TypeScript compiler is missing. Build requires TypeScript 7.0.2; no installation was attempted.'; return 1; }
  export OBSERVATORY_NATIVE_ARTIFACTS="$artifact_path"
  export OBSERVATORY_SWIFT_CACHE_PATH="$cache_path"
  export OBSERVATORY_RUST_CACHE_PATH="$rust_cache"
  export CLANG_MODULE_CACHE_PATH="$cache_path"
  export SWIFT_MODULECACHE_PATH="$cache_path"
  _preserve_verified_app "$artifact_path" "$backup_root"
  "$compiler_path" bundle app --out-dir "$artifact_path" --target macos --native
  node --experimental-strip-types tools/NativeRustPackager.ts --root "$project_root"
  node --experimental-strip-types tools/NativeHostCustomizer.ts --root "$project_root"
  node --experimental-strip-types tools/IconPackager.ts --root "$project_root"
}

main "$(cd -- "$(dirname -- "$0")" && pwd)" "$@"
