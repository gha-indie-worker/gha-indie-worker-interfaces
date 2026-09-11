# IndieBuild BYOC marketplace model

IndieBuild can support two delivery modes without forking the product contract:

1. **Managed SaaS** — IndieBuild operates the control plane and runner capacity.
2. **Bring Your Own Cloud (BYOC)** — IndieBuild operates the commercial/control-plane relationship while the execution data plane is installed into infrastructure owned by the customer.

BYOC is the preferred enterprise shape when source code, build credentials, signing keys, deployment credentials, internal package registries, or private network routes must remain inside the customer's security boundary.

## Boundary: control plane versus customer data plane

The BYOC deployment should make the boundary explicit and enforceable.

### IndieBuild control plane

The hosted `indiebuild.dev` control plane owns:

- organization and user tenancy;
- GitHub App installation metadata and authorization policy;
- subscription and marketplace entitlement state;
- runner-fleet desired state;
- signed worker enrollment challenges;
- scheduling metadata that does not contain customer secrets;
- usage-meter receipts and billing aggregation;
- support/audit metadata explicitly allowed by policy;
- release-channel metadata for the BYOC worker/operator.

### Customer-owned data plane

The customer cloud owns:

- runner compute and autoscaling;
- VPC/VNet/subnet placement;
- source checkouts and ephemeral workspaces;
- build caches and artifacts unless the customer opts into hosted storage;
- deployment credentials, signing keys, cloud identities, and secret stores;
- private registries and private network routes;
- customer-selected log destinations and retention policy.

The default network shape should be **outbound-only from the customer data plane**. IndieBuild should not require a public inbound listener in the customer VPC. Workers establish authenticated control channels and fetch signed work leases; the control plane cannot choose arbitrary container images or arbitrary commands outside operator-reviewed worker profiles.

## Deployment modes

| Mode | Execution location | Best fit | Commercial shape |
| --- | --- | --- | --- |
| Managed SaaS | IndieBuild cloud | Small teams / fastest onboarding | seat or usage subscription |
| BYOC Standard | Customer AWS/GCP/Azure/Kubernetes | Security-conscious teams | platform subscription + metered execution |
| BYOC Isolated | Dedicated customer account/project/subscription | Regulated enterprise | annual/private offer + support SLA |
| Offline / restricted egress | Customer environment | Special enterprise cases | negotiated annual license and support |

The same wire contracts should identify the deployment mode and entitlement state; deployment-specific infrastructure details belong in `gha-indie-worker-infra`, not in generated language bindings.

## Marketplace packaging

The product should be distributable through multiple procurement paths while keeping one runtime protocol:

- an OCI image for the worker and operator/control-side agent;
- a Helm chart for Kubernetes installations;
- Terraform modules for AWS, GCP, and Azure reference deployments;
- cloud-marketplace listings and private offers that grant an IndieBuild entitlement;
- direct annual contracts for customers that cannot purchase through a cloud marketplace.

Marketplace subscription is an **entitlement source**, not the source of truth for runtime identity. A subscription should resolve to an IndieBuild organization entitlement, and worker enrollment should still use short-lived cryptographic identity rather than a long-lived marketplace credential.

## Installation handshake

A safe BYOC enrollment flow is:

1. Customer subscribes or accepts a private offer.
2. IndieBuild maps the marketplace buyer/account to an IndieBuild organization entitlement.
3. Customer launches the infrastructure package in its own cloud account/project/subscription.
4. Deployment receives a one-time enrollment challenge, not a reusable API secret.
5. The customer-side agent proves workload identity and exchanges the challenge for a short-lived machine credential.
6. The agent registers the runner fleet capabilities and immutable image digest.
7. The control plane issues signed leases only for operator-reviewed execution profiles.
8. Workers report signed usage receipts and health evidence; customer source and deployment secrets remain local.

## Monetization

BYOC should not be priced only as raw compute because the customer already pays the cloud provider for compute. IndieBuild is selling orchestration, security controls, policy, auditability, developer experience, support, and procurement convenience.

A strong commercial model combines three dimensions:

- **Platform entitlement** — recurring monthly or annual fee per customer organization/environment.
- **Capacity or usage meter** — concurrent runner slots, active runner-hours, or executed build-minutes. Prefer one primary meter so bills remain understandable.
- **Enterprise add-ons** — SSO/SCIM, policy packs, compliance evidence, longer audit retention, dedicated release channels, premium support, and deployment assistance.

For larger buyers, prioritize annual marketplace private offers with a committed minimum plus overage. This lets the buyer use existing cloud procurement while giving IndieBuild predictable recurring revenue. A direct enterprise contract should remain available so the product is not economically coupled to any single marketplace.

Do not charge separately for every low-level feature. The paid boundary should map to buyer value: isolation, governance, compliance evidence, fleet scale, reliability, and support.

## Metering and entitlement contract

The commercial contract should eventually expose a small, provider-neutral entitlement model shared across Rust, TypeScript, Dart/Flutter, and other consumers. At minimum it needs:

- entitlement identifier and customer organization identifier;
- deployment mode;
- entitlement status and validity window;
- maximum concurrent workers/jobs;
- included usage quantity and meter unit;
- overage policy;
- support tier;
- source marketplace/provider and provider subscription reference;
- signed usage-receipt identifier to make billing events idempotent.

Those fields should be authored independently in TypeSpec and JSON Schema Draft 2020-12 and admitted only when TJSV reports parity. Marketplace-specific payloads should be translated at the boundary into this provider-neutral contract rather than leaking AWS/GCP/Azure billing objects through the core runtime.

## Security requirements

- Never send customer cloud credentials through the IndieBuild control plane.
- Prefer workload identity / federation and short-lived credentials.
- Pin worker images by immutable digest.
- Sign enrollment challenges, leases, release manifests, and usage receipts.
- Make billing events idempotent and replay-safe.
- Separate organization tenancy from cloud account/project/subscription identifiers.
- Record who changed fleet policy and entitlement policy.
- Default to least-privilege outbound connectivity.
- Make telemetry categories opt-in/explicit for customer source, logs, artifacts, and environment metadata.
- Preserve a customer-controlled kill switch that disables the IndieBuild agent without affecting unrelated customer infrastructure.

## Contract enforcement

`gha-indie-worker-interfaces` keeps TypeSpec and human-authored JSON Schema as peer authorities. They are not generated from each other. Two complementary gates are expected:

- `ores-contracts` independently parses both authorities and requires generated language/persistence projections to converge.
- `ORESoftware/typespec-json-schema-validator` (TJSV) uses the official TypeSpec JSON Schema emitter as comparison evidence, checks Draft 2020-12 structure, recursively compares declarations, and executes both authorities as validators over deterministic probes.

A generated witness is evidence only. It must never overwrite either authored authority.

## Follow-up implementation slices

The next implementation work should land without coupling cloud-provider objects into the core API:

1. provider-neutral `DeploymentMode`, `CloudProvider`, `Entitlement`, `UsageMeter`, and `UsageReceipt` contracts;
2. Rust serde mirrors and admission validation;
3. marketplace adapters in the write-side API server;
4. customer-cloud deployment modules in `gha-indie-worker-infra`;
5. signed enrollment and usage-receipt verification;
6. entitlement-aware scheduling and concurrency enforcement;
7. dashboard views for deployment health, usage, invoices, and policy;
8. private-offer/admin workflows in the separately permissioned admin surface.
