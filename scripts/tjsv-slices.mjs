#!/usr/bin/env node
// Run ORESoftware/typespec-json-schema-validator (TJSV) over every independently
// authored TypeSpec + JSON Schema slice. TJSV is a second, fail-closed gate next
// to ores-contracts: it compares official TypeSpec JSON Schema witness evidence
// against the human-authored Draft 2020-12 authority and executes both as
// validators over deterministic probes.
//
// TJSV is pinned to an immutable GitHub commit tarball instead of added to
// package.json so this change does not hand-edit package-lock.json. The tarball
// form deliberately avoids npm's GitFetcher path. Runner environments can
// override the package spec with TJSV_PACKAGE for controlled upgrade testing.
//
//   node scripts/tjsv-slices.mjs [check|inventory] [--only slice,slice]
//
// Exit code 0 only when every selected slice passes.
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const cfg = JSON.parse(readFileSync(resolve(repoRoot, 'contracts.config.json'), 'utf8'));
const argv = process.argv.slice(2);
const command = argv.find((arg) => !arg.startsWith('--')) ?? 'check';

if (!['check', 'inventory'].includes(command)) {
  console.error(`[tjsv-slices] unknown command ${command}`);
  process.exit(1);
}

const onlyIndex = argv.indexOf('--only');
const only = onlyIndex >= 0 && argv[onlyIndex + 1]
  ? new Set(argv[onlyIndex + 1].split(',').filter(Boolean))
  : null;
const slices = (cfg.slices ?? []).filter((slice) => !only || only.has(slice));

if (!slices.length) {
  console.error('[tjsv-slices] contracts.config.json lists no selected slices');
  process.exit(1);
}

const unknown = only ? [...only].filter((slice) => !(cfg.slices ?? []).includes(slice)) : [];
if (unknown.length) {
  console.error(`[tjsv-slices] unknown slice(s): ${unknown.join(', ')}`);
  process.exit(1);
}

const tjsvPackage = process.env.TJSV_PACKAGE
  ?? 'https://github.com/ORESoftware/typespec-json-schema-validator/archive/dfc28bfc000faba5a963f23c708171dfd5f8debf.tar.gz';
const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const reportDir = resolve(repoRoot, '.typespec-json-schema-validator', 'reports');
mkdirSync(reportDir, { recursive: true });

const failed = [];
for (const slice of slices) {
  const typespec = resolve(repoRoot, 'contracts', 'typespec', `${slice}.tsp`);
  const schema = resolve(repoRoot, 'contracts', 'json-schema', `${slice}.schema.json`);

  if (!existsSync(typespec) || !existsSync(schema)) {
    const missing = [
      !existsSync(typespec) ? `contracts/typespec/${slice}.tsp` : null,
      !existsSync(schema) ? `contracts/json-schema/${slice}.schema.json` : null,
    ].filter(Boolean);
    console.error(`[tjsv-slices] ✗ ${slice}: missing ${missing.join(', ')}`);
    failed.push(slice);
    continue;
  }

  console.log(`\n[tjsv-slices] ── ${slice} ─────────────────────────────`);
  const tjsvArgs = command === 'inventory'
    ? ['inventory', `--typespec=${typespec}`]
    : [
        'check',
        `--typespec=${typespec}`,
        `--schema=${schema}`,
        `--report=${resolve(reportDir, `${slice}.json`)}`,
        '--quiet',
      ];

  const result = spawnSync(
    npx,
    ['--yes', `--package=${tjsvPackage}`, 'tjsv', ...tjsvArgs],
    { cwd: repoRoot, stdio: 'inherit' },
  );

  if (result.error) {
    console.error(`[tjsv-slices] ✗ ${slice}: ${result.error.message}`);
    failed.push(slice);
    continue;
  }
  if (result.status !== 0) failed.push(slice);
}

console.log(
  `\n[tjsv-slices] ${slices.length - failed.length}/${slices.length} slices passed ${command}`,
);
if (failed.length) {
  console.error(`[tjsv-slices] failed: ${failed.join(', ')}`);
  process.exit(2);
}
