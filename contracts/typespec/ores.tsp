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

/** Wire-level UUID. Emitted as SQL UUID, Rust String, TypeScript string. */
@format("uuid")
scalar uuid extends string;

/** Opaque JSON document. Emitted as JSONB. */
alias json = Record<unknown>;
