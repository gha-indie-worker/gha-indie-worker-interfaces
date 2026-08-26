export const PROTOCOL_VERSION = "1" as const;
export const SCHEMA_REVISION = "gha-indie-worker-0001" as const;

export interface Health {
  ok: boolean;
  service: string;
  protocol: string;
}

export interface WorkerLease {
  id: string;
  revision: string;
  payload: Record<string, unknown>;
}

