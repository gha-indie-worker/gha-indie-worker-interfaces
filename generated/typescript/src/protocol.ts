import { InterfaceError } from "./errors";
import type { WorkerLease } from "./types";

export function parseWorkerLease(
  id: string,
  revision: string,
  payload: Record<string, unknown>,
): WorkerLease {
  if (!id.trim()) {
    throw new InterfaceError("empty_id");
  }
  if (!revision.trim()) {
    throw new InterfaceError("empty_revision");
  }
  return { id, revision, payload };
}

