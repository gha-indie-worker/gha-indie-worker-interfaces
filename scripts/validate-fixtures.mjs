#!/usr/bin/env node
// Validate every fixture in contracts/fixtures against the JSON Schema authority.
//
//   node scripts/validate-fixtures.mjs [--only slice,slice] [--engine ajv|builtin]
//
// Fixture layout and naming:
//
//   contracts/fixtures/<slice>/valid/<Model>.<case>.json          must validate
//   contracts/fixtures/<slice>/invalid/<Model>.<case>.json        must NOT validate,
//                                                                 and must also be rejected by serde
//                                                                 (tests/fixtures_roundtrip.rs)
//   contracts/fixtures/<slice>/invalid/schema-only/<Model>.<case>.json
//                                                                 must NOT validate here, but IS a
//                                                                 well-formed serde document (it only
//                                                                 breaks a value bound), so the Rust
//                                                                 round-trip test skips it
//
// The `<Model>` prefix names the entry in the slice schema's `$defs`.
//
// Engine: ajv (2020-12) when it is installed, otherwise the dependency-free
// validator below. Both implement the same subset and must agree — run with
// --engine builtin in CI as well to keep the fallback honest.
//
// Supported subset (see contracts/README.md):
//   $ref (#/$defs/*), type, required, properties, additionalProperties (false or a schema),
//   enum, const, items, minItems, maxItems, minLength, maxLength, minimum, maximum,
//   pattern, oneOf, and format-lite over uuid / date-time / date / byte.
// Anything else in an authority is a bug: this validator fails closed on unknown keywords.
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join, resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const fixturesRoot = join(repoRoot, 'contracts', 'fixtures');
const schemaRoot = join(repoRoot, 'contracts', 'json-schema');

// ---------------------------------------------------------------- format-lite
const FORMATS = {
  uuid: /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/,
  'date-time': /^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(\.\d+)?([Zz]|[+-]\d{2}:\d{2})$/,
  date: /^\d{4}-\d{2}-\d{2}$/,
  byte: /^[A-Za-z0-9+/]*={0,2}$/,
};

const KNOWN_KEYWORDS = new Set([
  '$ref', '$comment', 'title', 'description', 'default', 'examples', 'deprecated',
  'type', 'required', 'properties', 'additionalProperties', 'enum', 'const',
  'items', 'minItems', 'maxItems', 'minLength', 'maxLength', 'minimum', 'maximum',
  'pattern', 'format', 'oneOf',
]);

// -------------------------------------------------------- dependency-free core
function typeOf(value) {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  if (Number.isInteger(value)) return 'integer';
  return typeof value === 'number' ? 'number' : typeof value;
}

function typeMatches(expected, actual) {
  if (expected === 'number') return actual === 'number' || actual === 'integer';
  return expected === actual;
}

function deref(schema, root, where, errors) {
  let seen = 0;
  while (schema && typeof schema.$ref === 'string') {
    if (++seen > 16) { errors.push(`${where}: $ref chain too deep`); return null; }
    const m = /^#\/\$defs\/([A-Za-z0-9_-]+)$/.exec(schema.$ref);
    if (!m) { errors.push(`${where}: unsupported $ref ${schema.$ref}`); return null; }
    const target = (root.$defs ?? {})[m[1]];
    if (!target) { errors.push(`${where}: $ref points at missing $defs/${m[1]}`); return null; }
    const { $ref, ...rest } = schema;
    schema = Object.keys(rest).length ? { ...target, ...rest } : target;
  }
  return schema;
}

/** Validate `value` against `schema`. Pushes human-readable messages into `errors`. */
function validateNode(schema, value, root, where, errors) {
  if (schema === true) return;
  if (schema === false) { errors.push(`${where}: schema is false`); return; }
  const s = deref(schema, root, where, errors);
  if (!s) return;

  for (const key of Object.keys(s)) {
    if (!KNOWN_KEYWORDS.has(key) && !key.startsWith('x-ores-') && key !== '$defs' && key !== '$schema' && key !== '$id') {
      errors.push(`${where}: unsupported keyword ${key} (outside the contract subset)`);
    }
  }

  const actual = typeOf(value);
  if (s.type !== undefined) {
    const expected = Array.isArray(s.type) ? s.type : [s.type];
    if (!expected.some((t) => typeMatches(t, actual))) {
      errors.push(`${where}: expected ${expected.join(' or ')}, got ${actual}`);
      return;
    }
  }
  if (s.const !== undefined && JSON.stringify(value) !== JSON.stringify(s.const)) {
    errors.push(`${where}: expected const ${JSON.stringify(s.const)}`);
  }
  if (Array.isArray(s.enum) && !s.enum.some((v) => JSON.stringify(v) === JSON.stringify(value))) {
    errors.push(`${where}: ${JSON.stringify(value)} is not one of ${JSON.stringify(s.enum)}`);
  }

  if (actual === 'string') {
    if (s.minLength !== undefined && [...value].length < s.minLength) errors.push(`${where}: shorter than minLength ${s.minLength}`);
    if (s.maxLength !== undefined && [...value].length > s.maxLength) errors.push(`${where}: longer than maxLength ${s.maxLength}`);
    if (s.pattern !== undefined && !new RegExp(s.pattern, 'u').test(value)) errors.push(`${where}: does not match pattern ${s.pattern}`);
    if (s.format !== undefined) {
      const re = FORMATS[s.format];
      if (!re) errors.push(`${where}: unsupported format ${s.format}`);
      else if (!re.test(value)) errors.push(`${where}: not a valid ${s.format}`);
    }
  }

  if (actual === 'number' || actual === 'integer') {
    if (s.minimum !== undefined && value < s.minimum) errors.push(`${where}: below minimum ${s.minimum}`);
    if (s.maximum !== undefined && value > s.maximum) errors.push(`${where}: above maximum ${s.maximum}`);
  }

  if (actual === 'array') {
    if (s.minItems !== undefined && value.length < s.minItems) errors.push(`${where}: fewer than minItems ${s.minItems}`);
    if (s.maxItems !== undefined && value.length > s.maxItems) errors.push(`${where}: more than maxItems ${s.maxItems}`);
    if (s.items !== undefined) value.forEach((item, i) => validateNode(s.items, item, root, `${where}[${i}]`, errors));
  }

  if (actual === 'object') {
    for (const key of s.required ?? []) {
      if (!Object.prototype.hasOwnProperty.call(value, key)) errors.push(`${where}: missing required property ${key}`);
    }
    const props = s.properties ?? {};
    for (const [key, child] of Object.entries(value)) {
      if (Object.prototype.hasOwnProperty.call(props, key)) {
        validateNode(props[key], child, root, `${where}.${key}`, errors);
      } else if (s.additionalProperties === false) {
        errors.push(`${where}: additional property ${key} is not allowed`);
      } else if (s.additionalProperties && typeof s.additionalProperties === 'object') {
        validateNode(s.additionalProperties, child, root, `${where}.${key}`, errors);
      }
    }
  }

  if (Array.isArray(s.oneOf)) {
    const matched = [];
    for (const [i, branch] of s.oneOf.entries()) {
      const branchErrors = [];
      validateNode(branch, value, root, `${where}#oneOf/${i}`, branchErrors);
      if (!branchErrors.length) matched.push(i);
    }
    if (matched.length !== 1) {
      errors.push(`${where}: expected exactly one oneOf branch to match, ${matched.length} did`);
    }
  }
}

function builtinValidator(doc, model) {
  return (instance) => {
    const errors = [];
    const target = (doc.$defs ?? {})[model];
    if (!target) return [`schema has no $defs/${model}`];
    validateNode(target, instance, doc, model, errors);
    return errors;
  };
}

// ------------------------------------------------------------------ ajv engine
async function loadAjv() {
  try {
    const mod = await import('ajv/dist/2020.js');
    const Ajv = mod.default?.default ?? mod.default ?? mod.Ajv2020;
    const ajv = new Ajv({ strict: false, allErrors: true, allowUnionTypes: true });
    for (const [name, re] of Object.entries(FORMATS)) ajv.addFormat(name, re);
    return ajv;
  } catch {
    return null;
  }
}

function ajvValidator(ajv, doc, model) {
  const validate = ajv.compile({ $defs: doc.$defs, $ref: `#/$defs/${model}` });
  return (instance) => (validate(instance) ? [] : (validate.errors ?? []).map((e) => `${model}${e.instancePath}: ${e.message}`));
}

// ------------------------------------------------------------------------ walk
function walkJson(dir) {
  if (!existsSync(dir)) return [];
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walkJson(full));
    else if (name.endsWith('.json')) out.push(full);
  }
  return out;
}

function modelOf(file) {
  const stem = basename(file, '.json');
  const model = stem.split('.')[0];
  if (!/^[A-Z][A-Za-z0-9]*$/.test(model)) {
    throw new Error(`fixture ${file}: name must start with the $defs model, e.g. Org.minimal.json`);
  }
  return model;
}

// ------------------------------------------------------------------------ main
const argv = process.argv.slice(2);
const onlyIdx = argv.indexOf('--only');
const only = onlyIdx >= 0 && argv[onlyIdx + 1] ? new Set(argv[onlyIdx + 1].split(',')) : null;
const engineIdx = argv.indexOf('--engine');
const requested = engineIdx >= 0 ? argv[engineIdx + 1] : 'auto';

const ajv = requested === 'builtin' ? null : await loadAjv();
if (requested === 'ajv' && !ajv) {
  console.error('[fixtures] --engine ajv requested but ajv is not installed');
  process.exit(1);
}
const engine = ajv ? 'ajv' : 'builtin';
console.log(`[fixtures] engine: ${engine}`);

if (!existsSync(fixturesRoot)) {
  console.error(`[fixtures] missing ${fixturesRoot}`);
  process.exit(1);
}

let checked = 0;
const failures = [];
const slices = readdirSync(fixturesRoot).filter((s) => statSync(join(fixturesRoot, s)).isDirectory()).sort();
for (const slice of slices) {
  if (only && !only.has(slice)) continue;
  const schemaPath = join(schemaRoot, `${slice}.schema.json`);
  if (!existsSync(schemaPath)) {
    failures.push(`${slice}: no JSON Schema authority at contracts/json-schema/${slice}.schema.json`);
    continue;
  }
  const doc = JSON.parse(readFileSync(schemaPath, 'utf8'));
  const cache = new Map();
  const validatorFor = (model) => {
    if (!cache.has(model)) cache.set(model, ajv ? ajvValidator(ajv, doc, model) : builtinValidator(doc, model));
    return cache.get(model);
  };

  for (const [kind, dir] of [['valid', join(fixturesRoot, slice, 'valid')], ['invalid', join(fixturesRoot, slice, 'invalid')]]) {
    for (const file of walkJson(dir)) {
      checked += 1;
      const rel = file.slice(repoRoot.length + 1);
      let instance;
      try {
        instance = JSON.parse(readFileSync(file, 'utf8'));
      } catch (e) {
        failures.push(`${rel}: not parseable JSON — ${e.message}`);
        continue;
      }
      let errors;
      try {
        errors = validatorFor(modelOf(file))(instance);
      } catch (e) {
        failures.push(`${rel}: ${e.message}`);
        continue;
      }
      if (kind === 'valid' && errors.length) failures.push(`${rel}: expected valid, got ${errors.length} error(s)\n      ${errors.slice(0, 6).join('\n      ')}`);
      if (kind === 'invalid' && !errors.length) failures.push(`${rel}: expected INVALID, but it validated`);
    }
  }
}

console.log(`[fixtures] checked ${checked} fixtures across ${slices.length} slices`);
if (failures.length) {
  for (const f of failures) console.error(`[fixtures] ✗ ${f}`);
  console.error(`[fixtures] ${failures.length} failure(s)`);
  process.exit(2);
}
console.log('[fixtures] ok');
