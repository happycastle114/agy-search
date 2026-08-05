//! Temporal source recovery and bounded scoped execution.

use std::{collections::BTreeMap, future::Future, sync::Arc};

use tokio::task::JoinSet;

use super::{ExecutionContext, run_content_once};
use crate::{
    error::AgyError,
    prompt::build_scope_prompt,
    request::{SearchRequest, TemporalScopeRequest},
    response::{Document as ResponseDocument, temporal_scope_schema},
    response_models::EvidenceAudit,
    source_restriction::SourceRestriction,
    source_verification::{LocalFactRecovery, VerifiedSources},
    types::{Operation, ResearchToolPolicy},
    verification::{
        ScopeLabel, TemporalRecoveryPlan, merge_verified_scopes, validate_scope_result,
        verified_scope_from_source_fact,
    },
};

pub(super) const MAX_RECOVERY_CONCURRENCY: usize = 4;

pub(super) async fn recover_temporal(
    context: ExecutionContext,
    request: &SearchRequest,
    plan: TemporalRecoveryPlan,
    sources: Arc<VerifiedSources>,
    primary_audit: EvidenceAudit,
) -> Result<ResponseDocument, AgyError> {
    let contract = request
        .temporal_contract
        .as_ref()
        .ok_or(AgyError::OutputInvalid)?;
    if let LocalFactRecovery::Complete(facts) =
        sources.recover_local_facts(&plan, &primary_audit, contract.cutoff())?
    {
        let verified = plan
            .scopes()
            .iter()
            .cloned()
            .zip(&facts)
            .map(|(scope, fact)| verified_scope_from_source_fact(scope, fact, contract))
            .collect::<Result<Vec<_>, _>>()?;
        return merge_verified_scopes(request, &plan, &verified);
    }
    let schema = temporal_scope_schema(&request.source_restriction)?;
    let original = request.clone();
    let verified = run_bounded(plan.scopes(), move |scope| {
        let context = context.clone();
        let request = TemporalScopeRequest::from_search(&original, scope.clone());
        let schema = schema.clone();
        let sources = Arc::clone(&sources);
        async move {
            let request_json = request.to_json().map_err(|_| AgyError::InvalidCommand)?;
            let prompt = build_scope_prompt(&request_json);
            let response = run_content_once(
                &context,
                Operation::Search,
                match &request.source_restriction {
                    SourceRestriction::Unrestricted => ResearchToolPolicy::ScopedTemporalSearch(
                        request.required_search_query.clone(),
                    ),
                    restriction @ SourceRestriction::Allowlist { .. } => {
                        ResearchToolPolicy::RestrictedScopedTemporalSearch {
                            required_query: request.required_search_query.clone(),
                            restriction: Box::new(restriction.clone()),
                        }
                    }
                },
                schema,
                prompt,
            )
            .await?;
            let ResponseDocument::Search(result) = &response else {
                return Err(AgyError::OutputInvalid);
            };
            sources.verify_audit(&result.evidence_audit)?;
            let contract = request
                .temporal_contract
                .as_ref()
                .ok_or(AgyError::OutputInvalid)?;
            validate_scope_result(scope, response, contract)
        }
    })
    .await?;
    merge_verified_scopes(request, &plan, &verified)
}

pub(super) async fn run_bounded<T, Run, RunFuture>(
    scopes: &[ScopeLabel],
    run_scope: Run,
) -> Result<Vec<T>, AgyError>
where
    T: Send + 'static,
    Run: Fn(ScopeLabel) -> RunFuture + Clone + Send + 'static,
    RunFuture: Future<Output = Result<T, AgyError>> + Send + 'static,
{
    let mut pending = JoinSet::new();
    let mut next = 0;
    let mut completed = BTreeMap::new();
    while next < scopes.len() || !pending.is_empty() {
        while next < scopes.len() && pending.len() < MAX_RECOVERY_CONCURRENCY {
            let scope = scopes.get(next).cloned().ok_or(AgyError::OutputInvalid)?;
            let runner = run_scope.clone();
            let index = next;
            pending.spawn(async move { (index, runner(scope).await) });
            next += 1;
        }
        match pending.join_next().await {
            Some(Ok((index, Ok(result)))) => {
                completed.insert(index, result);
            }
            Some(Ok((_, Err(error)))) => {
                abort_and_drain(&mut pending).await;
                return Err(error);
            }
            Some(Err(_)) => {
                abort_and_drain(&mut pending).await;
                return Err(AgyError::OutputInvalid);
            }
            None => {}
        }
    }
    if completed.len() != scopes.len() {
        return Err(AgyError::OutputInvalid);
    }
    Ok(completed.into_values().collect())
}

async fn abort_and_drain<T>(pending: &mut JoinSet<(usize, Result<T, AgyError>)>)
where
    T: Send + 'static,
{
    pending.abort_all();
    while pending.join_next().await.is_some() {}
}
