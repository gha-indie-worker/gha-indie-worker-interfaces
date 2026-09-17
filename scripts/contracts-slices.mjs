#!/usr/bin/env node
// Sweep `ores-contracts <command>` over every slice named in contracts.config.json.
//
// The toolkit parses exactly one file per authority, so each slice owns a config
// under contracts/config/<slice>.config.json. This runner exists only to loop; it
// never parses, diffs or emits contract content itself, so the parity verdict
// always comes from the installed ores-contracts package and nowhere else.
//
//   node scripts/contracts-slices.mjs [check|generate|bootstrap] [--only slice,slice]
//
// Exit code 0 only when every requested slice actually ran and produced the
// command-specific receipt/artifacts expected from ores-contracts.
import { readFileSync, existsSync, rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const cfgPath = resolve(repoRoot, 'contracts.config.json');
const cfg = JSON.parse(readFileSync(cfgPath, 'utf8'));
const oresContractsCli = resolve(
  repoRoot,
  'node_modules/@oresoftware/ores-contracts/src/cli.mjs',
);

if (!existsSync(oresContractsCli)) {
  console.error(`[contracts-slices] missing installed ores-contracts CLI: ${oresContractsCli}`);
  process.exit(1);
}

const argv = process.argv.slice(2);
const command = argv.find((a) => !a.startsWith('--')) ?? 'check';
if (!['check', 'generate', 'bootstrap'].includes(command)) {
  console.error(`[contracts-slices] unknown command ${command}`);
  process.exit(1);
}
const onlyIdx = argv.indexOf('--only');
const only = onlyIdx >= 0 && argv[onlyIdx + 1] ? new Set(argv[onlyIdx + 1].split(',')) : null;
const passthrough = argv.filter((a, i) => a.startsWith('--') && a !== '--only' && argv[i - 1] !== '--only');

const configDir = cfg.sliceConfigDir ?? 'contracts/config';
const slices = (cfg.slices ?? []).filter((s) => !only || only.has(s));
if (!slices.length) {
  console.error('[contracts-slices] contracts.config.json lists no slices');
  process.exit(1);
}

function loadSliceConfig(sliceCfg) {
  const raw = JSON.parse(readFileSync(sliceCfg, 'utf8'));
  const root = dirname(sliceCfg);
  return {
    raw,
    out: resolve(root, raw.out ?? 'generated'),
    target: resolve(root, raw.target ?? 'target/ores-contracts'),
    artifacts: raw.artifacts ?? [],
  };
}

function clearExpectedEvidence(commandName, sliceConfig) {
  // Remove only evidence that this command is required to recreate. This makes
  // a zero-exit/no-op CLI impossible to misclassify as a successful run.
  rmSync(resolve(sliceConfig.target, 'receipt.json'), { force: true });
  if (commandName === 'generate') {
    for (const artifact of sliceConfig.artifacts) {
      rmSync(resolve(sliceConfig.out, artifact), { force: true });
    }
    rmSync(resolve(sliceConfig.out, 'receipt.json'), { force: true });
  }
}

function verifyEvidence(commandName, sliceConfig) {
  if (commandName === 'bootstrap') return null;
  const targetReceipt = resolve(sliceConfig.target, 'receipt.json');
  if (!existsSync(targetReceipt)) return `missing receipt ${targetReceipt}`;
  const receipt = JSON.parse(readFileSync(targetReceipt, 'utf8'));
  if (receipt.status !== 'passed') return `receipt status is ${JSON.stringify(receipt.status)}`;
  if (commandName === 'generate') {
    const missing = sliceConfig.artifacts
      .map((artifact) => resolve(sliceConfig.out, artifact))
      .filter((artifact) => !existsSync(artifact));
    if (missing.length) return `missing generated artifact(s): ${missing.join(', ')}`;
    const generatedReceipt = resolve(sliceConfig.out, 'receipt.json');
    if (!existsSync(generatedReceipt)) return `missing generated receipt ${generatedReceipt}`;
  }
  return null;
}

const failed = [];
for (const slice of slices) {
  const sliceCfg = resolve(repoRoot, configDir, `${slice}.config.json`);
  if (!existsSync(sliceCfg)) {
    console.error(`[contracts-slices] ✗ ${slice}: missing ${configDir}/${slice}.config.json`);
    failed.push(slice);
    continue;
  }
  const sliceConfig = loadSliceConfig(sliceCfg);
  clearExpectedEvidence(command, sliceConfig);
  console.log(`\n[contracts-slices] ── ${slice} ─────────────────────────────`);
  const result = spawnSync(
    process.execPath,
    [oresContractsCli, command, '--config', sliceCfg, ...passthrough],
    { cwd: repoRoot, stdio: 'inherit' },
  );
  if (result.error || result.signal || result.status !== 0) {
    if (result.error) console.error(`[contracts-slices] ${slice}: ${result.error.message}`);
    if (result.signal) console.error(`[contracts-slices] ${slice}: terminated by ${result.signal}`);
    failed.push(slice);
    continue;
  }
  const evidenceError = verifyEvidence(command, sliceConfig);
  if (evidenceError) {
    console.error(`[contracts-slices] ✗ ${slice}: ${evidenceError}`);
    failed.push(slice);
  }
}

console.log(`\n[contracts-slices] ${slices.length - failed.length}/${slices.length} slices passed ${command}`);
if (failed.length) {
  console.error(`[contracts-slices] failed: ${failed.join(', ')}`);
  process.exit(2);
}
