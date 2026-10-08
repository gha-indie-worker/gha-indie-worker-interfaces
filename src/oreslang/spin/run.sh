#!/usr/bin/env bash
set -euo pipefail
# Invoke: ORESLANG_SPIN_DIR=... ORESLANG_SOURCE_DIR=... bash src/oreslang/spin/run.sh shared|isolated
mode="${1:-shared}"
if [[ "$mode" != "shared" && "$mode" != "isolated" ]]; then
  echo "usage: $0 shared|isolated" >&2
  exit 64
fi
: "${ORESLANG_SPIN_DIR:?Set ORESLANG_SPIN_DIR to a local oreslang-spin checkout}"
: "${ORESLANG_SOURCE_DIR:?Set ORESLANG_SOURCE_DIR to the pinned oreslang-source.java checkout}"
spin_dir="$(cd "$ORESLANG_SPIN_DIR" && pwd -P)"
source_dir="$(cd "$ORESLANG_SOURCE_DIR" && pwd -P)"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
expected_spin_sha="8f29b2f75f13beb3e03357e6b15839129c33d72b"
actual_spin_sha="$(git -C "$spin_dir" rev-parse HEAD)"
if [[ "$actual_spin_sha" != "$expected_spin_sha" ]]; then
  echo "Spin checkout mismatch: expected $expected_spin_sha, got $actual_spin_sha" >&2
  exit 65
fi
expected_compiler_sha="$(tr -d '[:space:]' < "$spin_dir/SOURCE_REF")"
actual_compiler_sha="$(git -C "$source_dir" rev-parse HEAD)"
if [[ "$actual_compiler_sha" != "$expected_compiler_sha" ]]; then
  echo "Compiler checkout mismatch: expected $expected_compiler_sha, got $actual_compiler_sha" >&2
  exit 65
fi
if [[ "${PORT:-8080}" != "8080" || "${HOST:-127.0.0.1}" != "127.0.0.1" ]]; then
  echo "Spin draft adapter currently supports HOST=127.0.0.1 and PORT=8080 only" >&2
  exit 64
fi
if [[ ! -f "$spin_dir/dependencies/http-routing/src/http_routing.ores" ]]; then
  echo "Initialize the pinned Spin http-routing submodule first" >&2
  exit 66
fi
if [[ ! -f "$source_dir/target/classpath.txt" ]]; then
  echo "Build the pinned compiler with Maven as described in oreslang-spin/README.md" >&2
  exit 66
fi
stage="$spin_dir/examples/benchmark-${mode}.ores"
if [[ -e "$stage" ]]; then
  echo "Refusing to overwrite existing file: $stage" >&2
  exit 73
fi
cp "$script_dir/server_${mode}.ores" "$stage"
cleanup() { rm -f "$stage"; }
trap cleanup EXIT
(
  cd "$spin_dir"
  "$spin_dir/scripts/compiler.sh" --platform=server --allow-net=127.0.0.1:8080 "examples/benchmark-${mode}.ores"
)
