#!/usr/bin/env node
// Sweep `ores-contracts <command>` over every slice named in contracts.config.json.
//
// The toolkit parses exactly one file per authority, so each slice owns a config
// under contracts/config/<slice>.config.json. This runner exists only to loop; it
// never parses, diffs or emits anything itself, so the parity verdict always comes
// from ores-contracts and nowhere else.
//
//   node scripts/contracts-slices.mjs [check|generate|bootstrap] [--only slice,slice]
//
// Exit code 0 only when every slice passed.
import { readFileSync, existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const cfgPath = resolve(repoRoot, 'contracts.config.json');
const cfg = JSON.parse(readFileSync(cfgPath, 'utf8'));

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

const failed = [];
for (const slice of slices) {
  const sliceCfg = resolve(repoRoot, configDir, `${slice}.config.json`);
  if (!existsSync(sliceCfg)) {
    console.error(`[contracts-slices] ✗ ${slice}: missing ${configDir}/${slice}.config.json`);
    failed.push(slice);
    continue;
  }
  console.log(`\n[contracts-slices] ── ${slice} ─────────────────────────────`);
  const result = spawnSync('npx', ['ores-contracts', command, '--config', sliceCfg, ...passthrough], {
    cwd: repoRoot,
    stdio: 'inherit',
  });
  if (result.status !== 0) failed.push(slice);
}

console.log(`\n[contracts-slices] ${slices.length - failed.length}/${slices.length} slices passed ${command}`);
if (failed.length) {
  console.error(`[contracts-slices] failed: ${failed.join(', ')}`);
  process.exit(2);
}
