#!/bin/sh
# One entry point for every linter this repository runs.
#
#   .ores-lint/lint.sh            # everything
#   .ores-lint/lint.sh rust       # rustfmt + clippy only
#   .ores-lint/lint.sh contracts  # tsp compile + ores-contracts + fixtures
#   .ores-lint/lint.sh js         # eslint + markdownlint
#
# cargo fmt and cargo clippy only read a config from the crate root, so the
# canonical copies in .ores-lint/ are mirrored to ./rustfmt.toml and
# ./clippy.toml. Drift between the pair is itself a lint failure.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
cd "$here"
what=${1:-all}

mirror_check() {
  for f in rustfmt.toml clippy.toml; do
    if ! diff -q ".ores-lint/$f" "$f" >/dev/null 2>&1; then
      echo "[lint] $f differs from .ores-lint/$f — copy the canonical file over it" >&2
      exit 1
    fi
  done
}

if [ "$what" = all ] || [ "$what" = rust ]; then
  mirror_check
  echo "[lint] cargo fmt"
  cargo fmt --all -- --check
  echo "[lint] cargo clippy"
  cargo clippy --locked --all-targets -- -D warnings
fi

if [ "$what" = all ] || [ "$what" = contracts ]; then
  echo "[lint] tsp compile"
  npx --no-install tsp compile contracts/typespec --no-emit
  echo "[lint] ores-contracts (every slice)"
  node scripts/contracts-slices.mjs check
  echo "[lint] fixtures (both validator engines)"
  node scripts/validate-fixtures.mjs --engine builtin
  node scripts/validate-fixtures.mjs
fi

if [ "$what" = all ] || [ "$what" = js ]; then
  echo "[lint] eslint"
  npx --no-install eslint --config .ores-lint/eslint.config.mjs scripts
  echo "[lint] markdownlint"
  npx --no-install markdownlint-cli2 --config .ores-lint/.markdownlint.jsonc "**/*.md" "!node_modules" "!generated"
fi

echo "[lint] ok"
