#!/usr/bin/env node
// ============================================================================
// The parity gate.
//
// TypeSpec and JSON Schema are two independent, peer, human-authored
// descriptions of the same vocabulary. This script is the only thing that makes
// that claim testable: it derives a normalized shape from each authority
// *separately*, diffs them, runs the same fixture corpus through both, checks
// the protobuf wire types against both, and exits non-zero listing every
// discrepancy it found.
//
// It NEVER reconciles. There is no "prefer typespec", no "fill in from the other
// side", no auto-fix flag. A discrepancy is a finding with a stable fingerprint
// and a human decides which authority was wrong -- because the whole value of
// two authorities is that a mistake in one of them becomes visible, and a gate
// that silently picked a winner would destroy exactly that.
//
//   node scripts/parity.mjs             # every lane; a missing tool is a FINDING
//   node scripts/parity.mjs --offline   # tool-dependent lanes report SKIPPED loudly,
//                                       # and the gate still fails on real discrepancies
//   node scripts/parity.mjs --json      # receipt on stdout
//
// Lanes
//   tsp-compile    `tsp compile --warn-as-error`: the TypeSpec authority is well-formed [tsp]
//   tsp-emit       `tsp compile --emit @typespec/json-schema` into target/parity/       [tsp]
//   shape-emitted  the emitted schema vs the hand-authored schema, field by field       [tsp]
//   shape-source   the .tsp SOURCE vs the hand-authored schema, field by field       [always]
//   fixtures-js    every fixture vs the hand-authored schema (check_fixtures.py)    [always]
//   fixtures-tsp   the same corpus vs the schema TypeSpec emitted; verdicts must agree  [tsp]
//   proto          proto field JSON names, types and presence vs BOTH authorities   [always]
//   published      schema/v1/*.json still equals its $defs peer in contracts/       [always]
//   ores-contracts `npx ores-contracts check`: persisted subset -> SQL/ORM byte parity [npm]
//
// `shape-source` and `shape-emitted` are deliberately two different mechanisms
// reading the same file. If the reader here and the real compiler ever disagree
// about what a declaration means, that disagreement is itself a finding.
// ============================================================================
import { readFileSync, writeFileSync, mkdirSync, rmSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { resolve, join, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const sha = (s) => createHash('sha256').update(s).digest('hex');
const rel = (p) => relative(ROOT, p) || '.';

const argv = new Set(process.argv.slice(2));
const OFFLINE = argv.has('--offline');
const AS_JSON = argv.has('--json');

const findings = [];
const lanes = {};

function finding(kind, detail, where = null) {
  findings.push({ kind, detail, where, fingerprint: sha(`${kind} ${where ?? ''} ${detail}`).slice(0, 16) });
}
function lane(name, status, note = null) {
  lanes[name] = { status, note };
}
const log = (msg) => { if (!AS_JSON) process.stdout.write(`${msg}\n`); };

const CONFIG = JSON.parse(readFileSync(join(ROOT, 'contracts', 'parity.config.json'), 'utf8'));
const TARGET = join(ROOT, CONFIG.target);

// ===========================================================================
// 1. A string-aware TypeSpec reader for the subset both authorities may use.
//
// Regex on raw source is not good enough here. `@doc("...removed; an org...")`
// contains a semicolon and a comment in http.tsp contains a quote, so a naive
// splitter mis-reads real declarations in this repo. This scanner tracks string
// state, so it either understands a construct exactly or refuses it. Anything it
// refuses is REPORTED as a finding, never skipped.
// ===========================================================================
function scan(source) {
  const out = [...source];
  const strings = [];
  let i = 0;
  while (i < source.length) {
    const c = source[i];
    if (c === '"') {
      const start = i;
      i += 1;
      while (i < source.length && source[i] !== '"') i += source[i] === '\\' ? 2 : 1;
      i += 1;
      strings.push([start, i]);
      continue;
    }
    if (c === '/' && source[i + 1] === '/') {
      while (i < source.length && source[i] !== '\n') { out[i] = ' '; i += 1; }
      continue;
    }
    if (c === '/' && source[i + 1] === '*') {
      const end = source.indexOf('*/', i + 2);
      const stop = end === -1 ? source.length : end + 2;
      for (let k = i; k < stop; k += 1) if (out[k] !== '\n') out[k] = ' ';
      i = stop;
      continue;
    }
    i += 1;
  }
  return { text: out.join(''), strings };
}

function inString(strings, index) {
  for (const [a, b] of strings) if (index >= a && index < b) return true;
  return false;
}

/**
 * Split a declaration body on a top-level separator, outside strings and brackets.
 * `offset` is where `text` starts in the file the `strings` ranges were measured in.
 *
 * The string-awareness is load-bearing, not defensive: `@doc("... removed; an org
 * without an owner ...")` really does contain a `;`, and every enum member in
 * http.tsp carries a `@doc(...)` whose prose contains a `,`.
 */
function splitTop(text, strings, offset, sep) {
  const parts = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    if (inString(strings, offset + i)) continue;
    const c = text[i];
    if (c === '(' || c === '{' || c === '[') depth += 1;
    else if (c === ')' || c === '}' || c === ']') depth -= 1;
    else if (c === sep && depth === 0) { parts.push([text.slice(start, i), offset + start]); start = i + 1; }
  }
  parts.push([text.slice(start), offset + start]);
  return parts.filter(([s]) => s.trim());
}

/**
 * Consume the leading `@deco(...)` run of a statement and return it separately.
 *
 * Doing this BEFORE matching `name?: type` is what makes the reader correct:
 * decorator arguments are prose that contains colons (`runtime::tenancy`), colons
 * inside patterns (`^sha256:[0-9a-f]+$`) and quotes, and a regex applied to the
 * whole statement matches inside them instead of on the real declaration.
 */
function splitDecorators(text, strings, offset) {
  let i = 0;
  for (;;) {
    while (i < text.length && /\s/.test(text[i])) i += 1;
    if (text[i] !== '@' || inString(strings, offset + i)) break;
    i += 1;
    while (i < text.length && /[A-Za-z0-9_.]/.test(text[i])) i += 1;
    while (i < text.length && /\s/.test(text[i])) i += 1;
    if (text[i] !== '(') continue;
    let depth = 0;
    for (; i < text.length; i += 1) {
      if (inString(strings, offset + i)) continue;
      if (text[i] === '(') depth += 1;
      else if (text[i] === ')') { depth -= 1; if (depth === 0) { i += 1; break; } }
    }
  }
  return { prefix: text.slice(0, i), rest: text.slice(i).trim() };
}

/** Yield each `keyword Name { ... }` block, brace-matched outside strings. */
function* blocks(text, strings, keyword) {
  const re = new RegExp(`\\b${keyword}\\s+([A-Za-z_][A-Za-z0-9_]*)(<[^>]*>)?\\s*\\{`, 'g');
  let m;
  while ((m = re.exec(text)) !== null) {
    if (inString(strings, m.index)) continue;
    const open = m.index + m[0].length - 1;
    let depth = 0;
    let i = open;
    for (; i < text.length; i += 1) {
      if (inString(strings, i)) continue;
      if (text[i] === '{') depth += 1;
      else if (text[i] === '}') { depth -= 1; if (depth === 0) break; }
    }
    const before = text.slice(0, m.index);
    const declStart = before.search(/(?:@[A-Za-z_][A-Za-z0-9_.]*(?:\((?:[^()]|\([^()]*\))*\))?\s*)+$/);
    yield {
      name: m[1],
      generic: !!m[2],
      decorators: declStart === -1 ? '' : before.slice(declStart),
      body: text.slice(open + 1, i),
      bodyOffset: open + 1,
    };
    re.lastIndex = i;
  }
}

function decoratorsOf(prefix) {
  const out = [];
  const re = /@([A-Za-z_][A-Za-z0-9_.]*)(\((?:[^()"]|"(?:[^"\\]|\\.)*"|\((?:[^()]|"(?:[^"\\]|\\.)*")*\))*\))?/g;
  let m;
  while ((m = re.exec(prefix)) !== null) {
    const raw = m[2] ? m[2].slice(1, -1) : undefined;
    const args = [];
    if (raw !== undefined) {
      const ar = /"((?:[^"\\]|\\.)*)"|(-?\d+(?:\.\d+)?)/g;
      let a;
      while ((a = ar.exec(raw)) !== null) args.push(a[1] !== undefined ? a[1].replace(/\\(.)/g, '$1') : Number(a[2]));
    }
    out.push({ name: m[1], args, raw });
  }
  return out;
}

// TypeSpec scalar -> JSON type + format. The only mapping both authorities use.
//
// Integer WIDTH is deliberately absent. JSON has one number type, so int32 vs
// int64 is not a difference the JSON wire shape can carry, and asserting it here
// would manufacture a discrepancy out of a distinction the format cannot express.
// The width question is real and it is decided in the `ores-contracts` lane,
// which reads `int64` on one side and `x-ores-width: 64` on the other and
// byte-compares the SQL and Rust each lane emits. This routes the check to the
// lane that can settle it; it does not drop it. See contracts/README.md.
const TSP_JSON_TYPE = {
  string: ['string', null],
  uuid: ['string', 'uuid'],
  utcDateTime: ['string', 'date-time'],
  plainDate: ['string', 'date'],
  bytes: ['string', 'byte'],
  int32: ['integer', null],
  int64: ['integer', null],
  float64: ['number', null],
  float32: ['number', null],
  boolean: ['boolean', null],
};

function parseTypeSpecShape(files) {
  const enums = {};
  const models = {};
  const unions = {};
  const constants = {};
  const unparsed = [];

  for (const file of files) {
    const raw = readFileSync(join(ROOT, file), 'utf8');
    const { text, strings } = scan(raw);

    for (const m of text.matchAll(/\bconst\s+([A-Z][A-Z0-9_]*)\s*=\s*(-?\d+)\s*;/g)) {
      if (!inString(strings, m.index)) constants[m[1]] = Number(m[2]);
    }

    for (const b of blocks(text, strings, 'enum')) {
      const values = [];
      for (const [member, at] of splitTop(b.body, strings, b.bodyOffset, ',')) {
        const { rest } = splitDecorators(member, strings, at);
        if (!rest) continue;
        const v = rest.match(/^([A-Za-z_][A-Za-z0-9_]*)\s*:\s*"((?:[^"\\]|\\.)*)"$/) || rest.match(/^([A-Za-z_][A-Za-z0-9_]*)$/);
        if (!v) { unparsed.push(`${file}: enum ${b.name} member \`${rest.replace(/\s+/g, ' ').slice(0, 60)}\``); continue; }
        values.push(v[2] !== undefined ? v[2] : v[1]);
      }
      enums[b.name] = values;
    }

    for (const b of blocks(text, strings, 'union')) {
      const members = [];
      for (const [member, at] of splitTop(b.body, strings, b.bodyOffset, ',')) {
        const { rest } = splitDecorators(member, strings, at);
        if (!rest) continue;
        const v = rest.match(/^[A-Za-z_][A-Za-z0-9_]*\s*:\s*([A-Za-z_][A-Za-z0-9_.]*)$/) || rest.match(/^([A-Za-z_][A-Za-z0-9_.]*)$/);
        if (!v) { unparsed.push(`${file}: union ${b.name} member \`${rest.slice(0, 60)}\``); continue; }
        members.push(v[1].split('.').pop());
      }
      unions[b.name] = members;
    }

    for (const b of blocks(text, strings, 'model')) {
      if (b.generic) { models[b.name] = { generic: true, fields: {} }; continue; }
      const fields = {};
      for (const [stmt, at] of splitTop(b.body, strings, b.bodyOffset, ';')) {
        const { prefix, rest } = splitDecorators(stmt, strings, at);
        if (!rest || rest.startsWith('...')) continue; // spread of an HTTP query bag
        const fm = rest.match(/^([A-Za-z_][A-Za-z0-9_]*)(\?)?\s*:\s*([\s\S]+)$/);
        if (!fm) { unparsed.push(`${file}: model ${b.name} field \`${rest.replace(/\s+/g, ' ').slice(0, 70)}\``); continue; }
        const [, fname, optional, rawType] = fm;
        const decos = decoratorsOf(prefix);
        const field = { optional: !!optional, array: false, type: null, format: null, enum: null, const: null };
        let t = rawType.trim();
        if (t.endsWith('[]')) { field.array = true; t = t.slice(0, -2).trim(); }
        const literal = t.match(/^"((?:[^"\\]|\\.)*)"$/);
        const numberUnion = /^-?\d+(\s*\|\s*-?\d+)+$/.test(t);
        const stringUnion = /^"(?:[^"\\]|\\.)*"(\s*\|\s*"(?:[^"\\]|\\.)*")+$/.test(t);
        const bare = t.split('.').pop();
        if (literal) { field.type = 'string'; field.const = literal[1]; }
        else if (/^-?\d+$/.test(t)) { field.type = 'integer'; field.const = Number(t); }
        else if (numberUnion) { field.type = 'integer'; field.enum = t.split('|').map((x) => Number(x.trim())); }
        else if (stringUnion) { field.type = 'string'; field.enum = [...t.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((x) => x[1]); }
        else if (/^Record\s*<|^unknown$/.test(t)) { field.type = 'object'; }
        else if (TSP_JSON_TYPE[bare]) { [field.type, field.format] = TSP_JSON_TYPE[bare]; }
        else if (enums[bare]) { field.type = 'string'; field.enum = [...enums[bare]]; field.enumName = bare; }
        else { field.type = 'ref'; field.ref = bare; }
        for (const d of decos) {
          if (d.name === 'minLength') field.minLength = d.args[0];
          if (d.name === 'maxLength') field.maxLength = d.args[0];
          if (d.name === 'minValue') field.minimum = d.args[0];
          if (d.name === 'maxValue') field.maximum = d.args[0];
          if (d.name === 'minItems') field.minItems = d.args[0];
          if (d.name === 'maxItems') field.maxItems = d.args[0];
          if (d.name === 'pattern') field.pattern = d.args[0];
          if (d.name === 'statusCode' || d.name === 'header' || d.name === 'query' || d.name === 'path') field.httpBinding = d.name;
        }
        fields[fname] = field;
      }
      models[b.name] = { generic: false, fields };
    }
  }
  return { enums, models, unions, constants, unparsed };
}

// ===========================================================================
// 2. The same normalized shape, read out of a JSON Schema 2020-12 document.
// ===========================================================================
function jsonSchemaShape(documents) {
  const enums = {};
  const models = {};
  const unions = {};
  const owner = {};

  const deref = (doc, node) => {
    let n = node;
    let hops = 0;
    while (n && typeof n.$ref === 'string' && hops < 8) {
      const name = n.$ref.replace(/^#\/\$defs\//, '').replace(/\.json$/, '');
      const next = doc.$defs?.[name];
      if (!next) return { ...n, __unresolved: n.$ref };
      n = { ...next, __refName: name };
      hops += 1;
    }
    return n ?? {};
  };

  for (const [key, doc] of Object.entries(documents)) {
    for (const [name, def] of Object.entries(doc.$defs ?? {})) {
      owner[name] = key;
      if (Array.isArray(def.enum) && def.type === 'string') { enums[name] = [...def.enum]; continue; }
      if (Array.isArray(def.oneOf)) {
        unions[name] = def.oneOf.map((o) => String(o.$ref ?? '').replace(/^#\/\$defs\//, '').replace(/\.json$/, ''));
        continue;
      }
      if (def.type !== 'object') continue;
      const required = new Set(def.required ?? []);
      const fields = {};
      for (const [fname, p0] of Object.entries(def.properties ?? {})) {
        const field = { optional: !required.has(fname), array: false, type: null, format: null, enum: null, const: null };
        let p = deref(doc, p0);
        if (p.type === 'array') {
          field.array = true;
          if (p.maxItems !== undefined) field.maxItems = p.maxItems;
          if (p.minItems !== undefined) field.minItems = p.minItems;
          p = deref(doc, p.items ?? {});
        }
        if (p.__unresolved) { field.type = 'ref'; field.ref = String(p.__unresolved); }
        else if (Array.isArray(p.enum) && p.type === 'string') { field.type = 'string'; field.enum = [...p.enum]; if (p.__refName) field.enumName = p.__refName; }
        else if (Array.isArray(p.enum) && p.type === 'integer') { field.type = 'integer'; field.enum = [...p.enum]; }
        else if (p.const !== undefined) { field.type = p.type ?? typeof p.const; field.const = p.const; }
        // A `$ref`'d object is a named model; an INLINE `{"type":"object"}` with no
        // properties is a deliberately free-form payload. They are different types
        // and are recorded as different types, so one can never pass for the other.
        else if (p.type === 'object') {
          if (p.__refName) { field.type = 'ref'; field.ref = p.__refName; }
          else if (p.properties === undefined) { field.type = 'object'; }
          else { field.type = 'ref'; field.ref = null; }
        }
        else { field.type = p.type ?? null; field.format = p.format ?? null; }
        if (p.minLength !== undefined) field.minLength = p.minLength;
        if (p.maxLength !== undefined) field.maxLength = p.maxLength;
        if (p.pattern !== undefined) field.pattern = p.pattern;
        if (p.minimum !== undefined) field.minimum = p.minimum;
        if (p.maximum !== undefined) field.maximum = p.maximum;
        fields[fname] = field;
      }
      models[name] = { generic: false, fields, sealed: def.additionalProperties === false, document: key };
    }
  }
  return { enums, models, unions, owner };
}

// ===========================================================================
// 3. Diff the two shapes. Every difference is a finding; nothing is merged.
// ===========================================================================
const COMPARED = ['type', 'format', 'optional', 'array', 'const', 'minLength', 'maxLength', 'minimum', 'maximum', 'pattern', 'maxItems', 'minItems'];
const show = (v) => (v === undefined ? 'unset' : JSON.stringify(v));

function diffShapes(tsp, js, label, only) {
  const orphans = { typespec: [], jsonSchema: [] };

  for (const name of new Set([...Object.keys(tsp.enums), ...Object.keys(js.enums)])) {
    const a = tsp.enums[name];
    const b = js.enums[name];
    if (!a) { orphans.jsonSchema.push(`enum ${name}`); continue; }
    if (!b) { orphans.typespec.push(`enum ${name}`); continue; }
    if (JSON.stringify(a) !== JSON.stringify(b)) {
      finding('authority-parity', `enum ${name}: values differ - typespec=${JSON.stringify(a)} json-schema=${JSON.stringify(b)}`, label);
    }
  }

  for (const name of new Set([...Object.keys(tsp.unions), ...Object.keys(js.unions)])) {
    const a = tsp.unions[name];
    const b = js.unions[name];
    if (!a) { orphans.jsonSchema.push(`union ${name}`); continue; }
    if (!b) { orphans.typespec.push(`union ${name}`); continue; }
    if (JSON.stringify([...a].sort()) !== JSON.stringify([...b].sort())) {
      finding('authority-parity', `union ${name}: members differ - typespec=${JSON.stringify(a)} json-schema=${JSON.stringify(b)}`, label);
    }
  }

  for (const name of new Set([...Object.keys(tsp.models), ...Object.keys(js.models)])) {
    const a = tsp.models[name];
    const b = js.models[name];
    if (!a) { orphans.jsonSchema.push(`model ${name}`); continue; }
    if (!b) { orphans.typespec.push(`model ${name}`); continue; }
    if (a.generic) continue;
    if (b.sealed === false) {
      finding('authority-parity', `model ${name}: the JSON Schema does not set additionalProperties:false. TypeSpec models are closed, so an open peer describes a different type`, label);
    }
    for (const fname of new Set([...Object.keys(a.fields), ...Object.keys(b.fields)])) {
      const f = a.fields[fname];
      const g = b.fields[fname];
      if (!f) { finding('authority-parity', `field ${name}.${fname}: present in the JSON Schema, absent from the TypeSpec`, label); continue; }
      if (!g) {
        // @statusCode / @query / @path / @header are HTTP bindings, not body members;
        // a JSON Schema for the body correctly has no peer for them.
        if (f.httpBinding) continue;
        finding('authority-parity', `field ${name}.${fname}: present in the TypeSpec, absent from the JSON Schema`, label);
        continue;
      }
      const fe = f.enum === null || f.enum === undefined ? null : [...f.enum];
      const ge = g.enum === null || g.enum === undefined ? null : [...g.enum];
      if (JSON.stringify(fe) !== JSON.stringify(ge)) {
        finding('authority-parity', `field ${name}.${fname}.enum: typespec=${show(fe)} json-schema=${show(ge)}`, label);
      }
      if (f.type === 'ref' || g.type === 'ref') {
        if (f.ref !== g.ref) finding('authority-parity', `field ${name}.${fname}: refers to ${show(f.ref)} in the TypeSpec and ${show(g.ref)} in the JSON Schema`, label);
        if (f.optional !== g.optional) finding('authority-parity', `field ${name}.${fname}.optional: typespec=${f.optional} json-schema=${g.optional}`, label);
        if (f.array !== g.array) finding('authority-parity', `field ${name}.${fname}.array: typespec=${f.array} json-schema=${g.array}`, label);
        continue;
      }
      for (const k of COMPARED) {
        if (k === 'format' && fe) continue;
        if (JSON.stringify(f[k] ?? null) !== JSON.stringify(g[k] ?? null)) {
          finding('authority-parity', `field ${name}.${fname}.${k}: typespec=${show(f[k])} json-schema=${show(g[k])}`, label);
        }
      }
    }
  }

  // A declaration with no peer is acceptable only if a human wrote down why.
  for (const [side, entries] of [['typespec', orphans.typespec], ['jsonSchema', orphans.jsonSchema]]) {
    const excused = only[side] ?? {};
    for (const entry of entries) {
      const key = entry.split(' ')[1];
      if (typeof excused[key] === 'string' && excused[key].length > 0) continue;
      finding(
        'authority-parity',
        `${entry}: declared only in the ${side} authority, and contracts/parity.config.json gives no reason. `
        + `Author the peer, or record "${key}": "<why this cannot have a peer>" under only.${side}`,
        label,
      );
    }
    for (const key of Object.keys(excused)) {
      if (!entries.some((e) => e.split(' ')[1] === key)) {
        finding(
          'authority-parity',
          `contracts/parity.config.json excuses "${key}" as ${side}-only, but it is not ${side}-only any more. Delete the excuse`,
          label,
        );
      }
    }
  }
}

// ===========================================================================
// 4. Protobuf: the runtime-validated third description.
// ===========================================================================
function parseProto(text) {
  const { text: src } = scan(text);
  const messages = {};
  const enums = {};
  const re = /\b(message|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    const open = m.index + m[0].length - 1;
    let depth = 0;
    let i = open;
    for (; i < src.length; i += 1) {
      if (src[i] === '{') depth += 1;
      else if (src[i] === '}') { depth -= 1; if (depth === 0) break; }
    }
    const body = src.slice(open + 1, i);
    if (m[1] === 'enum') {
      const values = {};
      for (const v of body.matchAll(/([A-Z][A-Z0-9_]*)\s*=\s*(\d+)\s*;/g)) values[v[1]] = Number(v[2]);
      enums[m[2]] = values;
    } else {
      const fields = [];
      const reservedNumbers = new Set();
      const reservedNames = new Set();
      for (const r of body.matchAll(/reserved\s+([^;]+);/g)) {
        for (const tok of r[1].split(',')) {
          const t = tok.trim();
          const range = t.match(/^(\d+)\s+to\s+(\d+)$/);
          if (range) { for (let n = Number(range[1]); n <= Number(range[2]); n += 1) reservedNumbers.add(n); }
          else if (/^\d+$/.test(t)) reservedNumbers.add(Number(t));
          else if (/^"[^"]*"$/.test(t)) reservedNames.add(t.slice(1, -1));
        }
      }
      const oneofs = {};
      for (const o of body.matchAll(/\boneof\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{([^}]*)\}/g)) {
        oneofs[o[1]] = [];
        for (const f of o[2].matchAll(/([A-Za-z_][A-Za-z0-9_.]*)\s+([a-z_][a-z0-9_]*)\s*=\s*(\d+)\s*;/g)) {
          oneofs[o[1]].push({ type: f[1], name: f[2], number: Number(f[3]) });
          fields.push({ type: f[1], name: f[2], number: Number(f[3]), optional: false, repeated: false, oneof: o[1] });
        }
      }
      const flat = body.replace(/\boneof\s+[A-Za-z_][A-Za-z0-9_]*\s*\{[^}]*\}/g, '');
      for (const f of flat.matchAll(/(?:(optional|repeated|required)\s+)?([A-Za-z_][A-Za-z0-9_.]*)\s+([a-z_][a-z0-9_]*)\s*=\s*(\d+)\s*;/g)) {
        fields.push({ type: f[2], name: f[3], number: Number(f[4]), optional: f[1] === 'optional', repeated: f[1] === 'repeated', oneof: null });
      }
      messages[m[2]] = { fields, reservedNumbers, reservedNames, oneofs };
    }
    re.lastIndex = i;
  }
  return { messages, enums };
}

const PROTO_JSON_TYPE = {
  string: ['string', null], bytes: ['string', 'byte'], bool: ['boolean', null],
  int32: ['integer', null], uint32: ['integer', null], sint32: ['integer', null], fixed32: ['integer', null], sfixed32: ['integer', null],
  int64: ['integer', null], uint64: ['integer', null], sint64: ['integer', null], fixed64: ['integer', null], sfixed64: ['integer', null],
  double: ['number', null], float: ['number', null],
};

/** protobuf's own lowerCamelCase JSON mapping (proto3 JSON, canonical form). */
const jsonName = (snake) => snake.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());

/** SCREAMING_SNAKE proto enum value -> the contract's snake_case string. */
function contractValues(enumName, values) {
  const prefix = `${enumName.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase()}_`;
  return Object.keys(values)
    .filter((v) => !v.endsWith('_UNSPECIFIED'))
    .map((v) => (v.startsWith(prefix) ? v.slice(prefix.length) : v).toLowerCase());
}

function checkProto(tspShape, jsShape) {
  const AUTHORITIES = [['typespec', tspShape], ['json-schema', jsShape]];

  for (const [file, spec] of Object.entries(CONFIG.proto.files)) {
    const path = join(ROOT, file);
    if (!existsSync(path)) { finding('proto-parity', `${file} is named in parity.config.json but does not exist`, file); continue; }
    const proto = parseProto(readFileSync(path, 'utf8'));

    // --- field-number discipline -----------------------------------------
    for (const [msg, def] of Object.entries(proto.messages)) {
      const seen = new Map();
      for (const f of def.fields) {
        if (f.number < 1 || f.number > 536870911) finding('proto-discipline', `${msg}.${f.name} = ${f.number} is outside the legal field-number range`, file);
        if (f.number >= 19000 && f.number <= 19999) finding('proto-discipline', `${msg}.${f.name} = ${f.number} is inside protobuf's reserved 19000-19999 range`, file);
        if (seen.has(f.number)) finding('proto-discipline', `${msg}: field number ${f.number} is used by both \`${seen.get(f.number)}\` and \`${f.name}\`. A number is never reused`, file);
        seen.set(f.number, f.name);
        if (def.reservedNumbers.has(f.number)) finding('proto-discipline', `${msg}.${f.name} takes field number ${f.number}, which is \`reserved\`. A reserved number is burned for the life of the message`, file);
        if (def.reservedNames.has(f.name)) finding('proto-discipline', `${msg}.${f.name} takes a \`reserved\` name. A removed name is burned so a stale peer cannot resurrect it`, file);
      }
    }

    // --- agreement with BOTH authorities ----------------------------------
    for (const [msg, binding] of Object.entries(spec.messages ?? {})) {
      const model = typeof binding === 'string' ? binding : binding.model;
      const omits = typeof binding === 'string' ? {} : (binding.omits ?? {});
      const def = proto.messages[msg];
      if (!def) { finding('proto-parity', `${file} declares no message ${msg}, but parity.config.json binds it to ${model}`, file); continue; }

      for (const [authority, shape] of AUTHORITIES) {
        const target = shape.models[model];
        if (!target) { finding('proto-parity', `${msg} is bound to ${model}, which the ${authority} authority does not declare`, file); continue; }
        const covered = new Set();
        for (const f of def.fields) {
          if (f.oneof) continue; // union tags; checked against the contract union below
          const name = jsonName(f.name);
          covered.add(name);
          const field = target.fields[name];
          if (!field) {
            finding('proto-parity', `${msg}.${f.name} has JSON name \`${name}\`, which is not a field of ${model} in the ${authority} authority. A proto field no authority describes is a wire type nothing validates`, file);
            continue;
          }
          const mapped = PROTO_JSON_TYPE[f.type];
          if (mapped) {
            if (field.type !== mapped[0]) {
              finding('proto-parity', `${msg}.${f.name}: proto \`${f.type}\` is JSON ${mapped[0]}, but ${model}.${name} is ${field.type} in the ${authority} authority`, file);
            }
            if (mapped[1] && field.format && field.format !== mapped[1]) {
              finding('proto-parity', `${msg}.${f.name}: proto \`${f.type}\` is JSON format ${mapped[1]}, but ${model}.${name} is format ${field.format} in the ${authority} authority`, file);
            }
          } else if (proto.enums[f.type]) {
            const asContract = contractValues(f.type, proto.enums[f.type]);
            if (!field.enum) {
              finding('proto-parity', `${msg}.${f.name} is proto enum ${f.type}, but ${model}.${name} is not an enum in the ${authority} authority`, file);
            } else if (JSON.stringify([...asContract].sort()) !== JSON.stringify([...field.enum].sort())) {
              finding('proto-parity', `${msg}.${f.name}: proto enum ${f.type} maps to ${JSON.stringify(asContract)}, but ${model}.${name} allows ${JSON.stringify(field.enum)} in the ${authority} authority`, file);
            }
          } else {
            finding('proto-parity', `${msg}.${f.name} has type \`${f.type}\`, which is neither a scalar nor an enum declared in ${file}`, file);
          }
          if (f.repeated !== !!field.array) {
            finding('proto-parity', `${msg}.${f.name}: proto ${f.repeated ? 'is' : 'is not'} \`repeated\`, but ${model}.${name} ${field.array ? 'is' : 'is not'} an array in the ${authority} authority`, file);
          }
          if (f.optional !== !!field.optional) {
            finding('proto-parity', `${msg}.${f.name}: proto ${f.optional ? 'is' : 'is not'} \`optional\`, but ${model}.${name} ${field.optional ? 'is' : 'is not'} optional in the ${authority} authority`, file);
          }
        }
        for (const name of Object.keys(target.fields)) {
          if (covered.has(name) || target.fields[name].httpBinding) continue;
          if (typeof omits[name] === 'string' && omits[name].length > 0) continue;
          finding('proto-parity', `${model}.${name} is declared by the ${authority} authority but message ${msg} has no field for it, and parity.config.json gives no reason. Add the field, or record "${name}": "<why the binary encoding carries this differently>" under omits`, file);
        }
      }
      for (const k of Object.keys(omits)) {
        if (!jsShape.models[model]?.fields[k] && !tspShape.models[model]?.fields[k]) {
          finding('proto-parity', `parity.config.json excuses ${msg} for omitting \`${k}\`, but no authority declares ${model}.${k}. Delete the excuse`, file);
        }
      }
    }

    // --- oneof tags vs the contract union ---------------------------------
    for (const [msg, unionName] of Object.entries(spec.unions ?? {})) {
      const def = proto.messages[msg];
      if (!def) { finding('proto-parity', `${file} declares no message ${msg} for union ${unionName}`, file); continue; }
      const members = Object.values(def.oneofs).flat().map((f) => f.type);
      for (const [authority, shape] of AUTHORITIES) {
        const union = shape.unions[unionName];
        if (!union) { finding('proto-parity', `${msg} is bound to union ${unionName}, which the ${authority} authority does not declare`, file); continue; }
        if (JSON.stringify([...members].sort()) !== JSON.stringify([...union].sort())) {
          finding('proto-parity', `${msg} oneof carries ${JSON.stringify([...members].sort())}, but union ${unionName} is ${JSON.stringify([...union].sort())} in the ${authority} authority`, file);
        }
      }
    }

    // --- standalone enums --------------------------------------------------
    for (const [protoEnum, contractEnum] of Object.entries(spec.enums ?? {})) {
      const values = proto.enums[protoEnum];
      if (!values) { finding('proto-parity', `${file} declares no enum ${protoEnum}`, file); continue; }
      const zero = Object.keys(values).filter((n) => values[n] === 0);
      if (zero.length !== 1 || !zero[0].endsWith('_UNSPECIFIED')) {
        finding('proto-discipline', `enum ${protoEnum} must have exactly one zero value and it must end in _UNSPECIFIED (proto3 gives enums no presence)`, file);
      }
      const asContract = contractValues(protoEnum, values);
      for (const [authority, shape] of AUTHORITIES) {
        const contract = shape.enums[contractEnum];
        if (!contract) { finding('proto-parity', `enum ${protoEnum} is bound to ${contractEnum}, which the ${authority} authority does not declare`, file); continue; }
        const missing = contract.filter((v) => !asContract.includes(v));
        const extra = asContract.filter((v) => !contract.includes(v));
        if (missing.length) finding('proto-parity', `enum ${protoEnum} has no value for ${JSON.stringify(missing)} of ${contractEnum} (${authority}). A contract value the runtime cannot encode is unreachable`, file);
        if (extra.length) finding('proto-parity', `enum ${protoEnum} declares ${JSON.stringify(extra)}, which ${contractEnum} does not allow (${authority}). The runtime would accept a value no authority describes`, file);
      }
    }
  }
}

// ===========================================================================
// 5. External tools
// ===========================================================================
function run(cmd, args) {
  return execFileSync(cmd, args, { cwd: ROOT, stdio: 'pipe', encoding: 'utf8' });
}
function toolMissing(e) {
  const t = `${e.stderr ?? ''}${e.stdout ?? ''}${e.message ?? ''}`;
  return /ENOENT|not found|could not determine executable|EAI_AGAIN|ECONNREFUSED|getaddrinfo|ENOTFOUND|npm error|npm ERR!/i.test(t);
}
function tail(e, n = 6) {
  return `${e.stderr ?? ''}\n${e.stdout ?? ''}`.trim().split('\n').filter(Boolean).slice(-n)
    .join(' | ');
}

/**
 * A lane needing a tool this machine may not have. A MISSING TOOL IS A FINDING:
 * a gate that quietly passes because it could not run is worse than no gate.
 * `--offline` downgrades it to a loud SKIPPED banner and is never passed in CI.
 */
function toolLane(name, fn) {
  try {
    lane(name, 'passed', fn());
  } catch (e) {
    if (e.__reported) { lane(name, 'failed', tail(e)); return; }
    if (toolMissing(e)) {
      if (OFFLINE) {
        lane(name, 'skipped', 'tool unavailable and --offline was passed');
        log(`[parity] ${name}: SKIPPED (tool unavailable; --offline)`);
        return;
      }
      lane(name, 'failed', 'tool unavailable');
      finding('tool-unavailable', `${name}: the tool this lane needs is not installed, so the lane did not run and this gate cannot claim the authorities agree. Install it, or pass --offline and read the SKIPPED banner`, name);
      return;
    }
    lane(name, 'failed', tail(e));
    finding(name === 'ores-contracts' ? 'ores-contracts' : 'typespec-compile', tail(e), name);
  }
}

// ===========================================================================
// main
// ===========================================================================
log('[parity] two independent authorities; a discrepancy is a finding, never a merge');
rmSync(TARGET, { recursive: true, force: true });
mkdirSync(TARGET, { recursive: true });

const tspShape = parseTypeSpecShape(CONFIG.typespec.sources);
for (const u of tspShape.unparsed) {
  finding('typespec-unreadable', `${u} - the parity reader refuses constructs it cannot compare rather than skipping them`, 'shape-source');
}
const documents = Object.fromEntries(
  Object.entries(CONFIG.jsonSchema.documents).map(([k, p]) => [k, JSON.parse(readFileSync(join(ROOT, p), 'utf8'))]),
);
const jsShape = jsonSchemaShape(documents);
writeFileSync(join(TARGET, 'shape-typespec.json'), `${JSON.stringify(tspShape, null, 2)}\n`);
writeFileSync(join(TARGET, 'shape-json-schema.json'), `${JSON.stringify(jsShape, null, 2)}\n`);
log(`[parity] typespec source: ${Object.keys(tspShape.models).length} models, ${Object.keys(tspShape.enums).length} enums, ${Object.keys(tspShape.unions).length} union(s)`);
log(`[parity] json-schema:     ${Object.keys(jsShape.models).length} models, ${Object.keys(jsShape.enums).length} enums, ${Object.keys(jsShape.unions).length} union(s)`);

// -- lane: shape-source ------------------------------------------------------
let mark = findings.length;
diffShapes(tspShape, jsShape, 'shape-source', CONFIG.only ?? {});
for (const [name, spec] of Object.entries(CONFIG.constants ?? {})) {
  const a = tspShape.constants[name];
  const b = documents[spec.document]?.[spec.keyword];
  if (a === undefined) finding('authority-parity', `constant ${name} is not declared in the TypeSpec authority`, 'shape-source');
  else if (b === undefined) finding('authority-parity', `the ${spec.document} schema declares no ${spec.keyword}, the peer of TypeSpec's ${name}`, 'shape-source');
  else if (a !== b) finding('authority-parity', `constant ${name}: typespec=${a} json-schema(${spec.keyword})=${b}`, 'shape-source');
}
lane('shape-source', findings.length === mark ? 'passed' : 'failed');
log(`[parity] shape-source: ${findings.length === mark ? 'ok' : `${findings.length - mark} discrepancies`}`);

// -- lane: published ---------------------------------------------------------
// A standalone document this repository publishes at a stable $id (schema/v1/...)
// and its $defs peer are the SAME type. If they drift, consumers fetching the
// published URL and code built from the contracts believe different things --
// which is a two-truths failure of exactly the kind this repo exists to prevent.
const SHAPE_KEYS = ['type', 'format', 'const', 'enum', 'minLength', 'maxLength', 'minimum', 'maximum', 'pattern', 'maxItems', 'minItems'];
function checkPublished() {
  for (const [file, spec] of Object.entries(CONFIG.publishedSchemas ?? {})) {
    if (file.startsWith('$')) continue;
    const path = join(ROOT, file);
    if (!existsSync(path)) { finding('published-schema', `${file} is named in parity.config.json but does not exist`, 'published'); continue; }
    const published = JSON.parse(readFileSync(path, 'utf8'));
    const peer = documents[spec.document]?.$defs?.[spec.model];
    if (!peer) { finding('published-schema', `${file} is bound to ${spec.document}#/$defs/${spec.model}, which does not exist`, 'published'); continue; }
    if (published.additionalProperties !== peer.additionalProperties) {
      finding('published-schema', `${file}: additionalProperties=${show(published.additionalProperties)} but the ${spec.model} peer has ${show(peer.additionalProperties)}`, 'published');
    }
    const a = [...(published.required ?? [])].sort();
    const b = [...(peer.required ?? [])].sort();
    if (JSON.stringify(a) !== JSON.stringify(b)) {
      finding('published-schema', `${file}: required=${JSON.stringify(a)} but the ${spec.model} peer requires ${JSON.stringify(b)}`, 'published');
    }
    const pp = published.properties ?? {};
    const qq = peer.properties ?? {};
    for (const name of new Set([...Object.keys(pp), ...Object.keys(qq)])) {
      if (!pp[name]) { finding('published-schema', `${file}: the ${spec.model} peer declares \`${name}\`, which the published document does not. Publishing a v1 change means a new $id, not an edit`, 'published'); continue; }
      if (!qq[name]) { finding('published-schema', `${file} declares \`${name}\`, which the ${spec.model} peer does not`, 'published'); continue; }
      for (const k of SHAPE_KEYS) {
        if (JSON.stringify(pp[name][k] ?? null) !== JSON.stringify(qq[name][k] ?? null)) {
          finding('published-schema', `${file}: ${name}.${k}=${show(pp[name][k])} but the ${spec.model} peer has ${show(qq[name][k])}`, 'published');
        }
      }
    }
  }
}
mark = findings.length;
checkPublished();
lane('published', findings.length === mark ? 'passed' : 'failed');
log(`[parity] published: ${findings.length === mark ? 'ok' : `${findings.length - mark} discrepancies`}`);

// -- lane: proto -------------------------------------------------------------
mark = findings.length;
checkProto(tspShape, jsShape);
lane('proto', findings.length === mark ? 'passed' : 'failed');
log(`[parity] proto: ${findings.length === mark ? 'ok' : `${findings.length - mark} discrepancies`}`);

// -- fixture lanes -----------------------------------------------------------
function fixtureLane(name, extraArgs, laneLabel) {
  let out;
  try {
    out = run('python3', ['scripts/check_fixtures.py', '--json', '--lane', laneLabel, ...extraArgs]);
  } catch (e) {
    out = e.stdout;
    if (!out) {
      lane(name, 'failed', tail(e));
      finding('fixtures', `${laneLabel}: check_fixtures.py did not run - ${tail(e)}`, name);
      return null;
    }
  }
  const report = JSON.parse(out);
  for (const f of report.findings) finding(f.kind, `${laneLabel}: ${f.detail}`, name);
  const s = report.stats;
  lane(name, report.findings.length ? 'failed' : 'passed',
    `${s.positive_passed}/${s.positive_passed + s.positive_failed} positives accepted, ${s.negative_rejected}/${s.negative_rejected + s.negative_accepted} negatives rejected`);
  log(`[parity] ${name}: ${lanes[name].note}`);
  return report;
}
const jsFixtures = fixtureLane('fixtures-js', [], 'hand-authored json-schema');

// -- lanes that need the TypeSpec toolchain ---------------------------------
let emittedPath = null;
toolLane('tsp-compile', () => {
  run('npx', ['--no-install', 'tsp', 'compile', CONFIG.typespec.entry, '--no-emit', '--warn-as-error']);
  log('[parity] tsp-compile: ok');
  return 'tsp compile --warn-as-error';
});
toolLane('tsp-emit', () => {
  const out = join(TARGET, 'typespec-emitted');
  run('npx', ['--no-install', 'tsp', 'compile', CONFIG.typespec.entry, '--warn-as-error',
    '--emit', '@typespec/json-schema',
    '--option', `@typespec/json-schema.emitter-output-dir=${out}`,
    '--option', '@typespec/json-schema.file-type=json',
    '--option', `@typespec/json-schema.bundleId=${CONFIG.typespec.bundleId}`]);
  const bundle = join(out, CONFIG.typespec.bundleId);
  if (!existsSync(bundle)) throw new Error(`tsp emitted no ${rel(bundle)}`);
  emittedPath = bundle;
  log(`[parity] tsp-emit: ${rel(bundle)}`);
  return rel(bundle);
});

if (emittedPath) {
  mark = findings.length;
  const emitted = JSON.parse(readFileSync(emittedPath, 'utf8'));
  const emittedShape = jsonSchemaShape({ emitted });
  // TypeSpec models are closed by construction, so the emitted document not
  // spelling `additionalProperties: false` is a property of the emitter, not a
  // disagreement about the type. The hand-authored side is still required to
  // seal every model -- that check lives in diffShapes and is not relaxed here.
  for (const model of Object.values(emittedShape.models)) model.sealed = true;
  diffShapes(
    { enums: emittedShape.enums, models: emittedShape.models, unions: emittedShape.unions },
    jsShape, 'shape-emitted', CONFIG.only ?? {},
  );
  lane('shape-emitted', findings.length === mark ? 'passed' : 'failed');
  log(`[parity] shape-emitted: ${findings.length === mark ? 'ok' : `${findings.length - mark} discrepancies`}`);

  const map = join(TARGET, 'emitted-documents.json');
  writeFileSync(map, `${JSON.stringify({ '*': rel(emittedPath) }, null, 2)}\n`);
  const tspFixtures = fixtureLane('fixtures-tsp', ['--documents', rel(map)], 'typespec-emitted schema');
  if (jsFixtures && tspFixtures) {
    for (const [model, byPath] of Object.entries(jsFixtures.results)) {
      for (const [path, verdict] of Object.entries(byPath)) {
        const other = tspFixtures.results?.[model]?.[path];
        if (other && other !== verdict) {
          finding('fixture-disagreement',
            `${path}: the hand-authored JSON Schema ${verdict} it and the schema TypeSpec emitted ${other} it. `
            + 'The two authorities describe different types here; a human decides which one is wrong',
            'fixtures');
        }
      }
    }
  }
} else if (OFFLINE) {
  lane('shape-emitted', 'skipped', 'tsp unavailable; shape-source made the same comparison from the .tsp source');
  lane('fixtures-tsp', 'skipped', 'tsp unavailable; fixtures-js still ran against the hand-authored authority');
} else {
  lane('shape-emitted', 'failed', 'no emitted schema');
  lane('fixtures-tsp', 'failed', 'no emitted schema');
}

// -- lane: ores-contracts ----------------------------------------------------
toolLane('ores-contracts', () => {
  const out = run('npx', ['--no-install', 'ores-contracts', 'check', '--config', 'contracts.config.json']);
  for (const line of out.split('\n')) if (line.trim()) log(`[parity]   ${line.trim()}`);
  if (/STOPPED_FOR_EVALUATION/.test(out)) {
    for (const line of out.split('\n')) {
      const m = line.match(/[^\s]\s+([a-z-]+):\s*(.*)$/);
      if (m && /^(authority-parity|artifact-parity|typespec-compile|missing-authority)$/.test(m[1])) {
        finding(`ores-${m[1]}`, m[2], 'ores-contracts');
      }
    }
    throw Object.assign(new Error('ores-contracts stopped for evaluation'), { stdout: out, __reported: true });
  }
  return 'persisted subset agrees; both lanes emit byte-identical SQL / SeaORM / Diesel / Rust / TS / Dart';
});

// ---------------------------------------------------------------------------
const receipt = {
  tool: 'gha-indie-worker-interfaces/scripts/parity.mjs',
  checkedAt: new Date().toISOString(),
  offline: OFFLINE,
  authorities: {
    typespec: {
      entry: CONFIG.typespec.entry,
      sources: CONFIG.typespec.sources,
      sha256: sha(CONFIG.typespec.sources.map((f) => readFileSync(join(ROOT, f), 'utf8')).join(' ')),
    },
    'json-schema': {
      documents: CONFIG.jsonSchema.documents,
      sha256: sha(Object.values(CONFIG.jsonSchema.documents).map((f) => readFileSync(join(ROOT, f), 'utf8')).join(' ')),
    },
  },
  lanes,
  findings,
  status: findings.length ? 'stopped_for_evaluation' : 'passed',
};
writeFileSync(join(TARGET, 'receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`);

if (AS_JSON) {
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
} else {
  const skipped = Object.entries(lanes).filter(([, l]) => l.status === 'skipped');
  if (skipped.length) {
    log('');
    log(`[parity] ${skipped.length} LANE(S) DID NOT RUN - this gate did not check everything it can check:`);
    for (const [name, l] of skipped) log(`[parity]   ! ${name}: ${l.note}`);
  }
  log('');
  if (findings.length) {
    log(`[parity] ${findings.length} DISCREPANCIES - the two authorities do not agree. Nothing was reconciled.`);
    for (const f of findings) log(`[parity]   x [${f.fingerprint}] ${f.kind}${f.where ? ` (${f.where})` : ''}: ${f.detail}`);
    log('');
    log('[parity] Fix by editing an AUTHORED source - contracts/typespec/*.tsp or');
    log('[parity] contracts/json-schema/*.json - never by editing target/, which is disposable.');
  } else {
    log('[parity] PASSED - both authorities describe the same vocabulary.');
  }
  log(`[parity] receipt: ${rel(join(TARGET, 'receipt.json'))}`);
}

process.exit(findings.length ? 1 : 0);
