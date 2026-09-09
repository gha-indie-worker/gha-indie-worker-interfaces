namespace GhaIndieWorker.BuildLogs;

@minLength(1)
@maxLength(100)
@pattern("^[A-Za-z0-9_.-]+$")
scalar GitHubOrgName extends string;

@minLength(1)
@maxLength(100)
@pattern("^[A-Za-z0-9_.-]+$")
scalar GitHubRepoName extends string;

@minLength(1)
@maxLength(20)
@pattern("^[1-9][0-9]{0,19}$")
scalar GitHubId extends string;

@minLength(40)
@maxLength(40)
@pattern("^[0-9a-f]{40}$")
scalar GitCommitSha extends string;

@minLength(64)
@maxLength(64)
@pattern("^[0-9a-f]{64}$")
scalar Sha256Hex extends string;

@minLength(1)
@maxLength(1024)
scalar GitRef extends string;

@minLength(1)
@maxLength(2048)
scalar StorageRef extends string;

@minValue(0)
@maxValue(9007199254740991)
scalar SafeUnsignedInteger extends integer;

@minValue(-2147483648)
@maxValue(2147483647)
scalar ProcessExitCode extends integer;

@minLength(1)
@maxLength(255)
scalar LogSidecarJobId extends string;

@minLength(1)
@maxLength(255)
scalar LogSidecarProgram extends string;

@maxLength(131072)
@pattern("^[0-9a-f]*$")
scalar PayloadHex extends string;

@maxLength(131092)
@pattern("^[0-9a-f]*$")
scalar WireHex extends string;

enum BuildLogStream {
  Stdout: "stdout",
  Stderr: "stderr",
  System: "system",
}

enum BuildLogSource {
  Runner: "runner",
  GitHubReconciliation: "github-reconciliation",
}

enum BuildLogCompression {
  None: "none",
  Gzip: "gzip",
  Zstd: "zstd",
}

enum LogSidecarProtocol {
  V1: "gha-indie-worker.log-sidecar.v1",
}

enum LogSidecarFrameMagic {
  GHLG: "GHLG",
}

enum LogSidecarFrameStream {
  Stdout: "stdout",
  Stderr: "stderr",
}

enum LogSidecarStartEvent {
  CommandStarted: "command_started",
}

enum LogSidecarTerminalEvent {
  CommandFinished: "command_finished",
  CommandTimedOut: "command_timed_out",
  CommandWaitFailed: "command_wait_failed",
}

model LogSidecarFrameDescriptor {
  protocol: LogSidecarProtocol;
  magic: LogSidecarFrameMagic;

  @minValue(1)
  @maxValue(1)
  version: integer;

  @minValue(1)
  @maxValue(2)
  streamId: integer;

  stream: LogSidecarFrameStream;

  @minValue(0)
  @maxValue(65536)
  payloadLength: integer;
}

model LogSidecarWireVector {
  @minLength(1)
  @maxLength(64)
  @pattern("^[A-Za-z0-9_.-]+$")
  name: string;

  frame: LogSidecarFrameDescriptor;
  payloadHex: PayloadHex;
  wireHex: WireHex;
}

model LogSidecarCommandStarted {
  schemaVersion: LogSidecarProtocol;
  event: LogSidecarStartEvent;
  atMs: SafeUnsignedInteger;
  jobId: LogSidecarJobId | null;
  program: LogSidecarProgram;
}

model LogSidecarCommandTerminal {
  schemaVersion: LogSidecarProtocol;
  event: LogSidecarTerminalEvent;
  atMs: SafeUnsignedInteger;
  jobId: LogSidecarJobId | null;
  program: LogSidecarProgram;
  success: boolean;
  exitCode: ProcessExitCode | null;
  droppedFrames: SafeUnsignedInteger;
  droppedBytes: SafeUnsignedInteger;
}

model BuildLogIdentity {
  organization: GitHubOrgName;
  repository: GitHubRepoName;

  @minLength(1)
  @maxLength(255)
  workflow: string;

  runId: GitHubId;

  @minValue(1)
  @maxValue(1000)
  runAttempt: integer;

  jobId: GitHubId;
  headSha: GitCommitSha;
  gitRef: GitRef;

  @minLength(1)
  @maxLength(255)
  stepName?: string;

  @minLength(1)
  @maxLength(255)
  runnerName?: string;
}

model BuildLogRecord {
  identity: BuildLogIdentity;
  stream: BuildLogStream;
  source: BuildLogSource;

  @minValue(0)
  sequence: integer;

  @minValue(0)
  offsetBytes: integer;

  @maxLength(65536)
  body: string;

  observedAt: utcDateTime;
  redacted: boolean;
}

model BuildLogChunkManifest {
  identity: BuildLogIdentity;

  @minValue(0)
  firstSequence: integer;

  @minValue(0)
  lastSequence: integer;

  @minValue(0)
  byteCount: integer;

  sha256: Sha256Hex;
  compression: BuildLogCompression;
  objectRef: StorageRef;
  hotStoreRef?: StorageRef;
  storedAt: utcDateTime;
}

model BuildLogGap {
  identity: BuildLogIdentity;

  @minValue(0)
  expectedSequence: integer;

  @minValue(0)
  receivedSequence: integer;

  detectedAt: utcDateTime;
}

model BuildLogTerminalReconciliation {
  identity: BuildLogIdentity;
  complete: boolean;
  downloadedAt: utcDateTime;
  githubArchiveSha256: Sha256Hex;

  @minValue(0)
  highestSequence?: integer;

  @minValue(0)
  gapCount: integer;
}
