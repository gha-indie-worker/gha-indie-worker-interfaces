// Runtime for the Ores.* persistence decorators. Vendored verbatim from
// @oresoftware/ores-contracts so `tsp compile` resolves them without a node_modules
// lookup from inside a .tsp file. The decorators only record metadata; ores-contracts'
// own parser and scripts/parity.mjs both read them structurally from the source text.
export function $table() {}
export function $unique() {}
export function $index() {}
export function $references() {}
export const $decorators = { Ores: { table: $table, unique: $unique, index: $index, references: $references } };
