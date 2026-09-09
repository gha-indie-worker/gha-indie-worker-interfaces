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
