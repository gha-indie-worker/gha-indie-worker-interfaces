// ores-contracts persistence decorators. Vendored from @oresoftware/ores-contracts
// (`typespec/ores.tsp`) so this repo's TypeSpec authority compiles standalone.
// Declared `extern` and implemented by ./ores-decorators.js.
import "./ores-decorators.js";

namespace Ores;

// `Reflection.Model`, not a bare `Model`: the bare identifier does not resolve, so the decorator's
// target becomes ErrorType and every application is rejected with `decorator-wrong-target`.
// Upstream ORESoftware/ores-contracts declares these the same way and needs the same fix.

/** Table name for the model. */
extern dec table(target: Reflection.Model, name: valueof string);
/** Comma-separated field names forming a UNIQUE constraint. Repeatable. */
extern dec unique(target: Reflection.Model, fields: valueof string);
/** Comma-separated field names forming an index. Repeatable. */
extern dec index(target: Reflection.Model, fields: valueof string);
/** Foreign key: "Model.field". */
extern dec references(target: Reflection.ModelProperty, to: valueof string);
