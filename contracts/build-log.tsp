namespace GhaIndieWorker;

/** Logical source of a build-log chunk. */
enum BuildLogStream {
  stdout: "stdout",
  stderr: "stderr",
  worker: "worker",
}

/** Lifecycle event carried on the sidecar metadata stream. */
enum BuildLogEvent {
  chunk: "chunk",
  dropped: "dropped",
  receiverClosed: "receiver_closed",
  streamClosed: "stream_closed",
}

/**
 * Metadata for one independent build-log data-plane event.
 *
 * The metadata channel is newline-delimited JSON. Raw log bytes travel on a
 * separate data descriptor so consumers do not need to base64 encode logs.
 */
model BuildLogMetadata {
  schemaVersion: "gha-indie-worker.build-log-metadata/v1";
  event: BuildLogEvent;
  jobId: string;
  stream: BuildLogStream;
  sequence: uint32;
  byteLength: uint32;
  timestamp: string;
  repository?: string;
  githubOrganization?: string;
  workflow?: string;
  runId?: string;
  jobName?: string;
  stepName?: string;
  attempt?: uint32;
  traceId?: string;
  spanId?: string;
  droppedChunks?: uint32;
  droppedBytes?: uint32;
}
