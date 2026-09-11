# gha-indie-worker-interfaces

Data-only contracts for Independent GitHub Actions worker and clone-server control plane. Schema: `schema/v1`. Generated TypeScript and Dart live under `generated/` and must stay types-only.

## Contracts: two independent authorities

`contracts/` holds the contract for this product as **two hand-authored, peer
authorities**:

* `contracts/typespec/<slice>.tsp`
* `contracts/json-schema/<slice>.schema.json` (draft 2020-12)

**Neither authority is generated from the other**, and neither is generated from
the Rust in `src/v1/`. Two complementary fail-closed gates check those same
authorities:

* [`ores-contracts`](https://github.com/ORESoftware/ores-contracts) parses each
  authority independently, drives the language/persistence emitters from each
  parse, and requires the resulting artifacts to converge before generated output
  can be admitted.
* [`typespec-json-schema-validator`](https://github.com/ORESoftware/typespec-json-schema-validator)
  (TJSV) uses the official TypeSpec JSON Schema emitter only to create comparison
  evidence, recursively compares that witness with the human-authored Draft
  2020-12 schema, and executes both authorities as validators over deterministic
  probes. The generated witness is evidence only and never replaces either
  authority.

Ten slices: `identity`, `onboarding`, `runs`, `workers`, `webhooks`, `chat`,
`embeddings`, `sync`, `transport`, `errors`. See `contracts/README.md` for the
layout, the expressible subset, and the fixture rules.

```sh
npm ci
npm run contracts:check:all      # independent parse/projection parity, every slice
npm run contracts:tjsv           # TypeSpec ↔ Draft 2020-12 semantic/behavior parity
npm run validate:fixtures        # JSON Schema over contracts/fixtures/**
cargo test                       # the same fixtures through serde
.ores-lint/lint.sh               # everything, plus eslint and markdownlint
```

`src/v1/` is the ergonomic Rust wire surface (serde, `camelCase` on the wire,
`kebab-case` enums). `generated/<slice>/` is machine output committed by the
runner lane. `schema/v1/workerlease.json` and `src/protocol.rs` are the earlier
surface and are unchanged.

## IndieBuild BYOC / private SaaS

IndieBuild can keep its hosted control plane while running the execution data
plane inside a customer's AWS, GCP, Azure, or Kubernetes environment. The BYOC
model, trust boundary, enrollment flow, marketplace packaging, usage metering,
and monetization shape are documented in [`docs/byoc-marketplace.md`](docs/byoc-marketplace.md).

The key boundary is deliberate: customer source, build workspaces, deployment
credentials, private-network access, and customer-selected artifact/log stores
remain in the customer cloud by default. The hosted control plane owns tenancy,
entitlements, scheduling metadata, signed enrollment/lease flows, and billing
receipts.
