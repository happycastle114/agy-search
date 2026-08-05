use std::str::FromStr;

use super::temporal_fixtures_test::{search_request, verified_scope};
use super::*;
use crate::{error::AgyError, types::NonEmptyText};

#[test]
fn temporal_merge_rejects_partial_results_and_latest_date_ties() {
    // Given: a two-scope plan and independently validated tied scope results.
    let alpha_label = ScopeLabel::parse(&NonEmptyText::from_str("alpha").expect("valid scope"))
        .expect("bounded scope");
    let beta_label = ScopeLabel::parse(&NonEmptyText::from_str("beta").expect("valid scope"))
        .expect("bounded scope");
    let plan = TemporalRecoveryPlan::new(vec![alpha_label, beta_label]);
    let alpha = verified_scope("alpha", "alpha-v2", "2026-08-05");
    let beta = verified_scope("beta", "beta-v2", "2026-08-05");

    // When: partial and tied sets are merged.
    let partial = merge_verified_scopes(&search_request(), &plan, std::slice::from_ref(&alpha));
    let tied = merge_verified_scopes(&search_request(), &plan, &[alpha, beta]);

    // Then: neither can produce a public winner.
    assert!(matches!(partial, Err(AgyError::OutputInvalid)));
    assert!(matches!(tied, Err(AgyError::OutputInvalid)));
}
