#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$dir"
if [[ ! -f "mix.lock" ]]; then
  echo "Missing mix.lock: run mix deps.get and commit tested lockfile for reproducible runs" >&2
  exit 66
fi
exec mix run --no-halt
