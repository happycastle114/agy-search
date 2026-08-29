use super::{ModelPreference, preferred_model_policy};
use crate::types::{Effort, GeminiFlashGeneration, Operation, PreferredModel, VerificationMode};

#[test]
fn model_policy_uses_gemini_3_7_low_with_escalating_search_recovery() {
    let preferred = preferred_model_policy(
        Operation::Search,
        VerificationMode::Standard,
        Some(Effort::Low),
    );

    assert_eq!(
        preferred,
        Some(ModelPreference {
            primary: PreferredModel::gemini_flash(GeminiFlashGeneration::V3_7, Effort::Low,),
            recovery_generation: Some(GeminiFlashGeneration::V3_7),
        })
    );
}

#[test]
fn model_policy_uses_gemini_3_7_for_quality_and_content_operations() {
    for (operation, verification, effort) in [
        (
            Operation::Search,
            VerificationMode::Standard,
            Effort::Medium,
        ),
        (
            Operation::Search,
            VerificationMode::TemporalComparison,
            Effort::High,
        ),
        (Operation::Research, VerificationMode::Standard, Effort::Low),
        (Operation::Extract, VerificationMode::Standard, Effort::Low),
        (Operation::Map, VerificationMode::Standard, Effort::Medium),
        (Operation::Crawl, VerificationMode::Standard, Effort::High),
    ] {
        assert_eq!(
            preferred_model_policy(operation, verification, Some(effort)),
            Some(ModelPreference {
                primary: PreferredModel::gemini_flash(GeminiFlashGeneration::V3_7, effort,),
                recovery_generation: None,
            })
        );
    }
    assert_eq!(
        preferred_model_policy(Operation::Research, VerificationMode::Standard, None),
        None
    );
}
