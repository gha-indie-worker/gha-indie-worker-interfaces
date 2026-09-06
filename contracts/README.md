# contracts/

**TypeSpec and JSON Schema are two independent, peer, human-authored sources of
truth.** Neither is generated from the other. Neither is canonical. Each one is
written by a person, each one independently drives code, validation and SQL, and
the artifacts they produce are then **compared as evidence**. Any discrepancy
**fails closed** for a human to resolve.

This is the same policy `ORESoftware/ores-middleware` states and
`@oresoftware/ores-contracts` implements for the persisted subset. This directory
follows it, reuses that toolkit rather than duplicating it, and extends the same
idea to the wire types it does not cover.

```
contracts/typespec/*.tsp    ──authored by a human──┐
                                                   ├──▶ compared ──▶ findings ──▶ STOPPED_FOR_EVALUATION
contracts/json-schema/*.json ─authored by a human──┘

contracts/proto/*.proto     ──runtime validation of the binary transport,
                              checked against BOTH authorities, never a tiebreak
```

## Why two

A single source of truth has one failure mode nobody notices: it is wrong, and
everything downstream is consistently wrong with it. Generation makes that worse,
because a generated artifact agreeing with its generator is not evidence of
anything — it agrees by construction.

Two independently authored descriptions cannot be wrong in the same way by
accident. When a person writes `maxLength: 39` in one and `@maxLength(39)` in the
other, agreement is a real measurement. When they write `39` and `38`, the gate
says so and stops. That is the entire design.

The corollary is the rule everything else follows from: **the gate never
reconciles.** There is no "prefer TypeSpec", no auto-fix, no flag that copies one
side onto the other. `scripts/parity.mjs` reports and exits non-zero; a human
edits an authored file.

## What happens when they disagree

1. The gate exits non-zero and prints **every** discrepancy, each with a stable
   16-hex fingerprint so the same disagreement is recognisable across runs.
2. `target/parity/receipt.json` records both authorities' hashes, every lane's
   status and every finding. CI uploads it as an artifact.
3. `ores-contracts generate` refuses to write `generated/`. The last agreed
   artifacts stay on disk; nothing half-agreed is ever committed.
4. A human decides **which authority was wrong** and edits it. Both files are in
   the diff, so the reviewer sees the decision, not just the fix.

Nothing in the repository can be made to pass by editing `target/`. It is
deleted at the start of every run.

## The two authorities, file by file

| authority | file | peer | what it covers |
|---|---|---|---|
| TypeSpec | `typespec/main.tsp` | `json-schema/contract.schema.json` | persisted models: identity, tenancy, CI/CD, embedding spaces, findings |
| TypeSpec | `typespec/session.tsp` | `json-schema/session.schema.json` | the live session protocol — the `Frame` union |
| TypeSpec | `typespec/http.tsp` | `json-schema/errors.schema.json` | the error model, pagination envelope, principal, onboarding failures |
| TypeSpec | `typespec/discovery.tsp` | `json-schema/discovery.schema.json` | search, neighbours, the evidence required to enable a space |
| TypeSpec | `typespec/lease.tsp` | `json-schema/lease.schema.json` | `WorkerLease`, `Health`, `LeaseErrorCode` — the v1 types this repo already publishes |
| protobuf | `proto/session.proto` | *(both, as a check)* | runtime validation of the binary session transport |
| protobuf | `proto/events.proto` | *(both, as a check)* | run and job lifecycle events carried in `EventFrame.payload` |

The vocabulary in both authorities is the one `gha-indie-worker-lib-core`
already ships on `main`: `Role`, `Seats`, `Invitation`, `InvitationState`,
`JoinPolicy`, `OnboardingError`, `Frame`, `ErrorCode`, `Surface`, `Realm`,
`Audience`, `ComparisonSpace`, `AnnStrategy`, `Metric` and the embedding
`Role`. Every enum's string values are the Rust `as_str()` values, so a
serialized value from `runtime::` is a value these contracts accept.

## What is deliberately *not* symmetric

Two things are asymmetric, both because the formats genuinely differ, and both
are declared in `parity.config.json` with a written reason the gate checks is
still current:

- **HTTP bindings live only in TypeSpec.** Routes, verbs, status codes and query
  parameters are not part of any JSON document, so a JSON Schema has nothing to
  describe. Inventing a vendor keyword to fake a peer would make the parity claim
  weaker, not stronger — it would compare our invention against itself.
- **`Page<T>` and `ListQuery` are TypeSpec-only.** JSON Schema 2020-12 has no
  type parameters. A peer would mean four hand-copied envelopes that could drift
  from each other — strictly worse evidence than one generic with none. `PageInfo`,
  the non-generic half, *does* have a peer and is compared.

Integer **width** (`int32` vs `int64`) is not compared by the shape lane, because
JSON has one number type and the JSON wire shape cannot carry the distinction.
It is not dropped: it is checked in the `ores-contracts` lane, which reads
`int64` on one side and `x-ores-width: 64` on the other and byte-compares the SQL
and Rust each lane emits from them. The check is routed to the lane that can
settle it.

## The gate

```sh
just contracts                  # node scripts/parity.mjs — every lane
just contracts-offline          # skip tool-dependent lanes, loudly
just fixtures                   # fixtures vs the hand-authored authority only
```

| lane | what it proves | needs |
|---|---|---|
| `tsp-compile` | the TypeSpec authority compiles with `--warn-as-error` | `tsp` |
| `tsp-emit` | TypeSpec emits a JSON Schema into disposable `target/parity/` | `tsp` |
| `shape-emitted` | that emitted schema matches the hand-authored one, field by field | `tsp` |
| `shape-source` | the `.tsp` **source** matches the hand-authored one, field by field | — |
| `fixtures-js` | every fixture behaves as declared against the hand-authored schema | — |
| `fixtures-tsp` | the same corpus against the emitted schema; verdicts must agree | `tsp` |
| `proto` | proto JSON names, types, presence and enum values agree with **both** | — |
| `published` | `schema/v1/workerlease.json` still equals its `$defs` peer | — |
| `ores-contracts` | the persisted subset emits byte-identical SQL / SeaORM / Diesel / Rust / TS / Dart from each authority | `npm` |

`shape-source` and `shape-emitted` are two different mechanisms reading the same
file — a string-aware reader here, and the real compiler. If they ever disagree
about what a declaration means, that disagreement is itself a finding.

## The v1 lease keeps its published contract

`schema/v1/workerlease.json` is a standalone document this repository already
publishes at a stable `$id`, and `src/protocol.rs` already has `WorkerLease` and
`Health`. That is a type with exactly one description and no peer — the situation
the rest of this directory exists to fix — so `lease.tsp` and `lease.schema.json`
describe it too, and the `published` lane checks the published artifact and its
`$defs` peer have not drifted.

The published document is not edited and not replaced. Its bounds are copied into
the new authorities exactly as they stand: `id` and `revision` have `minLength: 1`
and **no maximum**, and inventing one would tighten a contract that consumers are
already coded against. A v1 change means a new `$id`, not an edit.

**A lane that cannot run is a finding.** Missing `tsp` fails the gate. `--offline`
downgrades it to a `SKIPPED` banner that names every lane that did not run, and
is never passed in CI — a gate that passes because it did not look is worse than
no gate.

## Fixtures are the strongest evidence

`fixtures/valid/<Model>/*.json` must be **accepted** and
`fixtures/invalid/<Model>/*.json` must be **rejected**, by *both* authorities.

The negative cases carry most of the weight. Two schemas agreeing that a
well-formed object is fine is weak: almost any pair of schemas agrees about a
valid document. Two schemas *independently rejecting the same malformed document,
for the same reason* means they actually encode the same constraint. A negative
fixture that only one authority rejects is reported as `fixture-disagreement` and
is exactly the discrepancy this repository exists to surface.

Every model in `fixtures/index.json` must have at least one of each, and every
`$defs` object must be in `fixtures/index.json` — so adding a model cannot skip
its evidence.

## Adding a model — touch both authorities plus a fixture, never one

Editing a single file is a **gate failure, not a shortcut**. The steps:

1. Write the model in `typespec/<file>.tsp`.
2. Write the peer in `json-schema/<file>.schema.json`, **from the requirement, not
   from step 1.** Copying the TypeSpec into JSON Schema mechanically produces two
   files that agree because one was transcribed from the other, which proves
   nothing. If you find yourself translating rather than describing, stop — write
   down what the field means and describe that.
   - `type: object`, `additionalProperties: false`, an explicit `required`
   - real `format`, `pattern`, `minLength`/`maxLength`, `minimum`/`maximum`
   - `x-ores-table` / `x-ores-primary-key` / `x-ores-unique` / `x-ores-indexes` /
     `x-ores-references` when the model is persisted (see `ores-contracts`'
     `docs/subset.md`)
3. Add the model to `fixtures/index.json`.
4. Add at least one positive fixture and at least one negative fixture per
   constraint you care about. A negative that both authorities reject is the
   evidence; a positive alone is not.
5. If a binary wire type carries the model, add the message to `proto/` and bind
   it in `parity.config.json`.
6. `just contracts`. Fix what it names by editing an authored file.

## Layout

```
contracts/
  typespec/        main.tsp session.tsp http.tsp discovery.tsp lease.tsp   authority A
                   ores.tsp ores-decorators.js   (vendored persistence decorators)
  json-schema/     contract/session/errors/discovery/lease .schema.json   authority B
  proto/           session.proto events.proto      runtime validation; see proto/README.md
  fixtures/        index.json + valid/<Model>/ + invalid/<Model>/
  parity.config.json                               what is compared, and every declared asymmetry
```

`target/` is disposable, git-ignored, and deleted at the start of every gate run.
Nothing is ever authored there and nothing is ever read back from it into a
source file.
