// Runtime for the Ores.* persistence decorators.
//
// The decorators only record intent so that `tsp compile` accepts the TypeSpec
// authority. ores-contracts' own parser reads them structurally out of the
// source text; nothing here is executed by the parity gate.
//
// Vendored from ORESoftware/ores-contracts (lib/ores-decorators.js). Keep byte
// compatible with the toolkit so `npx ores-contracts check` and `tsp compile`
// can never disagree about what a decorator means.
export function $table() {}
export function $unique() {}
export function $index() {}
export function $references() {}
export const $decorators = { Ores: { table: $table, unique: $unique, index: $index, references: $references } };
