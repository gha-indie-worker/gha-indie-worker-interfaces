#![forbid(unsafe_code)]

//! Serde mirror of the `embeddings` slice.
//!
//! The central rule is [`ComparisonSpace::identity`]: two vectors are comparable
//! only when all twelve identity fields are equal. Nothing in this crate compares
//! vectors; the identity tuple is the value the servers key their caches and
//! indexes on.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddingRole {
    Query,
    Document,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnnStrategy {
    HalfvecExact,
    HalfvecMrl,
    BinaryFullRerank,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistanceMetric {
    Cosine,
    L2,
    InnerProduct,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Normalization {
    None,
    L2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegressionKind {
    Drift,
    Collapse,
    Decoupling,
    Saturation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchOutcome {
    Accepted,
    Rejected,
    Deferred,
}

/// The largest native dimensionality the fleet indexes.
pub const MAX_DIMS_NATIVE: i32 = 4096;
/// The wire bound on a vector array, leaving room for a bounded MRL suffix.
pub const MAX_VECTOR_LEN: usize = 4100;

/// The twelve-field identity of a comparison space, as a comparable tuple.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SpaceIdentity<'a> {
    pub tenant_id: &'a str,
    pub entity_kind: &'a str,
    pub entity_id: &'a str,
    pub model_family: &'a str,
    pub model_name: &'a str,
    pub model_version: &'a str,
    pub dims_native: i32,
    pub role: EmbeddingRole,
    pub normalization: Normalization,
    pub distance_metric: DistanceMetric,
    pub ann_strategy: AnnStrategy,
    pub generation_profile: &'a str,
}

/// The twelve-field identity of a comparison space.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonSpace {
    pub id: String,
    pub tenant_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub model_family: String,
    pub model_name: String,
    pub model_version: String,
    pub dims_native: i32,
    pub role: EmbeddingRole,
    pub normalization: Normalization,
    pub distance_metric: DistanceMetric,
    pub ann_strategy: AnnStrategy,
    pub generation_profile: String,
    pub created_at: String,
}

impl ComparisonSpace {
    /// The identity tuple. `id` and `createdAt` are deliberately excluded: two
    /// rows with different surrogate keys but the same identity describe the
    /// same space and their vectors are comparable.
    #[must_use]
    pub fn identity(&self) -> SpaceIdentity<'_> {
        SpaceIdentity {
            tenant_id: &self.tenant_id,
            entity_kind: &self.entity_kind,
            entity_id: &self.entity_id,
            model_family: &self.model_family,
            model_name: &self.model_name,
            model_version: &self.model_version,
            dims_native: self.dims_native,
            role: self.role,
            normalization: self.normalization,
            distance_metric: self.distance_metric,
            ann_strategy: self.ann_strategy,
            generation_profile: &self.generation_profile,
        }
    }

    /// Whether a vector from `other` may be compared with a vector from `self`.
    #[must_use]
    pub fn comparable_with(&self, other: &ComparisonSpace) -> bool {
        self.identity() == other.identity()
    }

    /// Whether `dims` is an acceptable stored width for this space.
    /// Only `halfvec-mrl` admits a truncated matryoshka prefix.
    #[must_use]
    pub fn accepts_dims(&self, dims: i32) -> bool {
        match self.ann_strategy {
            AnnStrategy::HalfvecMrl => (1..=self.dims_native).contains(&dims),
            AnnStrategy::HalfvecExact | AnnStrategy::BinaryFullRerank => dims == self.dims_native,
        }
    }
}

/// One stored vector inside a comparison space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingRecord {
    pub id: String,
    pub space_id: String,
    pub entity_id: String,
    pub vector: Vec<f64>,
    pub dims: i32,
    pub checksum: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    pub created_at: String,
}

/// A request to (re)index a bounded set of entities into one comparison space.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexRequest {
    pub id: String,
    pub space_id: String,
    pub entity_ids: Vec<String>,
    pub priority: i32,
    pub idempotency_key: String,
    pub requested_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
}

/// A vector search inside one comparison space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchRequest {
    pub id: String,
    pub space_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_text: Option<String>,
    pub query_vector: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lexical_prefilter: Option<String>,
    pub top_k: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,
    pub requested_at: String,
}

/// The settled result of a [`SearchRequest`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchResponse {
    pub id: String,
    pub request_id: String,
    pub matched_entity_ids: Vec<String>,
    pub scores: Vec<f64>,
    pub prefilter_hits: i32,
    pub elapsed_millis: i32,
    pub truncated: bool,
    pub responded_at: String,
}

impl SearchResponse {
    /// `matchedEntityIds` and `scores` are positionally aligned.
    #[must_use]
    pub fn is_aligned(&self) -> bool {
        self.matched_entity_ids.len() == self.scores.len()
    }
}

/// A statistical relationship detected between two entities in a space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegressionFinding {
    pub id: String,
    pub space_id: String,
    pub kind: RegressionKind,
    pub x_entity: String,
    pub y_entity: String,
    pub coefficient: f64,
    pub p_value: f64,
    pub window: String,
    pub sample_n: i32,
    pub detected_at: String,
}

impl RegressionFinding {
    /// The bounds the authorities state: coefficient in -1..=1, pValue in 0..=1.
    #[must_use]
    pub fn bounds_hold(&self) -> bool {
        (-1.0..=1.0).contains(&self.coefficient)
            && (0.0..=1.0).contains(&self.p_value)
            && self.sample_n >= 1
    }
}

/// A standing rule that turns regression findings into alerts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlertRule {
    pub id: String,
    pub space_id: String,
    pub name: String,
    pub kind: RegressionKind,
    pub severity: AlertSeverity,
    pub threshold_coefficient: f64,
    pub max_p_value: f64,
    pub min_sample_n: i32,
    pub enabled: bool,
    pub notify_channels: Vec<String>,
    pub created_at: String,
}

impl AlertRule {
    /// Whether this rule fires on `finding`. Rules only ever look inside one space.
    #[must_use]
    pub fn matches(&self, finding: &RegressionFinding) -> bool {
        self.enabled
            && self.space_id == finding.space_id
            && self.kind == finding.kind
            && finding.sample_n >= self.min_sample_n
            && finding.p_value <= self.max_p_value
            && finding.coefficient.abs() >= self.threshold_coefficient.abs()
    }
}

/// A human or policy decision about one retrieved match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatchEvent {
    pub id: String,
    pub space_id: String,
    pub query_entity_id: String,
    pub matched_entity_id: String,
    pub score: f64,
    pub outcome: MatchOutcome,
    pub occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_user_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(strategy: AnnStrategy) -> ComparisonSpace {
        ComparisonSpace {
            id: "space-1".into(),
            tenant_id: "tenant-1".into(),
            entity_kind: "workflow".into(),
            entity_id: "ci.yml".into(),
            model_family: "e5".into(),
            model_name: "e5-large-v2".into(),
            model_version: "2024-05".into(),
            dims_native: 1024,
            role: EmbeddingRole::Document,
            normalization: Normalization::L2,
            distance_metric: DistanceMetric::Cosine,
            ann_strategy: strategy,
            generation_profile: "batch-nightly".into(),
            created_at: "2026-03-14T09:26:53Z".into(),
        }
    }

    #[test]
    fn identity_ignores_surrogate_key_and_timestamp() {
        let a = space(AnnStrategy::HalfvecMrl);
        let mut b = a.clone();
        b.id = "space-2".into();
        b.created_at = "2026-04-01T00:00:00Z".into();
        assert!(a.comparable_with(&b));
    }

    #[test]
    fn every_identity_field_breaks_comparability() {
        let a = space(AnnStrategy::HalfvecMrl);
        type Mutation = Box<dyn Fn(&mut ComparisonSpace)>;
        let mutations: Vec<Mutation> = vec![
            Box::new(|s| s.tenant_id = "other".into()),
            Box::new(|s| s.entity_kind = "other".into()),
            Box::new(|s| s.entity_id = "other".into()),
            Box::new(|s| s.model_family = "other".into()),
            Box::new(|s| s.model_name = "other".into()),
            Box::new(|s| s.model_version = "other".into()),
            Box::new(|s| s.dims_native = 512),
            Box::new(|s| s.role = EmbeddingRole::Query),
            Box::new(|s| s.normalization = Normalization::None),
            Box::new(|s| s.distance_metric = DistanceMetric::InnerProduct),
            Box::new(|s| s.ann_strategy = AnnStrategy::BinaryFullRerank),
            Box::new(|s| s.generation_profile = "other".into()),
        ];
        assert_eq!(
            mutations.len(),
            12,
            "the identity has exactly twelve fields"
        );
        for mutate in mutations {
            let mut b = a.clone();
            mutate(&mut b);
            assert!(!a.comparable_with(&b));
        }
    }

    #[test]
    fn only_mrl_admits_a_truncated_prefix() {
        assert!(space(AnnStrategy::HalfvecMrl).accepts_dims(256));
        assert!(!space(AnnStrategy::HalfvecExact).accepts_dims(256));
        assert!(space(AnnStrategy::HalfvecExact).accepts_dims(1024));
        assert!(!space(AnnStrategy::HalfvecMrl).accepts_dims(2048));
    }

    #[test]
    fn alert_rule_stays_inside_its_space() {
        let finding = RegressionFinding {
            id: "f1".into(),
            space_id: "space-1".into(),
            kind: RegressionKind::Decoupling,
            x_entity: "ci.yml".into(),
            y_entity: "release.yml".into(),
            coefficient: -0.42,
            p_value: 0.013,
            window: "p30d".into(),
            sample_n: 412,
            detected_at: "2026-03-14T09:27:31Z".into(),
        };
        assert!(finding.bounds_hold());
        let rule = AlertRule {
            id: "r1".into(),
            space_id: "space-1".into(),
            name: "decoupling".into(),
            kind: RegressionKind::Decoupling,
            severity: AlertSeverity::Warning,
            threshold_coefficient: 0.3,
            max_p_value: 0.05,
            min_sample_n: 100,
            enabled: true,
            notify_channels: vec!["slack".into()],
            created_at: "2026-03-01T00:00:00Z".into(),
        };
        assert!(rule.matches(&finding));
        assert!(!AlertRule {
            space_id: "space-2".into(),
            ..rule.clone()
        }
        .matches(&finding));
        assert!(!AlertRule {
            enabled: false,
            ..rule.clone()
        }
        .matches(&finding));
        assert!(!AlertRule {
            min_sample_n: 1000,
            ..rule
        }
        .matches(&finding));
    }
}
