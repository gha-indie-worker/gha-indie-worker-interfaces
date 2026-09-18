// Runtime for the Ores.* persistence decorators.
//
// ores-contracts remains the persistence/code-generation parser and reads these
// decorators structurally from TypeSpec source. At TypeSpec compile time the
// same decorator invocations also project their persistence metadata into the
// official @typespec/json-schema witness. That gives TJSV a faithful generated
// witness without making generated JSON Schema authoritative or rewriting the
// independently authored JSON Schema peer.
import { isKey } from "@typespec/compiler";
import { setExtension } from "@typespec/json-schema";

const uniqueByModel = new WeakMap();
const indexesByModel = new WeakMap();

function fieldsList(fields) {
  return String(fields)
    .split(",")
    .map((field) => field.trim())
    .filter(Boolean);
}

function repeatedExtension(program, target, key, state, fields) {
  let entries = state.get(target);
  if (!entries) {
    entries = [];
    state.set(target, entries);
    setExtension(program, target, key, entries);
  }
  entries.push(fieldsList(fields));
}

export function $table(context, target, name) {
  setExtension(context.program, target, "x-ores-table", name);

  // Built-in @key remains the authored TypeSpec primary-key declaration that
  // ores-contracts parses. Mirror that already-authored intent into the JSON
  // Schema witness instead of introducing a second ORES primary-key decorator.
  const primaryKey = [...target.properties.values()]
    .filter((property) => isKey(context.program, property))
    .map((property) => property.name);
  if (primaryKey.length) {
    setExtension(context.program, target, "x-ores-primary-key", primaryKey);
  }
}

export function $unique(context, target, fields) {
  repeatedExtension(context.program, target, "x-ores-unique", uniqueByModel, fields);
}

export function $index(context, target, fields) {
  repeatedExtension(context.program, target, "x-ores-indexes", indexesByModel, fields);
}

export function $references(context, target, to) {
  setExtension(context.program, target, "x-ores-references", to);
}

export const $decorators = {
  Ores: {
    table: $table,
    unique: $unique,
    index: $index,
    references: $references,
  },
};
