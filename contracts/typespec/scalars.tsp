// ============================================================================
// TypeSpec authority — shared scalars.
//
// TypeSpec has no built-in `uuid`, so the identifier has to be declared before
// it can be used. The JSON Schema peer describes the same thing as
// `{"type": "string", "format": "uuid"}`, and this declaration is written to
// match it exactly: same base type, same format. If one side ever changes,
// the parity gate reports it rather than reconciling it.
// ============================================================================

namespace GhaIndieWorker;

@doc("An RFC 4122 UUID, rendered as its canonical lowercase hyphenated string. Peer: json-schema `{type: string, format: uuid}`.")
@format("uuid")
scalar uuid extends string;
