import "./ores-decorators.js";

namespace Ores;

// `Model` and `ModelProperty` below are reflection types; without this the extern
// declarations do not resolve and every @Ores.* use reports decorator-wrong-target.
using TypeSpec.Reflection;

/** Table name for the model. */
extern dec table(target: Model, name: valueof string);
/** Comma-separated field names forming a UNIQUE constraint. Repeatable. */
extern dec unique(target: Model, fields: valueof string);
/** Comma-separated field names forming an index. Repeatable. */
extern dec index(target: Model, fields: valueof string);
/** Foreign key: "Model.field". */
extern dec references(target: ModelProperty, to: valueof string);

/** Wire-level UUID. Emitted as SQL UUID, Rust String, TypeScript string. */
@format("uuid")
scalar uuid extends string;

/** Opaque JSON document. Emitted as JSONB. */
alias json = Record<unknown>;
