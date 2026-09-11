#![forbid(unsafe_code)]

//! Serde mirror of the `sync` slice (opto-sync causal envelopes).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncOp {
    Upsert,
    Delete,
    Merge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictPolicy {
    LastWriteWins,
    CausalMerge,
    Reject,
}

/// One opto-sync causal envelope.
///
/// `vectorClock` is `json` in both authorities because the contract subset has no
/// map type; its runtime shape is `{ nodeId: nonNegativeInteger }`, which the JSON
/// Schema authority pins with `additionalProperties`. [`CausalEnvelope::clock`]
/// is the typed read of that field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CausalEnvelope {
    pub id: String,
    pub entity: String,
    pub entity_id: String,
    pub op: SyncOp,
    pub vector_clock: Value,
    pub payload: Value,
    pub conflict_policy: ConflictPolicy,
    pub origin_node: String,
    pub origin_sequence: i64,
    pub recorded_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,
}

/// How two envelopes relate in the causal order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CausalOrder {
    /// `self` happened before `other`.
    Before,
    /// `other` happened before `self`.
    After,
    /// The two clocks are identical.
    Equal,
    /// Neither dominates: a genuine conflict for `conflictPolicy` to settle.
    Concurrent,
}

impl CausalEnvelope {
    /// The vector clock as a typed map, or `None` when the field is not the
    /// documented `{ nodeId: nonNegativeInteger }` shape.
    #[must_use]
    pub fn clock(&self) -> Option<BTreeMap<String, u64>> {
        let object = self.vector_clock.as_object()?;
        let mut out = BTreeMap::new();
        for (node, value) in object {
            out.insert(node.clone(), value.as_u64()?);
        }
        Some(out)
    }

    /// Compare two envelopes by vector clock. Returns `None` when either clock is
    /// malformed, so callers fail closed rather than guessing an order.
    #[must_use]
    pub fn causal_order(&self, other: &CausalEnvelope) -> Option<CausalOrder> {
        let (a, b) = (self.clock()?, other.clock()?);
        let mut a_ahead = false;
        let mut b_ahead = false;
        for node in a.keys().chain(b.keys()) {
            let x = a.get(node).copied().unwrap_or(0);
            let y = b.get(node).copied().unwrap_or(0);
            if x > y {
                a_ahead = true;
            } else if y > x {
                b_ahead = true;
            }
        }
        Some(match (a_ahead, b_ahead) {
            (false, false) => CausalOrder::Equal,
            (true, false) => CausalOrder::After,
            (false, true) => CausalOrder::Before,
            (true, true) => CausalOrder::Concurrent,
        })
    }
}

/// The high-water mark a node has durably applied for one entity family.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncCursor {
    pub id: String,
    pub node_id: String,
    pub entity: String,
    pub last_sequence: i64,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn envelope(clock: Value, sequence: i64) -> CausalEnvelope {
        CausalEnvelope {
            id: format!("env-{sequence}"),
            entity: "runs.Run".into(),
            entity_id: "run-1".into(),
            op: SyncOp::Upsert,
            vector_clock: clock,
            payload: json!({}),
            conflict_policy: ConflictPolicy::CausalMerge,
            origin_node: "api-1".into(),
            origin_sequence: sequence,
            recorded_at: "2026-03-14T09:26:53Z".into(),
            applied_at: None,
        }
    }

    #[test]
    fn dominating_clock_is_ordered() {
        let a = envelope(json!({ "api-1": 2, "worker-7": 1 }), 2);
        let b = envelope(json!({ "api-1": 1, "worker-7": 1 }), 1);
        assert_eq!(a.causal_order(&b), Some(CausalOrder::After));
        assert_eq!(b.causal_order(&a), Some(CausalOrder::Before));
    }

    #[test]
    fn missing_node_counts_as_zero() {
        let a = envelope(json!({ "api-1": 1 }), 1);
        let b = envelope(json!({ "api-1": 1, "worker-7": 0 }), 1);
        assert_eq!(a.causal_order(&b), Some(CausalOrder::Equal));
    }

    #[test]
    fn divergent_clocks_are_concurrent() {
        let a = envelope(json!({ "api-1": 2, "worker-7": 1 }), 2);
        let b = envelope(json!({ "api-1": 1, "worker-7": 3 }), 1);
        assert_eq!(a.causal_order(&b), Some(CausalOrder::Concurrent));
    }

    #[test]
    fn malformed_clock_fails_closed() {
        let a = envelope(json!({ "api-1": -1 }), 1);
        let b = envelope(json!({ "api-1": 1 }), 1);
        assert_eq!(a.causal_order(&b), None);
        assert_eq!(envelope(json!("nope"), 1).clock(), None);
    }
}
