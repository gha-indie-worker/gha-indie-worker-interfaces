import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Webhooks;

@doc("Where a delivery stopped. rejected and duplicate are terminal and never dispatch work.")
enum DeliveryStatus {
  Received: "received",
  Verified: "verified",
  Rejected: "rejected",
  Dispatched: "dispatched",
  Duplicate: "duplicate",
}

enum RegistryEventAction {
  Push: "push",
  Delete: "delete",
  Mirror: "mirror",
}

@doc("The envelope for one GitHub webhook delivery. The HMAC verdict is recorded before any fetch or plan.")
@Ores.table("github_deliveries")
@Ores.unique("deliveryId")
@Ores.index("repository,receivedAt")
@Ores.index("status")
model GitHubDelivery {
  @key id: uuid;
  deliveryId: uuid;

  @minLength(1)
  @maxLength(64) event: string;

  @maxLength(64) action?: string;

  @minLength(3)
  @maxLength(140)
  @pattern("^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$") repository: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") headSha?: string;

  @maxLength(255) workflowPath?: string;

  signatureValid: boolean;
  status: DeliveryStatus;
  receivedAt: utcDateTime;

  @maxLength(200) rejectionReason?: string;

  payload: json;
}

@doc("A normalized container-registry image event. ECR EventBridge and distribution v2 payloads fold into this shape.")
@Ores.table("registry_image_events")
@Ores.unique("registry,repository,digest,action,occurredAt")
@Ores.index("repository,occurredAt")
model RegistryImageEvent {
  @key id: uuid;

  @minLength(1)
  @maxLength(255) registry: string;

  @minLength(1)
  @maxLength(255) repository: string;

  @maxLength(128) tag?: string;

  @minLength(71)
  @maxLength(71)
  @pattern("^sha256-[0-9a-f]+$") digest: string;

  action: RegistryEventAction;
  occurredAt: utcDateTime;
  receivedAt: utcDateTime;

  @Ores.references("GitHubDelivery.id") sourceDeliveryId?: uuid;

  payload: json;
}
