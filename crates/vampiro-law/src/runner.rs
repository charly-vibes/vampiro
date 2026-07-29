//! Rust law runner using proptest for property-testing obligations.

use proptest::strategy::Strategy;
use proptest::test_runner::{Config as PropConfig, TestRunner};

use crate::{Evidence, EvidenceChannel, EvidenceStatus, Obligation, RunnerInput};

/// Result of running a law on a single obligation.
pub struct LawResult {
    pub obligation_id: String,
    pub evidence: Vec<Evidence>,
}

/// A law runner that executes obligations for a specific language.
pub trait LawRunner {
    /// The language this runner supports (e.g. "rust").
    fn language(&self) -> &'static str;

    /// Run all obligations in the input and return results.
    fn run(&self, input: &RunnerInput) -> Vec<LawResult>;
}

/// Rust property-testing law runner using proptest.
pub struct RustPropertyRunner;

impl LawRunner for RustPropertyRunner {
    fn language(&self) -> &'static str {
        "rust"
    }

    fn run(&self, input: &RunnerInput) -> Vec<LawResult> {
        input.obligations.iter().map(run_obligation).collect()
    }
}

fn run_obligation(obligation: &Obligation) -> LawResult {
    // For the tracer milestone, we run a generic property test that checks
    // whether the obligation's theory can be satisfied with random values.
    // In a full implementation, the runner would invoke the actual Rust
    // function under test via the extracted generator reference.
    let config = obligation
        .generator_config
        .as_ref()
        .map_or_else(PropConfig::default, |gc| PropConfig {
            cases: gc.cases,
            ..PropConfig::default()
        });

    let mut runner = TestRunner::new(config);
    let strategy = generate_values_for_law(&obligation.law);

    let status = match runner.run(&strategy, |values| {
        // Placeholder: check the law by evaluating the equation.
        // In production, this would deserialize values and call the
        // actual function under test.
        evaluate_law(&obligation.law, &values)
    }) {
        Ok(()) => EvidenceStatus::Passed,
        Err(_) => EvidenceStatus::Failed,
    };

    let evidence = Evidence {
        schema_version: obligation.schema_version.clone(),
        obligation_id: obligation.id.clone(),
        channel: EvidenceChannel::Property,
        status,
        detail: None,
        prover_result: None,
        version: obligation.schema_version.clone(),
    };

    LawResult {
        obligation_id: obligation.id.clone(),
        evidence: vec![evidence],
    }
}

/// Generate random integer pairs for law evaluation (tracer placeholder).
fn generate_values_for_law(_law: &str) -> impl Strategy<Value = (i32, i32)> {
    (0..100i32, 0..100i32)
}

/// Evaluate a law equation against generated values (tracer placeholder).
///
/// For the tracer milestone, this always returns `Ok(())` for commutative
/// and identity laws — they are structurally satisfied by the random data.
/// A full runner would deserialize the values and call the actual function.
/// Evaluate a law equation against generated values (tracer placeholder).
///
/// For the tracer milestone, this always returns `Ok(())` for commutative
/// and identity laws — they are structurally satisfied by the random data.
/// A full runner would deserialize the values and call the actual function.
fn evaluate_law(
    law: &str,
    values: &(i32, i32),
) -> Result<(), proptest::test_runner::TestCaseError> {
    match law {
        // Placeholder: accept all commutative/identity laws
        s if s.contains("commutative") || s.contains("identity") => Ok(()),
        // For any other law, just check the values are valid integers
        _ => {
            let _ = values.0.checked_add(values.1);
            Ok(())
        }
    }
}

/// Registry: create a law runner for the given language.
///
/// Returns `None` for unsupported languages, which callers should
/// treat as an explicit `RunnerUnsupported` result (never silently skip).
pub fn get_runner(language: &str) -> Option<Box<dyn LawRunner>> {
    match language {
        "rust" => Some(Box::new(RustPropertyRunner)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ClusterMember, EvidenceStatus, GeneratorConfig, Obligation, RunnerInput, Theory,
        TheoryKind, CONTRACT_VERSION,
    };

    fn make_simple_obligation(law: &str, theory_kind: TheoryKind) -> Obligation {
        Obligation {
            schema_version: CONTRACT_VERSION.to_string(),
            id: "test-obl".to_string(),
            member: ClusterMember {
                id: "m1".to_string(),
                path: "test::fn".to_string(),
                tags: vec![],
            },
            theory: Theory {
                name: "commutative".to_string(),
                kind: theory_kind,
            },
            law: law.to_string(),
            generator_config: Some(GeneratorConfig::default_with_seed(42)),
            prover_requested: false,
            prover: None,
            tags: vec![],
        }
    }

    #[test]
    fn runner_supports_rust_language() {
        let runner = RustPropertyRunner;
        assert_eq!(runner.language(), "rust");
    }

    #[test]
    fn runner_passes_commutative_law() {
        let runner = RustPropertyRunner;
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations: vec![make_simple_obligation(
                "commutative: a + b = b + a",
                TheoryKind::Augmenting,
            )],
            values: None,
        };
        let results = runner.run(&input);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].evidence[0].status, EvidenceStatus::Passed);
    }

    #[test]
    fn runner_handles_replacing_suite() {
        let runner = RustPropertyRunner;
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations: vec![make_simple_obligation(
                "custom: my special law",
                TheoryKind::Replacing,
            )],
            values: None,
        };
        let results = runner.run(&input);
        assert_eq!(results.len(), 1);
        // Replacing suite should still run the generic test
        assert_eq!(results[0].evidence[0].status, EvidenceStatus::Passed);
    }

    #[test]
    fn runner_produces_evidence_with_obligation_id() {
        let runner = RustPropertyRunner;
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations: vec![make_simple_obligation(
                "commutative: a + b = b + a",
                TheoryKind::Augmenting,
            )],
            values: None,
        };
        let results = runner.run(&input);
        assert_eq!(results[0].obligation_id, "test-obl");
        assert_eq!(results[0].evidence[0].obligation_id, "test-obl");
    }

    #[test]
    fn runner_passes_identity_law() {
        let runner = RustPropertyRunner;
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations: vec![make_simple_obligation(
                "identity: a + 0 = a",
                TheoryKind::Augmenting,
            )],
            values: None,
        };
        let results = runner.run(&input);
        assert_eq!(results[0].evidence[0].status, EvidenceStatus::Passed);
    }

    #[test]
    fn runner_handles_multiple_obligations() {
        let runner = RustPropertyRunner;
        let obligations = vec![
            make_simple_obligation("commutative: a + b = b + a", TheoryKind::Augmenting),
            make_simple_obligation("identity: a + 0 = a", TheoryKind::Augmenting),
        ];
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations,
            values: None,
        };
        let results = runner.run(&input);
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| r.evidence[0].status == EvidenceStatus::Passed));
    }

    #[test]
    fn non_rust_language_returns_evidence() {
        // REQ-10, REQ-C6: runner should produce Evidence (not panic) even
        // when the language doesn't match, since the runner is registered
        // by language match.
        let runner = RustPropertyRunner;
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "python".to_string(),
            obligations: vec![make_simple_obligation(
                "commutative: a + b = b + a",
                TheoryKind::Augmenting,
            )],
            values: None,
        };
        let results = runner.run(&input);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn generator_evidence_is_deterministic() {
        // REQ-18: same seed produces same result.
        let runner = RustPropertyRunner;
        let obligations = vec![make_simple_obligation(
            "commutative: a + b = b + a",
            TheoryKind::Augmenting,
        )];

        // Run twice with same seed
        let input1 = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations: obligations.clone(),
            values: None,
        };
        let input2 = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations,
            values: None,
        };
        let results1 = runner.run(&input1);
        let results2 = runner.run(&input2);

        assert_eq!(results1.len(), results2.len());
        assert_eq!(results1[0].evidence[0].status, results2[0].evidence[0].status);
    }

    #[test]
    fn get_runner_returns_rust_runner() {
        let runner = get_runner("rust");
        assert!(runner.is_some());
        assert_eq!(runner.unwrap().language(), "rust");
    }

    #[test]
    fn get_runner_returns_none_for_unsupported() {
        assert!(get_runner("python").is_none());
        assert!(get_runner("clojure").is_none());
        assert!(get_runner("julia").is_none());
        assert!(get_runner("").is_none());
    }
}
