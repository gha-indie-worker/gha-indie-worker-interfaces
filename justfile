# gha-indie-worker-interfaces
#
# `just lint` is the whole fleet lint lane in one command; `just contracts` is the
# parity gate. Both are exactly what CI runs, so a green terminal means a green PR.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

# ---------------------------------------------------------------------------
# Contracts
# ---------------------------------------------------------------------------

# The parity gate: both authorities, every fixture, the proto, fail closed.
contracts:
    node scripts/parity.mjs

# The gate minus the lanes that need a TypeSpec/npm toolchain. Prints a loud
# banner naming every lane that did not run -- never use it to claim parity.
contracts-offline:
    node scripts/parity.mjs --offline

# Fixtures against the hand-authored authority only. Fast; runs anywhere Python does.
fixtures:
    python3 scripts/check_fixtures.py

# ---------------------------------------------------------------------------
# Lint -- one recipe, every language in the repo
# ---------------------------------------------------------------------------

lint: lint-rust lint-ts lint-dart lint-schema lint-tsp lint-yaml lint-shell
    @echo "[lint] all lanes passed"

# rustfmt settles formatting; clippy -D warnings makes a lint a build failure;
# `[lints] unsafe_code = "forbid"` in Cargo.toml means an interfaces crate cannot
# reach for unsafe at all. deny + audit are the supply chain half.
lint-rust:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo deny check
    cargo audit

# Biome is one binary doing lint AND format in one pass -- it replaces
# ESLint+Prettier and their plugin graph, and it is fast enough to run on save.
# It has no type information, so `tsc --noEmit` is the other half of the lane.
lint-ts:
    npx --no-install biome ci .
    npx --no-install tsc --noEmit

# package:lints/recommended is the set the Dart team ships and keeps current, so
# this tracks upstream rather than a private list that rots.
lint-dart:
    cd generated/dart && dart analyze --fatal-infos

# ajv strict-mode compile: catches a keyword that enforces nothing where it sits,
# a dangling $ref and an unknown format -- constraints a reviewer believes exist.
lint-schema:
    node scripts/lint-schemas.mjs

# `tsp format --check` settles TypeSpec formatting; `--warn-as-error` means a
# warning in an authority is a failed build, not a line in a log nobody reads.
lint-tsp:
    npx --no-install tsp format --check "contracts/typespec/**/*.tsp"
    npx --no-install tsp compile contracts/typespec/main.tsp --no-emit --warn-as-error

# actionlint reads workflow semantics (bad `needs:`, an expression that cannot be
# true, a `run:` shell that will not parse); yamllint reads the file as YAML.
# Disjoint failure sets, so both.
lint-yaml:
    actionlint
    yamllint .github/ .yamllint.yaml analysis_options.yaml

# shellcheck on every script that is actually shell.
lint-shell:
    #!/usr/bin/env bash
    set -euo pipefail
    mapfile -t files < <(git ls-files '*.sh' 'scripts/*' | xargs -r file --mime-type | awk -F: '/x-shellscript/ {print $1}')
    if [ ${#files[@]} -eq 0 ]; then echo "[lint] no shell scripts"; else shellcheck "${files[@]}"; fi

# Rewrite what can be rewritten. Never run by CI -- CI only ever checks.
fmt:
    cargo fmt --all
    npx --no-install biome format --write .
    npx --no-install tsp format "contracts/typespec/**/*.tsp"
    cd generated/dart && dart format .

# Everything a PR is judged on.
ci: lint contracts
