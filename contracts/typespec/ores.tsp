// ores-contracts persistence decorators. Vendored from @oresoftware/ores-contracts
// (`typespec/ores.tsp`) so this repo's TypeSpec authority compiles standalone.
// Declared `extern` and implemented by ./ores-decorators.js.
import "./ores-decorators.js";

namespace Ores;

/** Table name for the model. */
extern dec table(target: Model, name: valueof string);
/** Comma-separated field names forming a UNIQUE constraint. Repeatable. */
extern dec unique(target: Model, fields: valueof string);
/** Comma-separated field names forming an index. Repeatable. */
extern dec index(target: Model, fields: valueof string);
/** Foreign key: "Model.field". */
extern dec references(target: ModelProperty, to: valueof string);
