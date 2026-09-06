# `contracts/` — two independent authorities

**TypeSpec and JSON Schema are peers. Neither is generated from the other, and
neither is generated from the Rust in `src/v1/`.** Each is hand-authored, each is
parsed on its own into a normalized IR, each IR drives the same deterministic
emitters, and the results must agree byte for byte. A discrepancy is a finding
with a stable fingerprint and `generate` refuses to write `generated/` until a
human changes an authored source.

```text
contracts/typespec/<slice>.tsp ─────parse──▶ IR_T ──emit──▶ SQL SeaORM Diesel Rust TS Dart
contracts/json-schema/<slice>.schema.json ──▶ IR_J ──emit──▶ SQL SeaORM Diesel Rust TS Dart
                                              │                ║ byte parity + receipt.json ║
                                              └── structural diff ──▶ findings → STOPPED_FOR_EVALUATION
```

Toolkit: [`ORESoftware/ores-contracts`](https://github.com/ORESoftware/ores-contracts).

## Layout

| path | what it is |
|---|---|
| `contracts/typespec/<slice>.tsp` | authority A, hand-authored |
| `contracts/typespec/ores.tsp` | vendored decorator declarations (`@Ores.table`, `@Ores.unique`, `@Ores.index`, `@Ores.references`) plus the `uuid` scalar and the `json` alias |
| `contracts/typespec/ores-decorators.js` | the JS runtime those `extern dec` declarations need so `tsp compile` accepts them |
| `contracts/typespec/main.tsp` | import index, so `tsp compile contracts/typespec --no-emit` type-checks every slice at once |
| `contracts/json-schema/<slice>.schema.json` | authority B, hand-authored, draft 2020-12 |
| `contracts/config/<slice>.config.json` | one ores-contracts config per slice |
| `contracts/fixtures/<slice>/` | worked examples, see below |
| `../contracts.config.json` | the slice index, plus the default single-slice target for a bare `npx ores-contracts check` |
| `../generated/<slice>/` | machine output — never hand-edited, see `generated/README.md` |

The toolkit's parsers are deliberately single-file: they do not follow `import`.
That is why each slice has its own config and `main.tsp` is only for `tsp
compile`. `npm run contracts:check:all` sweeps all ten.

## Slices

| slice | namespace | what it owns |
|---|---|---|
| `identity` | `GhaIndieWorker.V1.Identity` | Org, User, OrgMember, Invitation, Seat, `Role` |
| `onboarding` | `GhaIndieWorker.V1.Onboarding` | the org and user onboarding state machines and their advance request/response |
| `runs` | `GhaIndieWorker.V1.Runs` | Plan, Run, Job, Step, LogChunk, RunCancellation, `RunStatus` |
| `workers` | `GhaIndieWorker.V1.Workers` | Worker, Capability, Heartbeat, Profile |
| `webhooks` | `GhaIndieWorker.V1.Webhooks` | GitHubDelivery envelope, RegistryImageEvent |
| `chat` | `GhaIndieWorker.V1.Chat` | ChatSession, ChatMessage, `ChatSurface` |
| `embeddings` | `GhaIndieWorker.V1.Embeddings` | ComparisonSpace and its twelve identity fields, EmbeddingRecord, index/search, RegressionFinding, AlertRule, MatchEvent |
| `sync` | `GhaIndieWorker.V1.Sync` | the opto-sync CausalEnvelope and per-node cursors |
| `transport` | `GhaIndieWorker.V1.Transport` | WsCommand, WsEvent, TcpFrame for the websocket and stateful-TCP avenues |
| `errors` | `GhaIndieWorker.V1.Errors` | RFC 9457 Problem Details |

## What the subset can express, and what it cannot

The parity IR is persistence-shaped (see the toolkit's `docs/subset.md`): scalars,
string enums, arrays of scalars/enums, a primary key, unique constraints, indexes
and foreign keys. Consequences worth knowing before editing an authority:

* **Every model carries `@Ores.table` / `x-ores-table` and a primary key.** For
  request/response and frame models that means an audit table — `ws_command_audit`,
  `onboarding_advance_requests`, `embedding_search_requests` — which is what the
  servers already write for replay and support triage.
* **There is no sum type.** Tagged unions are modelled as a sealed object with a
  `kind` discriminator. The JSON Schema authority adds a `oneOf` over `kind`
  consts as a runtime refinement; the TypeSpec authority states the same table in
  a comment above the model, and `src/v1/transport.rs` turns the flat frame into a
  real Rust enum with `TryFrom`.
* **There is no map type.** `vectorClock` is `json`; its runtime shape is pinned
  by `additionalProperties` in the JSON Schema authority.
* **Value bounds are not part of the IR.** `minLength`, `maxLength` (beyond the one
  the IR carries), `minimum`, `maximum`, `minItems`, `maxItems` and `pattern` are
  runtime refinements. Both authorities still state them — `@minValue` /
  `@maxItems` / `@pattern` in TypeSpec, the matching keywords in JSON Schema — so
  that "semantically identical" means identical to a reader, not only to the gate.
* **Foreign keys stay inside one slice**, because each slice is parsed alone.
  Cross-slice references are carried as a plain `uuid` and documented.

### Two syntax traps in the TypeSpec authority

The toolkit's TypeSpec parser is regex-based and fails closed. Inside a model body:

* no `{` or `}` — so `@pattern` must avoid `{n,m}` quantifiers (use `+`/`*` plus
  `@minLength`/`@maxLength`);
* no `)` inside a decorator argument, and no `//` inside a string.

`npm run contracts:check:all` catches both immediately.

## Fixtures

```text
contracts/fixtures/<slice>/valid/<Model>.<case>.json
contracts/fixtures/<slice>/invalid/<Model>.<case>.json
contracts/fixtures/<slice>/invalid/schema-only/<Model>.<case>.json
```

The filename prefix before the first `.` names the `$defs` entry. Two consumers
read this tree and they check different things:

| directory | `npm run validate:fixtures` (JSON Schema) | `cargo test` (serde) |
|---|---|---|
| `valid/` | must validate | must deserialize, and re-serialize to the identical JSON value |
| `invalid/` | must fail | must fail — these break a *structural* rule (missing required field, wrong scalar type, unknown enum value, extra property) |
| `invalid/schema-only/` | must fail | must **succeed** — these break only a value bound, which serde does not enforce |

`tests/fixtures_roundtrip.rs` asserts that split, so a fixture filed in the wrong
directory fails the build with a message telling you where it belongs.

## Commands

```sh
npm run contracts:check          # ores-contracts on the default slice
npm run contracts:check:all      # ores-contracts on all ten slices
npm run contracts:generate       # writes generated/<slice>/** when parity passes
npm run lint:typespec            # tsp compile contracts/typespec --no-emit
npm run validate:fixtures        # ajv when installed
npm run validate:fixtures:builtin # the dependency-free validator, always
cargo test                       # serde round-trip over the same fixtures
.ores-lint/lint.sh               # all of the above plus eslint and markdownlint
```
