//! Combined evidence aggregation for law verification (REQ-26).
//!
//! Takes property and proof evidence for an obligation and combines them
//! into a single finding. If either channel failed, the combined result
//! is a finding. If both passed, no finding is produced.

use crate::{CombinedEvidence, Evidence, EvidenceChannel, EvidenceStatus};

/// A combined finding for a single obligation.
#[derive(Debug, Clone)]
pub struct CombinedFinding {
    /// The obligation identifier.
    pub obligation_id: String,
    /// Combined evidence from both channels.
    pub combined: CombinedEvidence,
    /// Whether this obligation has a finding (at least one channel failed).
    pub has_finding: bool,
}

/// Aggregate evidence from multiple results into combined findings.
///
/// For each obligation, combines property and proof evidence into a single
/// `CombinedFinding`. An obligation has a finding if either channel failed
/// or if a prover disproved it.
pub fn aggregate_evidence(
    property_results: &[Evidence],
    proof_results: &[Evidence],
) -> Vec<CombinedFinding> {
    let mut findings = Vec::new();

    // Collect by obligation_id
    let mut all_evidence: std::collections::BTreeMap<String, Vec<&Evidence>> =
        std::collections::BTreeMap::new();
    for ev in property_results.iter().chain(proof_results.iter()) {
        all_evidence
            .entry(ev.obligation_id.clone())
            .or_default()
            .push(ev);
    }

    for (obligation_id, evidence_list) in all_evidence {
        let mut property_evidence: Option<Evidence> = None;
        let mut proof_evidence: Option<Evidence> = None;

        for ev in evidence_list {
            match ev.channel {
                EvidenceChannel::Property => property_evidence = Some(ev.clone()),
                EvidenceChannel::Proof => proof_evidence = Some(ev.clone()),
            }
        }

        let has_finding = property_evidence
            .as_ref()
            .is_some_and(|e| e.status != EvidenceStatus::Passed)
            || proof_evidence
                .as_ref()
                .is_some_and(|e| e.status != EvidenceStatus::Passed);

        let combined = CombinedEvidence {
            schema_version: "0.1.0".to_string(),
            obligation_id: obligation_id.clone(),
            property_evidence,
            proof_evidence,
            lifecycle_ref: None,
        };

        findings.push(CombinedFinding {
            obligation_id,
            combined,
            has_finding,
        });
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvidenceChannel, EvidenceStatus, ProverStatus};

    fn property_passed(obligation_id: &str) -> Evidence {
        Evidence {
            schema_version: "0.1.0".to_string(),
            obligation_id: obligation_id.to_string(),
            channel: EvidenceChannel::Property,
            status: EvidenceStatus::Passed,
            detail: None,
            prover_result: None,
            version: "0.1.0".to_string(),
        }
    }

    fn property_failed(obligation_id: &str) -> Evidence {
        Evidence {
            schema_version: "0.1.0".to_string(),
            obligation_id: obligation_id.to_string(),
            channel: EvidenceChannel::Property,
            status: EvidenceStatus::Failed,
            detail: Some("counterexample: [1, 2]".to_string()),
            prover_result: None,
            version: "0.1.0".to_string(),
        }
    }

    fn proof_proved(obligation_id: &str) -> Evidence {
        Evidence {
            schema_version: "0.1.0".to_string(),
            obligation_id: obligation_id.to_string(),
            channel: EvidenceChannel::Proof,
            status: EvidenceStatus::Passed,
            detail: None,
            prover_result: Some(ProverStatus::Proved),
            version: "0.1.0".to_string(),
        }
    }

    fn proof_disproved(obligation_id: &str) -> Evidence {
        Evidence {
            schema_version: "0.1.0".to_string(),
            obligation_id: obligation_id.to_string(),
            channel: EvidenceChannel::Proof,
            status: EvidenceStatus::Failed,
            detail: Some("counterexample found".to_string()),
            prover_result: Some(ProverStatus::Disproved),
            version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn both_passed_no_finding() {
        let results = aggregate_evidence(&[property_passed("obl-1")], &[proof_proved("obl-1")]);
        assert_eq!(results.len(), 1);
        assert!(!results[0].has_finding);
    }

    #[test]
    fn property_failed_creates_finding() {
        let results = aggregate_evidence(&[property_failed("obl-1")], &[proof_proved("obl-1")]);
        assert_eq!(results.len(), 1);
        assert!(results[0].has_finding);
    }

    #[test]
    fn proof_failed_creates_finding() {
        let results = aggregate_evidence(&[property_passed("obl-1")], &[proof_disproved("obl-1")]);
        assert_eq!(results.len(), 1);
        assert!(results[0].has_finding);
    }

    #[test]
    fn both_failed_creates_single_finding() {
        let results = aggregate_evidence(&[property_failed("obl-1")], &[proof_disproved("obl-1")]);
        assert_eq!(results.len(), 1);
        assert!(results[0].has_finding);
        // Both channels are present in combined evidence
        assert!(results[0].combined.property_evidence.is_some());
        assert!(results[0].combined.proof_evidence.is_some());
    }

    #[test]
    fn property_only_evidence() {
        let results = aggregate_evidence(&[property_passed("obl-1")], &[]);
        assert_eq!(results.len(), 1);
        assert!(!results[0].has_finding);
        assert!(results[0].combined.proof_evidence.is_none());
    }

    #[test]
    fn proof_only_evidence() {
        let results = aggregate_evidence(&[], &[proof_proved("obl-1")]);
        assert_eq!(results.len(), 1);
        assert!(!results[0].has_finding);
        assert!(results[0].combined.property_evidence.is_none());
    }

    #[test]
    fn multiple_obligations_separate_findings() {
        let results = aggregate_evidence(
            &[property_passed("obl-1"), property_failed("obl-2")],
            &[proof_proved("obl-1")],
        );
        assert_eq!(results.len(), 2);
        let f1 = results.iter().find(|r| r.obligation_id == "obl-1").unwrap();
        let f2 = results.iter().find(|r| r.obligation_id == "obl-2").unwrap();
        assert!(!f1.has_finding);
        assert!(f2.has_finding);
    }

    #[test]
    fn no_evidence_returns_empty() {
        let results = aggregate_evidence(&[], &[]);
        assert!(results.is_empty());
    }
}
