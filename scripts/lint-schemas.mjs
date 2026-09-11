#!/usr/bin/env node
// ajv in STRICT MODE, as a lint step.
//
// This is not validation -- scripts/check_fixtures.py validates. This compiles
// every hand-authored schema document and fails on the mistakes that make a
// schema *weaker than it looks*, which ordinary validation cannot catch because
// a weaker schema simply accepts more:
//
//   * a keyword that does nothing where it sits (`maxLength` on an integer,
//     `items` on an object) -- it reads like a constraint and enforces nothing
//   * a `$ref` to a `$defs` that does not exist
//   * an unknown `format`, which ajv otherwise ignores silently
//   * a `required` naming a property the schema does not declare
//
// Every one of those is a constraint a reviewer believes is enforced and is not,
// which is exactly the failure mode a two-authority model is built to prevent.
import { readFileSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const CONFIG = JSON.parse(readFileSync(join(ROOT, 'contracts', 'parity.config.json'), 'utf8'));

const ajv = new Ajv2020({
  strict: true,
  strictSchema: true,
  strictTypes: true,
  strictTuples: true,
  strictRequired: true,
  allowUnionTypes: false,
  allErrors: true,
  validateFormats: true,
});
addFormats(ajv);

// The vendor keywords both authorities are allowed to carry. Declaring them is
// what lets strict mode reject every OTHER unknown keyword -- a typo'd
// `maxLenght` must be an error, not a silently ignored annotation.
for (const keyword of [
  'x-ores-namespace', 'x-ores-table', 'x-ores-primary-key', 'x-ores-unique',
  'x-ores-indexes', 'x-ores-references', 'x-ores-width', 'x-ores-json',
  'x-protocol-version', 'x-max-frame-bytes',
]) ajv.addKeyword({ keyword, metaSchema: {} });

// `byte` is base64 in a JSON string; ajv-formats does not ship it, and an
// undeclared format is silently ignored, so it is declared here with the same
// rule scripts/check_fixtures.py enforces on the Python side.
ajv.addFormat('byte', { type: 'string', validate: (s) => /^[A-Za-z0-9+/]*={0,2}$/.test(s) && s.length % 4 === 0 });

let failures = 0;
for (const [key, path] of Object.entries(CONFIG.jsonSchema.documents)) {
  const doc = JSON.parse(readFileSync(join(ROOT, path), 'utf8'));
  try {
    ajv.compile(doc);
    const models = Object.values(doc.$defs ?? {}).filter((d) => d.type === 'object').length;
    const enums = Object.values(doc.$defs ?? {}).filter((d) => Array.isArray(d.enum)).length;
    process.stdout.write(`[schema-lint] ${key}: ok (${models} models, ${enums} enums)\n`);
  } catch (e) {
    failures += 1;
    process.stdout.write(`[schema-lint] ${key} (${path}): ${e.message}\n`);
  }
}
process.stdout.write(failures ? `[schema-lint] ${failures} document(s) failed strict compilation\n` : '[schema-lint] all documents compile under ajv strict mode\n');
process.exit(failures ? 1 : 0);
