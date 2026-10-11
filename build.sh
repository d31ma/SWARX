#!/bin/zsh
set -euo pipefail

_external() {
  node --experimental-strip-types --input-type=module -e 'import {ToolPathUtils} from "./tools/utils/ToolPathUtils.ts"; process.stdout.write(ToolPathUtils.external(process.argv[1], process.argv[2]));' "$1" "$2"
}

_preserve_verified_app() {
  local app_path="$1/macos/Swarx.app"
  local backup_root="$2"
  local backup_stamp
  backup_stamp="$(/bin/date -u +%Y%m%dT%H%M%SZ)-$$"
  if [[ -d "$app_path" ]]; then
    mkdir -p "$backup_root/$backup_stamp"
    /bin/cp -R "$app_path" "$backup_root/$backup_stamp/Swarx.app"
    print "Previous app preserved at $backup_root/$backup_stamp/Swarx.app"
  fi
}

_target() {
  if [[ "$#" == 0 ]]; then
    print macos
    return
  fi
  [[ "$#" == 2 && "$1" == --target ]] || { print -u2 'Usage: ./build.sh [--target macos|linux|windows]'; return 1; }
  case "$2" in
    macos|linux|windows) print "$2" ;;
    *) print -u2 'Native target must be macos, linux or windows.'; return 1 ;;
  esac
}

_prepare_target() {
  if [[ "$1" == macos ]]; then
    _preserve_verified_app "$2" "$3"
  fi
}

_finish_target() {
  if [[ "$1" == macos ]]; then
    node --experimental-strip-types tools/NativeHostCustomizer.ts --root "$2"
    node --experimental-strip-types tools/IconPackager.ts --root "$2"
  fi
}

main() {
  local project_root="$1"
  shift
  local target
  target="$(_target "$@")"
  cd "$project_root"
  local compiler_path artifact_path cache_path rust_cache backup_root
  compiler_path="$(_external "$project_root" TACHYON_COMPILER)"
  artifact_path="$(_external "$project_root" OBSERVATORY_NATIVE_ARTIFACTS)"
  cache_path="$(_external "$project_root" OBSERVATORY_SWIFT_CACHE_PATH)"
  rust_cache="$(_external "$project_root" OBSERVATORY_RUST_CACHE_PATH)"
  backup_root="$(_external "$project_root" OBSERVATORY_ROLLBACK_ROOT)"
  node --experimental-strip-types --input-type=module -e 'import {ToolPathUtils} from "./tools/utils/ToolPathUtils.ts"; ToolPathUtils.separate(process.argv.slice(1));' "$artifact_path" "$cache_path" "$rust_cache" "$backup_root"
  [[ -x "$compiler_path" ]] || { print -u2 'TACHYON_COMPILER must name an executable qualified compiler.'; return 1; }
  command -v tsc >/dev/null 2>&1 || { print -u2 'TypeScript compiler is missing. Build requires TypeScript 7.0.2; no installation was attempted.'; return 1; }
  [[ "$(tsc --version)" == 'Version 7.0.2' ]] || { print -u2 'Build requires TypeScript 7.0.2.'; return 1; }
  export TAC_CACHE_DIR="$rust_cache/tachyon"
  export CARGO_NET_OFFLINE=true
  export CARGO_BUILD_JOBS=2
  export OBSERVATORY_NATIVE_ARTIFACTS="$artifact_path"
  export OBSERVATORY_SWIFT_CACHE_PATH="$cache_path"
  export OBSERVATORY_RUST_CACHE_PATH="$rust_cache"
  export CLANG_MODULE_CACHE_PATH="$cache_path"
  export SWIFT_MODULECACHE_PATH="$cache_path"
  _prepare_target "$target" "$artifact_path" "$backup_root"
  "$compiler_path" bundle app --out-dir "$artifact_path" --target "$target"
  _finish_target "$target" "$project_root"
}

main "$(cd -- "$(dirname -- "$0")" && pwd)" "$@"
