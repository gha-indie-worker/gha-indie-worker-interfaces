<!-- generated-policy: frozen -->

# Generated files — read-only

Do **not** hand-edit files in this directory. They are produced by tooling such as:

- https://github.com/flags-2-env/flags-2-env (typical Dart path: `generated/dart/env.dart`)
- https://github.com/oresoftware/api-docs
- JSON Schema / OpenAPI / route-map generators in this repository

## Disk permissions

After generation, files here are frozen with `chmod a-w` (not writable). Directories
and this `README.md` stay writable so generators can replace files.

Git does **not** persist the write bit (only the executable bit). A fresh clone is
writable until you re-freeze:

```sh
find generated -type f ! -name 'README.md' ! -name 'readme.md' -exec chmod a-w {} +
```

To regenerate, change the **primary source** (`.cli-flags.toml`, route map, OpenAPI,
`schema/*.schema.json`, …) and re-run the generator. Preferred generators thaw,
write, then `chmod a-w` themselves.

## Gitignored trees

If `generated/` is in `.gitignore`, generated artifacts stay off VCS. Still commit
this `README.md` (`git add -f generated/README.md` or a `.gitignore` exception) so
the freeze policy is visible. Example exception:

```
generated/**
!generated/README.md
```

## Runtime contract (not just compile-time)

JSON Schema is a **cross-check**, not always the primary generator input. Unit tests
should validate fixtures/examples against Draft 2020-12 at runtime (valid must pass,
invalid must fail) and compare schema keys to `.cli-flags.toml` env names or
route-map keys when those exist.

## ores-contracts output (`generated/<slice>/`)

Since the `contracts/` adoption, the bulk of this directory is written by
[`ores-contracts`](https://github.com/ORESoftware/ores-contracts) from the **two
independent authorities** in `contracts/`:

```sh
npm run contracts:generate        # node scripts/contracts-slices.mjs generate
```

That command runs the toolkit once per slice. For each slice it parses
`contracts/typespec/<slice>.tsp` and `contracts/json-schema/<slice>.schema.json`
*separately*, emits every artifact from each parse, and byte-compares the two
lanes. Nothing is written unless the lanes agree; a disagreement is a finding in
`target/ores-contracts/<slice>/receipt.json` and the run stops for human
evaluation. **Neither authority is generated from the other, and neither is
generated from `src/v1/`.**

Per slice you get:

```text
generated/<slice>/sql/schema.sql            # DDL input for declarative-migrations / dpm
generated/<slice>/rust/types.rs             # serde types
generated/<slice>/seaorm/entities.rs        # code-first ORM entities
generated/<slice>/diesel/schema.rs          # db-first mirror; `diesel print-schema` must equal it
generated/<slice>/typescript/types.d.ts
generated/<slice>/typescript/validate.mjs
generated/<slice>/dart/models.dart
generated/<slice>/receipt.json              # the parity receipt for this generation
```

The runner lane — not a developer's laptop — runs `npm run contracts:generate`
and commits the result, and `.github/workflows/contracts.yml` re-runs it and
fails on `git diff --exit-code -- generated`. So the rule above still holds:
**do not hand-edit anything here.** Change an authority in `contracts/` instead.
