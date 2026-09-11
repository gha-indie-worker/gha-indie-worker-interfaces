# contracts/proto/

Protobuf v3 for the wire types that are **validated at runtime**: session frames
on the binary transport, and the run/job lifecycle events carried in
`EventFrame.payload`.

## This is not a third authority

TypeSpec and JSON Schema say what a frame **is**. These files say how it is
**checked on the wire**. `scripts/parity.mjs` verifies the proto agrees with
**both** authorities and never lets it be the tiebreak:

- every field's canonical proto3 JSON name (`skipped_to` → `skippedTo`) must be a
  field of the bound model in **both** authorities;
- its proto type must map to the same JSON type and format both authorities give
  it (`bytes` → `string`/`byte`, `uint64` → `integer`, and so on);
- `repeated` must match array-ness and `optional` must match optionality, in both;
- every proto enum value maps back to the contract's `snake_case` string, and the
  value sets must be equal in both directions — a contract value the runtime
  cannot encode is unreachable, and a proto value no authority allows is a value
  the runtime would accept that nothing describes;
- a field of the model with no field in the message needs a **written reason** in
  `contracts/parity.config.json`, and the gate fails if that reason goes stale.

Runtime validation is what protobuf adds that neither schema does: a length-prefixed
frame is rejected by the decoder before any handler sees it, so a malformed peer
is a closed connection rather than a half-applied state change.

## Field-number discipline

A protobuf field number is a permanent, load-bearing identifier. It is not a
serial number and it is not free to change. These rules are enforced
mechanically by the `proto` lane of `scripts/parity.mjs`.

**1. A field number, once assigned, is never reused for a different field.**
Not after the field is deprecated, not after it is deleted, not ever. An old peer
decoding a new message reads by number, so a reused number means the old peer
silently parses the new field into the old field's slot — wrong data with no
error anywhere. This is the rule the whole file exists to protect.

**2. Removing a field means `reserved` on BOTH the number and the name.**

```proto
reserved 4;
reserved "observed_at";
```

The number stops anyone re-issuing it; the name stops anyone re-declaring the
field and quietly getting a *new* number, which breaks the other direction. See
`EventFrame` in `session.proto` for a real removal and why it happened.

**3. Numbers 1–15 encode their tag in one byte; 16–2047 take two.**
They are spent deliberately on the fields present in every message of a hot type,
not handed out first-come. `EventFrame`'s `stream`, `sequence` and `payload` hold
1–3 for that reason.

**4. Changing a field's type in place is a removal plus an addition.**
`reserved` the old number, add a new field with a new number. Editing the type on
the existing number produces a message that decodes to garbage on an old peer,
which is the same failure as rule 1 with a friendlier-looking diff.

**5. 19000–19999 are reserved by protobuf itself.** Never use them.

**6. Every proto3 enum has exactly one zero value and it ends in `_UNSPECIFIED`.**
proto3 gives enums no field presence, so the zero value is what an absent or
unknown value decodes to. Naming it `_UNSPECIFIED` makes "not set" distinguishable
from a real state; making a real state the zero value makes an absent field look
like that state.

## Numbers as wire tags

Two number assignments are deliberately not arbitrary, and both are stated in the
files themselves:

- `ErrorCode`'s proto numbers are the byte tags of the TCP encoding
  (`runtime::session::ErrorCode::tag`), so the proto number and the wire tag
  cannot drift apart.
- `Frame`'s `oneof kind` tag numbers are the frame tags of the TCP encoding, so a
  binary frame's first tag byte and its proto field number agree.

Changing either means changing `runtime::session` in `gha-indie-worker-lib-core`
in the same PR.

## The `kind` discriminator

The JSON encoding discriminates frames by a `kind` string. The binary encoding
discriminates by the `oneof` tag number. So the proto messages have no `kind`
field, and `parity.config.json` records that reason per message — carrying the
discriminator twice on the binary transport would let the tag and the field
disagree, and there would be no rule saying which one wins.
