use super::preferred_search_model;
use crate::types::{Effort, Operation, PreferredSearchModel, VerificationMode};

#[test]
fn prefers_only_unpinned_low_effort_standard_search() {
    // Given: every operation, verification mode, and supported effort class.
    let ineligible = [
        (
            Operation::Search,
            VerificationMode::TemporalComparison,
            Some(Effort::Low),
        ),
        (
            Operation::Search,
            VerificationMode::Standard,
            Some(Effort::Medium),
        ),
        (
            Operation::Search,
            VerificationMode::Standard,
            Some(Effort::High),
        ),
        (Operation::Search, VerificationMode::Standard, None),
        (
            Operation::Research,
            VerificationMode::Standard,
            Some(Effort::Low),
        ),
        (
            Operation::Extract,
            VerificationMode::Standard,
            Some(Effort::Low),
        ),
        (
            Operation::Map,
            VerificationMode::Standard,
            Some(Effort::Low),
        ),
        (
            Operation::Crawl,
            VerificationMode::Standard,
            Some(Effort::Low),
        ),
    ];

    // When: preference eligibility is resolved before advisory discovery.
    let preferred = preferred_search_model(
        Operation::Search,
        VerificationMode::Standard,
        Some(Effort::Low),
    );

    // Then: only ordinary low-effort Search can request the fixed typed preference.
    assert_eq!(preferred, Some(PreferredSearchModel::Gemini36FlashLow));
    for (operation, verification, effort) in ineligible {
        assert_eq!(
            preferred_search_model(operation, verification, effort),
            None
        );
    }
}
