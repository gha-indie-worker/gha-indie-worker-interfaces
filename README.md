# gha-indie-worker-interfaces

Data-only contracts for Independent GitHub Actions worker and clone-server control plane. Schema: `schema/v1`. Generated TypeScript and Dart live under `generated/` and must stay types-only.

## Two authorities

`contracts/` holds **two independent, peer, human-authored descriptions** of the
same vocabulary. Neither is generated from the other and neither is canonical:

- `contracts/typespec/` -- TypeSpec: identity and tenancy, the CI/CD domain, the
  live session protocol, the embedding/discovery surface, with real HTTP
  operations and one error model.
- `contracts/json-schema/` -- JSON Schema 2020-12 describing the same vocabulary,
  authored from the requirements rather than transcribed from the TypeSpec.
- `contracts/proto/` -- protobuf v3 for the wire types that are validated at
  runtime. Not a third authority: it is checked against both and never breaks a tie.
- `contracts/fixtures/` -- positive and negative cases per model. A negative case
  must be rejected by *both* authorities; that is the strongest evidence the two
  descriptions actually encode the same constraints.

Each authority independently generates code, validation and SQL, and the results
are compared as evidence. **A discrepancy fails closed** -- the gate reports every
one and exits non-zero, and a human decides which authority was wrong. Nothing is
ever silently reconciled.

```sh
just contracts     # the parity gate: both authorities, every fixture, the proto
just lint          # rust + typescript + dart + json-schema + typespec + yaml + shell
```

The persisted subset runs through [`@oresoftware/ores-contracts`][oc], the fleet's
existing parity toolkit, wired in as a zed-pkg dependency -- this repo adds only
what that toolkit does not cover. See [`contracts/README.md`](contracts/README.md)
for the model, what happens on a disagreement, and how to add a model (touch both
authorities plus a fixture, never one).

[oc]: https://github.com/ORESoftware/ores-contracts

## Linting

One command, `just lint`, runs every lane; CI runs the same lanes through the
reusable workflow in `.github/workflows/fleet-lint.yml`, which the org's other
repositories **call** rather than copy:

```yaml
jobs:
  lint:
    uses: gha-indie-worker/gha-indie-worker-interfaces/.github/workflows/fleet-lint.yml@main
    with: { rust: true, typescript: true }
```
