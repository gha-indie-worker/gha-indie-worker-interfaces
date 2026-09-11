import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Identity;

enum Role {
  Owner: "owner",
  Admin: "admin",
  Member: "member",
  Billing: "billing",
}

enum InvitationStatus {
  Pending: "pending",
  Accepted: "accepted",
  Revoked: "revoked",
  Expired: "expired",
}

enum SeatStatus {
  Active: "active",
  Suspended: "suspended",
  Released: "released",
}

@doc("A billing and ownership boundary. Every run, worker and seat hangs off exactly one org.")
@Ores.table("orgs")
@Ores.unique("slug")
@Ores.index("githubLogin")
model Org {
  @key id: uuid;

  @maxLength(64)
  @pattern("^[a-z0-9][a-z0-9-]*[a-z0-9]$")
  slug: string;

  @minLength(1)
  @maxLength(200) displayName: string;

  @maxLength(320)
  @pattern("^[^@ ]+@[^@ ]+[.][^@ ]+$") billingEmail?: string;

  @maxLength(64) githubLogin?: string;

  @minValue(1)
  @maxValue(10000)
  seatLimit: int32;

  createdAt: utcDateTime;
  deletedAt?: utcDateTime;
}

@doc("A human principal. Identity is federated by shared-auth over Supabase Auth and Neon Auth.")
@Ores.table("users")
@Ores.unique("sharedAuthSubject")
@Ores.unique("email")
model User {
  @key id: uuid;
  @minLength(1)
  @maxLength(255) sharedAuthSubject: string;
  @maxLength(320)
  @pattern("^[^@ ]+@[^@ ]+[.][^@ ]+$") email: string;
  @maxLength(200) displayName?: string;
  @maxLength(64) githubLogin?: string;
  createdAt: utcDateTime;
  lastSeenAt?: utcDateTime;
}

@doc("Membership of a user in an org, carrying exactly one role.")
@Ores.table("org_members")
@Ores.unique("orgId,userId")
@Ores.index("userId")
model OrgMember {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @Ores.references("User.id") userId: uuid;
  role: Role;
  createdAt: utcDateTime;
  @Ores.references("User.id") invitedBy?: uuid;
}

@doc("A pending or settled invitation to join an org at a given role.")
@Ores.table("org_invitations")
@Ores.unique("orgId,email")
@Ores.index("token")
model Invitation {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @maxLength(320)
  @pattern("^[^@ ]+@[^@ ]+[.][^@ ]+$") email: string;
  role: Role;
  status: InvitationStatus;
  @minLength(32)
  @maxLength(128) token: string;
  createdAt: utcDateTime;
  expiresAt: utcDateTime;
  acceptedAt?: utcDateTime;
  @Ores.references("User.id") acceptedBy?: uuid;
}

@doc("A paid seat in an org. A seat may be held by no user while it is being reassigned.")
@Ores.table("org_seats")
@Ores.index("orgId,status")
model Seat {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @Ores.references("User.id") userId?: uuid;
  status: SeatStatus;
  assignedAt?: utcDateTime;
  releasedAt?: utcDateTime;
}
