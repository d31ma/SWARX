#!/bin/zsh
set -euo pipefail

main() {
  local project_root
  project_root="$(cd -- "$(dirname -- "$0")" && pwd)"
  cd "$project_root"
  exec ./run-local.sh --native
}

main "$@"
