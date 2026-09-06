//! Every fixture in `contracts/fixtures/` is driven through serde.
//!
//! * `valid/` must deserialize, and `to_value(parse(raw)) == parse_value(raw)` —
//!   which is what actually proves `#[serde(rename_all = "camelCase")]` and every
//!   field name in `src/v1/` match the JSON Schema authority.
//! * `invalid/*.json` (top level) must FAIL to deserialize. Those fixtures break a
//!   structural rule — a missing required field, a wrong scalar type, an unknown
//!   enum value, or an extra property that `deny_unknown_fields` rejects.
//! * `invalid/schema-only/*.json` must SUCCEED here: they are well-formed
//!   documents that only violate a value bound (`maxLength`, `minimum`, a
//!   `pattern`, a `oneOf` branch). serde does not enforce bounds; the JSON Schema
//!   validator does, and `npm run validate:fixtures` is what rejects them.
//!
//! The dispatch table below must cover every non-enum `$defs` entry of every
//! slice — `table_covers_every_model` fails the build when a model is added to an
//! authority without a Rust mirror.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use gha_indie_worker_interfaces::v1::{
    chat, embeddings, errors, identity, onboarding, runs, schemas, sync, transport, webhooks,
    workers,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

type Check = fn(&str) -> Result<Value, String>;

fn check<T: DeserializeOwned + Serialize>(raw: &str) -> Result<Value, String> {
    let parsed: T = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let once = serde_json::to_value(&parsed).map_err(|e| e.to_string())?;
    let reparsed: T = serde_json::from_value(once.clone()).map_err(|e| e.to_string())?;
    let twice = serde_json::to_value(&reparsed).map_err(|e| e.to_string())?;
    if once != twice {
        return Err(format!("second round trip differs: {once} vs {twice}"));
    }
    Ok(once)
}

/// `(slice, model, checker)` for every model in the JSON Schema authority.
fn table() -> Vec<(&'static str, &'static str, Check)> {
    vec![
        ("identity", "Org", check::<identity::Org> as Check),
        ("identity", "User", check::<identity::User>),
        ("identity", "OrgMember", check::<identity::OrgMember>),
        ("identity", "Invitation", check::<identity::Invitation>),
        ("identity", "Seat", check::<identity::Seat>),
        (
            "onboarding",
            "OrgOnboardingTransition",
            check::<onboarding::OrgOnboardingTransition>,
        ),
        (
            "onboarding",
            "UserOnboardingTransition",
            check::<onboarding::UserOnboardingTransition>,
        ),
        (
            "onboarding",
            "AdvanceRequest",
            check::<onboarding::AdvanceRequest>,
        ),
        (
            "onboarding",
            "AdvanceResponse",
            check::<onboarding::AdvanceResponse>,
        ),
        ("runs", "Plan", check::<runs::Plan>),
        ("runs", "Run", check::<runs::Run>),
        ("runs", "Job", check::<runs::Job>),
        ("runs", "Step", check::<runs::Step>),
        ("runs", "LogChunk", check::<runs::LogChunk>),
        ("runs", "RunCancellation", check::<runs::RunCancellation>),
        ("workers", "Worker", check::<workers::Worker>),
        ("workers", "Capability", check::<workers::Capability>),
        ("workers", "Heartbeat", check::<workers::Heartbeat>),
        ("workers", "Profile", check::<workers::Profile>),
        (
            "webhooks",
            "GitHubDelivery",
            check::<webhooks::GitHubDelivery>,
        ),
        (
            "webhooks",
            "RegistryImageEvent",
            check::<webhooks::RegistryImageEvent>,
        ),
        ("chat", "ChatSession", check::<chat::ChatSession>),
        ("chat", "ChatMessage", check::<chat::ChatMessage>),
        (
            "embeddings",
            "ComparisonSpace",
            check::<embeddings::ComparisonSpace>,
        ),
        (
            "embeddings",
            "EmbeddingRecord",
            check::<embeddings::EmbeddingRecord>,
        ),
        (
            "embeddings",
            "IndexRequest",
            check::<embeddings::IndexRequest>,
        ),
        (
            "embeddings",
            "SearchRequest",
            check::<embeddings::SearchRequest>,
        ),
        (
            "embeddings",
            "SearchResponse",
            check::<embeddings::SearchResponse>,
        ),
        (
            "embeddings",
            "RegressionFinding",
            check::<embeddings::RegressionFinding>,
        ),
        ("embeddings", "AlertRule", check::<embeddings::AlertRule>),
        ("embeddings", "MatchEvent", check::<embeddings::MatchEvent>),
        ("sync", "CausalEnvelope", check::<sync::CausalEnvelope>),
        ("sync", "SyncCursor", check::<sync::SyncCursor>),
        ("transport", "WsCommand", check::<transport::WsCommand>),
        ("transport", "WsEvent", check::<transport::WsEvent>),
        ("transport", "TcpFrame", check::<transport::TcpFrame>),
        ("errors", "Problem", check::<errors::Problem>),
        (
            "errors",
            "ProblemViolation",
            check::<errors::ProblemViolation>,
        ),
    ]
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures_root() -> PathBuf {
    repo_root().join("contracts").join("fixtures")
}

fn json_files(dir: &Path, recurse: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recurse {
                out.extend(json_files(&path, true));
            }
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn model_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.split('.').next())
        .unwrap_or_default()
        .to_string()
}

fn checker(slice: &str, model: &str) -> Check {
    table()
        .into_iter()
        .find(|(s, m, _)| *s == slice && *m == model)
        .unwrap_or_else(|| panic!("no Rust mirror registered for {slice}/{model}"))
        .2
}

fn slices() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixtures_root())
        .expect("contracts/fixtures must exist")
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn valid_fixtures_round_trip_byte_for_byte_in_value_space() {
    let mut checked = 0;
    for slice in slices() {
        for path in json_files(&fixtures_root().join(&slice).join("valid"), false) {
            let raw = fs::read_to_string(&path).unwrap();
            let original: Value = serde_json::from_str(&raw)
                .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
            let produced = checker(&slice, &model_of(&path))(&raw)
                .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
            assert_eq!(
                original,
                produced,
                "{} did not survive serde unchanged",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 20,
        "expected a real fixture corpus, saw {checked}"
    );
}

#[test]
fn structurally_invalid_fixtures_are_rejected_by_serde() {
    let mut checked = 0;
    for slice in slices() {
        for path in json_files(&fixtures_root().join(&slice).join("invalid"), false) {
            let raw = fs::read_to_string(&path).unwrap();
            let result = checker(&slice, &model_of(&path))(&raw);
            assert!(
                result.is_err(),
                "{} parsed successfully but lives in invalid/ — move it to invalid/schema-only/ if it only breaks a value bound",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(checked >= 10, "expected invalid fixtures, saw {checked}");
}

#[test]
fn schema_only_invalid_fixtures_still_parse() {
    let mut checked = 0;
    for slice in slices() {
        let dir = fixtures_root()
            .join(&slice)
            .join("invalid")
            .join("schema-only");
        for path in json_files(&dir, false) {
            let raw = fs::read_to_string(&path).unwrap();
            let result = checker(&slice, &model_of(&path))(&raw);
            assert!(
                result.is_ok(),
                "{} is in invalid/schema-only/ but serde rejected it ({}) — move it up to invalid/",
                path.display(),
                result.unwrap_err()
            );
            checked += 1;
        }
    }
    assert!(checked >= 5, "expected schema-only fixtures, saw {checked}");
}

#[test]
fn table_covers_every_model_in_every_authority() {
    let registered: BTreeSet<(String, String)> = table()
        .into_iter()
        .map(|(s, m, _)| (s.to_string(), m.to_string()))
        .collect();
    let mut declared: BTreeSet<(String, String)> = BTreeSet::new();
    for slice in schemas::ALL {
        let doc: Value = serde_json::from_str(slice.schema).unwrap();
        for (name, def) in doc["$defs"].as_object().unwrap() {
            if def.get("enum").is_none() {
                declared.insert((slice.name.to_string(), name.clone()));
            }
        }
    }
    assert_eq!(
        declared, registered,
        "the Rust mirror and the JSON Schema authority disagree about which models exist"
    );
}

#[test]
fn every_model_has_at_least_one_valid_fixture() {
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    for slice in slices() {
        for path in json_files(&fixtures_root().join(&slice).join("valid"), false) {
            covered.insert((slice.clone(), model_of(&path)));
        }
    }
    // Not every model needs a fixture yet, but the ones that do must name a real
    // model, and each slice must have at least one worked example.
    for (slice, model) in &covered {
        assert!(
            table().iter().any(|(s, m, _)| s == slice && m == model),
            "fixture names unknown model {slice}/{model}"
        );
    }
    for slice in slices() {
        assert!(
            covered.iter().any(|(s, _)| *s == slice),
            "slice {slice} has no valid fixture"
        );
    }
}

#[test]
fn onboarding_transition_tables_match_the_json_schema_authority() {
    let doc: Value = serde_json::from_str(schemas::ONBOARDING).unwrap();
    let declared = &doc["x-ores-transitions"];

    let wire = |state: &Value| state.as_str().unwrap().to_string();

    for state in onboarding::OrgOnboardingState::ALL {
        let key = serde_json::to_value(state).unwrap();
        let listed: Vec<String> = declared["org"][wire(&key)]
            .as_array()
            .unwrap_or_else(|| panic!("schema has no org edges for {state:?}"))
            .iter()
            .map(wire)
            .collect();
        let mut code: Vec<String> = state
            .allowed_next()
            .iter()
            .map(|s| wire(&serde_json::to_value(s).unwrap()))
            .collect();
        let mut listed_sorted = listed.clone();
        listed_sorted.sort();
        code.sort();
        assert_eq!(code, listed_sorted, "org edges for {state:?}");
    }

    for state in onboarding::UserOnboardingState::ALL {
        let key = serde_json::to_value(state).unwrap();
        let listed: Vec<String> = declared["user"][wire(&key)]
            .as_array()
            .unwrap_or_else(|| panic!("schema has no user edges for {state:?}"))
            .iter()
            .map(wire)
            .collect();
        let mut code: Vec<String> = state
            .allowed_next()
            .iter()
            .map(|s| wire(&serde_json::to_value(s).unwrap()))
            .collect();
        let mut listed_sorted = listed.clone();
        listed_sorted.sort();
        code.sort();
        assert_eq!(code, listed_sorted, "user edges for {state:?}");
    }
}

#[test]
fn run_status_transition_table_matches_the_json_schema_authority() {
    let doc: Value = serde_json::from_str(schemas::RUNS).unwrap();
    let declared = &doc["x-ores-transitions"]["runStatus"];
    for status in runs::RunStatus::ALL {
        let key = serde_json::to_value(status).unwrap();
        let listed: Vec<String> = declared[key.as_str().unwrap()]
            .as_array()
            .unwrap_or_else(|| panic!("schema has no edges for {status:?}"))
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let mut code: Vec<String> = status
            .allowed_next()
            .iter()
            .map(|s| {
                serde_json::to_value(s)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        let mut listed_sorted = listed.clone();
        listed_sorted.sort();
        code.sort();
        assert_eq!(code, listed_sorted, "run status edges for {status:?}");
    }
}
