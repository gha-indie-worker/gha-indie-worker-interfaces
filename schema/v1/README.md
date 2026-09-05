# `schema/v1`

JSON Schema documents (draft 2020-12) owned by this organization.

| File | Owns |
|---|---|
| `workerlease.json` | The `WorkerLease` envelope |
| `wasm-release-profile.json` | This organization's profile over the OWLS `release-v1` WASM release contract |

## The WASM release profile is a *narrowing*, not a replacement

`wasm-release-profile.json` does **not** define the WASM release contract. That
contract is **`release-v1`**, owned by the `ores-wasm-loaders` organization and
published as `schemas/release.schema.json` in `owls-interfaces`:

```
$id: https://ores-wasm-loaders.github.io/schemas/release-v1.json
```

**`release-v1` is the base authority.** The profile only adds organization-
specific restrictions on top of it:

| The profile narrows | From release-v1 | To |
|---|---|---|
| `appId` | any `^[a-z0-9][a-z0-9-]{0,63}$` | the closed set `user-app`, `org-app`, `auth-app`, `owls-pilot` |
| `runtime` | `raw-wasm`, `wasm-bindgen`, `flutter-web` | `raw-wasm`, `wasm-bindgen` |
| asset `url` | any `^https://[^\s]+$` | an origin fixed by the declared deploy surface, and no query or fragment |
| asset `bytes` | max 268435456 (256 MiB) | max 67108864 (64 MiB) |
| `extensions` | optional, unconstrained | **required**, and must contain `ghaIndieWorker` |

`runtime` excludes `flutter-web` because this organization has no Flutter web
build: `gha-indie-worker-flutter` has no `web/` directory on any branch. Re-add
it when such a build actually exists.

The profile **never** widens `release-v1`, relaxes one of its constraints, or
redefines a field. Anything it permits, `release-v1` already permitted.

## Exact validation order

Validate a release manifest in **two separate passes**, and stop at the first
failure:

1. **`release-v1`** — the base contract. A document that fails here is invalid,
   full stop, whatever the profile says.
2. **`wasm-release-profile.json`** — this organization's restrictions.

Do not merge the two into one schema, and do not treat the profile as a
substitute for step 1. Two reasons:

- The profile restates some `release-v1` constraints so that it is
  self-contained and needs no network `$ref` resolution. Those restatements are
  a convenience, **not** a complete copy. `release-v1` also enforces cross-field
  rules the profile does not reproduce — most importantly that `entrypoint`
  names a declared asset whose `kind` matches the `runtime`, and that asset IDs
  and URLs are unique.
- `release-v1` is owned by another organization and can change. Vendoring it
  here would create a second, silently diverging authority.

### Deploy-surface coupling

`extensions.ghaIndieWorker.deploySurface` selects which asset origins and which
owning repository are permitted:

| `deploySurface` | Asset origin must match | `repository` must be owned by |
|---|---|---|
| `test-harness` | `https://gha-indie-worker-test.github.io/…` | `gha-indie-worker-test/…` |
| `user-app`, `org-app`, `auth-app` | `https://{user,org,auth,app}.gha-indie-worker.github.io/…` | `gha-indie-worker/…` |

This is what stops a production manifest from naming test bytes, and a test
manifest from naming production bytes.

**As of this file's authorship, none of the production surfaces exist.** Naming
one in a manifest does not create it, and the profile cannot check that a host
resolves. `test-harness` is the only surface with anything actually deployed
behind it.

## Rust accessor

`src/wasm_release_profile.rs` exposes the profile as an `include_str!` constant
plus its `$id`, with tests that it parses as JSON and that its `$id` matches the
constant. It embeds **only** the profile — `release-v1` is deliberately not
vendored, for the reason above.

```rust
use gha_indie_worker_interfaces::{
    RELEASE_V1_SCHEMA_ID,            // validate against this FIRST
    WASM_RELEASE_PROFILE_SCHEMA,     // then against this
    WASM_RELEASE_PROFILE_SCHEMA_ID,
};
```

The crate stays data-only: it ships no validator and adds no dependency beyond
the `serde` / `serde_json` already in `Cargo.toml`.
