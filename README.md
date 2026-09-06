# gha-indie-worker-interfaces

Data-only contracts for Independent GitHub Actions worker and clone-server control plane. Schema: `schema/v1`. Generated TypeScript and Dart live under `generated/` and must stay types-only.

## Contracts: two independent authorities

`contracts/` holds the contract for this product as **two hand-authored, peer
authorities**:

* `contracts/typespec/<slice>.tsp`
* `contracts/json-schema/<slice>.schema.json` (draft 2020-12)

**Neither authority is generated from the other**, and neither is generated from
the Rust in `src/v1/`. Each is parsed on its own by
[`ores-contracts`](https://github.com/ORESoftware/ores-contracts), each parse
drives the same emitters, and the two sets of artifacts must match byte for byte
before anything is written to `generated/`. A mismatch is a finding with a stable
fingerprint and stops the run for human evaluation.

Ten slices: `identity`, `onboarding`, `runs`, `workers`, `webhooks`, `chat`,
`embeddings`, `sync`, `transport`, `errors`. See `contracts/README.md` for the
layout, the expressible subset, and the fixture rules.

```sh
npm ci
npm run contracts:check:all      # parity for every slice
npm run validate:fixtures        # JSON Schema over contracts/fixtures/**
cargo test                       # the same fixtures through serde
.ores-lint/lint.sh               # everything, plus eslint and markdownlint
```

`src/v1/` is the ergonomic Rust wire surface (serde, `camelCase` on the wire,
`kebab-case` enums). `generated/<slice>/` is machine output committed by the
runner lane. `schema/v1/workerlease.json` and `src/protocol.rs` are the earlier
surface and are unchanged.
