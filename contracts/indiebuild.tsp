namespace GhaIndieWorker;

/** Repository-level role used to prevent client/server privilege bleed. */
enum IndieBuildRepositoryRole {
  client: "client",
  server: "server",
  mixed: "mixed",
}

/** Per-target role. Mixed repositories declare separate client/server targets. */
enum IndieBuildTargetRole {
  client: "client",
  server: "server",
}

/** Runner operating system requested by the repository contract. */
enum IndieBuildPlatform {
  linux: "linux",
  macos: "macos",
  windows: "windows",
}

/**
 * One buildable target. `profile` names an operator-reviewed IndieBuild profile;
 * repositories never provide arbitrary runner shell as part of this contract.
 */
model IndieBuildTarget {
  name: string;
  role: IndieBuildTargetRole;
  path: string;
  profile: string;
  platform: IndieBuildPlatform;
  artifacts: string[];
  cache_paths: string[];
  env: string[];
  secret_env: string[];
  allow_network: boolean;
  allow_push: boolean;
  allow_deploy: boolean;
  timeout_seconds: uint32;
}

/** Repository-root `.indiebuild.toml`, represented as normalized JSON. */
model IndieBuildConfig {
  schema_version: "gha-indie-worker.indiebuild/v1";
  repository_role: IndieBuildRepositoryRole;
  default_target: string;
  targets: IndieBuildTarget[];
}
