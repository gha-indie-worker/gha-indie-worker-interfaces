#!/usr/bin/env node
// Run ORESoftware/typespec-json-schema-validator (TJSV) over every independently
// authored TypeSpec + JSON Schema slice. TJSV is a second, fail-closed gate next
// to ores-contracts: it compares official TypeSpec JSON Schema witness evidence
// against the human-authored Draft 2020-12 authority and executes both as
// validators over deterministic probes plus this repository's recorded corpus.
//
// The two tools answer different questions. ores-contracts checks the
// persistence/code-generation projection. TJSV checks the language/runtime data
// boundary and emits deterministic receipts/Contract IR evidence. Neither tool
// may rewrite or rank either authored authority.
//
// TJSV is pinned to an immutable GitHub commit tarball instead of a moving npm
// tag. Runner environments can override the package spec with TJSV_PACKAGE for
// controlled upgrade testing, but CI uses the immutable default below.
//
//   node scripts/tjsv-slices.mjs [check|inventory] [--only slice,slice]
//
// Exit code 0 only when every selected slice passes.
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { basename, dirname, join, relative, resolve } from 'node:path';
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

// Current reviewed TJSV main: language/runtime boundary lockstep + fail-closed
// multi-file reference normalization. Keep this as an exact 40-char commit.
const tjsvRevision = 'a4b731fbf82c4d162abd74fd03758fa32bb76176';
const tjsvPackage = process.env.TJSV_PACKAGE
  ?? `https://github.com/ORESoftware/typespec-json-schema-validator/archive/${tjsvRevision}.tar.gz`;
const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const evidenceRoot = resolve(repoRoot, '.typespec-json-schema-validator');
const reportDir = resolve(evidenceRoot, 'reports');
const mappingDir = resolve(evidenceRoot, 'mappings');
const instanceRoot = resolve(evidenceRoot, 'instances');
const generatedRoot = resolve(evidenceRoot, 'generated');
const contractIrDir = resolve(evidenceRoot, 'contract-ir');
for (const dir of [reportDir, mappingDir, instanceRoot, generatedRoot, contractIrDir]) {
  mkdirSync(dir, { recursive: true });
}

// `uuid` and `json` are persistence-parser helper declarations in ores.tsp, not
// product declarations. TJSV's ignore is deliberately explicit and audited:
// stale or misspelled ignore entries fail closed. `RecordUnknown` is an official
// emitter helper produced only for slices that use Record<unknown>.
const recordUnknownSlices = new Set(['errors', 'sync', 'transport', 'webhooks']);

function writeMapping(slice) {
  const mapping = {
    schema: 'ores.typespec-json-schema-validator.mapping/v1',
    declarations: [],
    ignore: {
      typespec: ['Ores.json', 'Ores.uuid'],
      generated: ['uuid', ...(recordUnknownSlices.has(slice) ? ['RecordUnknown'] : [])],
      authored: [],
    },
  };
  const path = resolve(mappingDir, `${slice}.json`);
  writeFileSync(path, `${JSON.stringify(mapping, null, 2)}\n`);
  return path;
}

function walkJsonFiles(root) {
  if (!existsSync(root)) return [];
  const out = [];
  const stack = [root];
  while (stack.length) {
    const current = stack.pop();
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) stack.push(path);
      else if (entry.isFile() && entry.name.endsWith('.json')) out.push(path);
    }
  }
  return out.sort();
}

function materializeInstances(slice) {
  const source = resolve(repoRoot, 'contracts', 'fixtures', slice);
  const destination = resolve(instanceRoot, slice);
  rmSync(destination, { recursive: true, force: true });
  mkdirSync(destination, { recursive: true });

  let copied = 0;
  for (const file of walkJsonFiles(source)) {
    const rel = relative(source, file).split('\\').join('/');
    const segments = rel.split('/');
    const verdict = segments.includes('valid') ? 'valid' : segments.includes('invalid') ? 'invalid' : null;
    if (!verdict) continue;

    // Fixture naming is `<Declaration>.<case>.json`; keep the declaration as the
    // first path component required by TJSV's corpus contract.
    const model = basename(file).split('.')[0];
    if (!model) continue;
    const targetDir = resolve(destination, model, verdict);
    mkdirSync(targetDir, { recursive: true });
    const safeName = rel.replaceAll('/', '__');
    copyFileSync(file, resolve(targetDir, safeName));
    copied += 1;
  }
  return { destination, copied };
}

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
    : (() => {
        const mapping = writeMapping(slice);
        const corpus = materializeInstances(slice);
        console.log(`[tjsv-slices] ${slice}: materialized ${corpus.copied} recorded instances`);
        return [
          'check',
          `--typespec=${typespec}`,
          `--schema=${schema}`,
          `--mapping=${mapping}`,
          `--instances=${corpus.destination}`,
          `--output-dir=${resolve(generatedRoot, slice)}`,
          `--report=${resolve(reportDir, `${slice}.json`)}`,
          `--contract-ir=${resolve(contractIrDir, `${slice}.json`)}`,
          '--int64-strategy=number',
          '--seal-object-schemas=false',
          '--format-assertion=true',
          '--probes=true',
          '--max-probes=96',
          '--quiet',
        ];
      })();

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
