#!/bin/zsh
set -euo pipefail

main() {
  local project_root="$1"
  local app_path
  app_path="$(node --experimental-strip-types --input-type=module -e 'import {ToolPathUtils} from "./tools/utils/ToolPathUtils.ts"; process.stdout.write(ToolPathUtils.external(process.argv[1], "OBSERVATORY_APP_BUNDLE"));' "$project_root")"
  [[ -d "$app_path" && "$app_path" == *.app ]] || { print -u2 'OBSERVATORY_APP_BUNDLE must name an existing external .app.'; return 1; }
  /usr/bin/open "$app_path"
}

cd -- "$(dirname -- "$0")"
main "$PWD" "$@"
