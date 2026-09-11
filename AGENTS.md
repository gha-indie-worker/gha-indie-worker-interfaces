# GHA Indie Worker — interfaces

## Parent / root agent contract

The fleet-wide parent lives at:

- GitHub: https://github.com/oresoftware/my-ai/AGENTS.md
- Canonical disk path: `~/codes/oresoftware/my-ai/AGENTS.md`
- `~/codes/AGENTS.md` is a symlink to `~/codes/oresoftware/my-ai/AGENTS.md` (installed by `~/codes/oresoftware/my-ai/setup-final.sh`)

When this file and the parent disagree: follow this file for this repository's local layout and tools; follow the parent for org-wide conventions and the functional programming rules.

Canonical `interfaces` repository for [`gha-indie-worker`](https://github.com/gha-indie-worker).

- Internal runtimes: Rust, TypeScript, Dart.
- Contracts: independent TypeSpec and JSON Schema Draft 2020-12 authorities in `gha-indie-worker-interfaces`.
- Auth: github.com/shared-auth.
- Sync: github.com/opto-sync.
- Telemetry: github.com/ores-otel.
- Flags: github.com/flags-2-env.
- Packages: github.com/zed-pkg.
- Never use React/JSX or webviews.
- Resolve git conflicts semantically; never rebase, stash, or reset.

No function bodies except parse/validate.

## Code style and coding patterns

remember to modularize the rust, typescript and dart - not everything belongs in main.rs, main.ts and main.dart; also follow functional coding principles - fewer side-effects (use pure functions more), more immutability (immutable variables); but for stateful apps like the client or stateful servers like websockets or tcp connections, sometimes classes and oop make more sense than functional programming perse, but we can still adhere to functional programming more than usual. Favor exhaustive pattern matching and use formal methods checking too. Favor composability and re-use , so basically create more utility functions and routines for shared use. You can follow a medium level of D.R.Y. (don't repeat yourself) - in other words you can repeat yourself at medium amount (not too much not too little). Some chaining is totally fine, so either method-chaining (immutable sometimes although with classes can be mutable too for performance), and chaining via the pipe operator is ok in languages like gleamlang.

Functional programming is mostly the following:

- explicit inputs
- explicit outputs
- immutable values
- pure transformations
- typed errors
- explicit state transitions
- composition
- effects pushed outward
- illegal states excluded by types

## Contracts protocol (ores-contracts + TJSV)

**TypeSpec and JSON Schema are independent, human-authored, top-level
authorities. Neither is generated from the other.** This is the single most
important rule in this repository:

- never write a `.tsp` by running a converter over a `.schema.json`, or the
  reverse — `ores-contracts bootstrap` exists only to *draft* a new authority,
  and its output must be reviewed and re-authored before it counts;
- never hand-edit `generated/**` — change an authority instead;
- never resolve a parity finding by editing whichever authority is "wrong". A
  finding means the two documents disagree about meaning: decide what the
  contract should say, then change both deliberately;
- a new field is three edits — the `.tsp`, the `.schema.json` and `src/v1/` — plus
  at least one fixture. `tests/fixtures_roundtrip.rs` fails if a model exists in
  the JSON Schema authority with no Rust mirror;
- run both parity gates for contract changes: `npm run contracts:check:all` for
  independent parse/projection parity and `npm run contracts:tjsv` for official
  TypeSpec-emitter witness comparison plus differential validator behavior;
- treat TJSV-generated schemas and receipts as evidence only. They never become
  an authored authority and never overwrite `contracts/typespec/**` or
  `contracts/json-schema/**`.

The parity IR is persistence-shaped, so every model carries a table name and a
primary key, tagged unions are a sealed object plus a `kind` discriminator, and
value bounds (`minimum`, `pattern`, `maxItems`, …) are runtime refinements that
both authorities still state. `contracts/README.md` has the details, including the
two syntax traps in the toolkit's TypeSpec parser (no braces inside a model body,
no `)` inside a decorator argument).

Fixtures are three-way: `valid/` must pass both the JSON Schema validator and
serde, `invalid/` must fail both, and `invalid/schema-only/` must fail the
validator while still parsing under serde. Filing one in the wrong directory is a
test failure with a message that names the right directory.
