use std::future::Future;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::sync::Arc;

use codex_file_search::Cli;
use codex_file_search::Reporter;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchQuery;

use crate::FileSearchProvider;
use crate::SelectionContext;
use crate::SelectionPolicy;
use crate::cli_reporter::OneShotReporter;
use crate::select_provider;

/// An explicit interruption whose provider subsequently joined cleanly.
/// The binary maps this marker to exit 130. Any provider operation failure or
/// cleanup uncertainty replaces the marker with a diagnostic failure (exit 1).
#[derive(Debug)]
pub struct CliInterrupted;
impl std::fmt::Display for CliInterrupted {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("file search interrupted")
    }
}
impl std::error::Error for CliInterrupted {}

/// Explicit standalone composition profile. Entry/index/worker allocation is
/// not inferred from the result limit, and no options are silently clamped to
/// an installed provider's potentially lower negotiated limits.
#[derive(Debug, Clone)]
pub struct CliSearchPolicy {
    pub selection: SelectionPolicy,
    pub session_budget: SearchBudget,
}

/// The selected-runtime standalone path. No-pattern listing preserves the
/// legacy command and bypasses all context/catalog/provider validation.
/// `interrupted` may be Ctrl+C or an embedding's cancellation notification.
/// Every observed exit requests cleanup and inspects its retained receipt;
/// success requires Joined. An explicit Unconfirmed error can return while
/// owned drainage continues. Cancellation of this *observer* only requests
/// retained cleanup and must not itself be described as a joined exit.
pub async fn run_cli_with_context<T: Reporter>(
    cli: Cli,
    reporter: T,
    context: SelectionContext,
    policy: CliSearchPolicy,
    interrupted: impl Future<Output = std::io::Result<()>>,
) -> anyhow::Result<()> {
    if cli.pattern.is_none() {
        return codex_file_search::run_main(cli, reporter).await;
    }
    let search_directory = cli.cwd.unwrap_or_else(|| context.base_dir.clone());
    let Some(pattern) = cli.pattern else {
        unreachable!("pattern checked above")
    };
    let selection = select_provider(context, policy.selection);
    tokio::pin!(selection);
    tokio::pin!(interrupted);
    let selected = tokio::select! {
        biased;
        result = &mut selection => result?,
        signal = &mut interrupted => {
            // Do not abandon an accepted process startup when Ctrl+C arrives.
            // Its bounded initialization is retained through the real receipt.
            let selected = selection.await?;
            return with_cleanup::<()>(Err(interruption(signal)), selected.provider.shutdown().await);
        }
    };
    let provider = selected.provider;
    let request = SearchOpen {
        roots: vec![search_directory],
        options: FileSearchOptions {
            limit: cli.limit,
            exclude: cli.exclude,
            threads: cli.threads,
            compute_indices: cli.compute_indices,
            respect_gitignore: true,
        },
        budget: policy.session_budget,
    };
    let snapshot = run_selected_query(provider, request, pattern, &mut interrupted).await?;
    let shown = snapshot.matches.len();
    for matched in &snapshot.matches {
        reporter.report_match(matched);
    }
    if snapshot.total_match_count > shown {
        reporter.warn_matches_truncated(snapshot.total_match_count, shown);
    }
    Ok(())
}

async fn run_selected_query(
    provider: FileSearchProvider,
    request: SearchOpen,
    pattern: String,
    interrupted: impl Future<Output = std::io::Result<()>>,
) -> anyhow::Result<FileSearchSnapshot> {
    let operation = run_once(&provider, request, pattern);
    tokio::pin!(operation);
    tokio::pin!(interrupted);
    let result = tokio::select! {
        biased;
        result = &mut operation => result,
        signal = &mut interrupted => Err(interruption(signal)),
    };
    // Provider shutdown fences any open/update retained after the observation
    // future is abandoned, and joins the callback before results are printed.
    with_cleanup(result, provider.shutdown().await)
}

async fn run_once(
    provider: &FileSearchProvider,
    request: SearchOpen,
    pattern: String,
) -> anyhow::Result<FileSearchSnapshot> {
    let scope = provider.scope_factory().new_scope(ScopeLimits {
        max_sessions: NonZeroUsize::MIN,
    })?;
    let (reporter, response) = OneShotReporter::new();
    let compute_indices = request.options.compute_indices;
    let session = scope.open(request, Arc::new(reporter)).await?;
    session
        .update_query(SearchQuery {
            id: NonZeroU64::MIN,
            text: pattern,
        })
        .await?;
    let snapshot = tokio::select! {
        // A completed, current Idle callback wins if cleanup is also ready.
        // The outer provider receipt still retains any later operation failure.
        biased;
        result = response => result.map_err(|_| SearchError::new(SearchErrorKind::TransportLost,
            "file-search CLI completion owner ended without a current-query result"))?.map_err(anyhow::Error::from),
        outcome = session.wait_closed() => Err(crate::session::close_cause(&outcome).into()),
    }?;
    if compute_indices
        && snapshot
            .matches
            .iter()
            .any(|matched| matched.indices.is_none())
    {
        return Err(SearchError::new(
            SearchErrorKind::InvalidInput,
            "file-search provider omitted requested highlight indices",
        )
        .into());
    }
    Ok(snapshot)
}

fn interruption(result: std::io::Result<()>) -> anyhow::Error {
    match result {
        Ok(()) => CliInterrupted.into(),
        Err(error) => anyhow::Error::new(error).context("file-search interruption listener failed"),
    }
}
fn with_cleanup<T>(operation: anyhow::Result<T>, outcome: SearchCloseOutcome) -> anyhow::Result<T> {
    let mut failure = outcome
        .operation
        .err()
        .map(|error| format!("file-search provider failed: {error}"));
    if let CloseCleanup::Unconfirmed(error) = outcome.cleanup {
        let message = format!("file-search cleanup unconfirmed: {error}");
        failure = Some(failure.map_or_else(
            || message.clone(),
            |previous| format!("{previous}; {message}"),
        ));
    }
    let Some(failure) = failure else {
        // Keep typed caller cancellation only with a clean joined receipt.
        return operation;
    };
    let diagnostic = match operation {
        Ok(_) => failure,
        Err(error) => format!("{error}; {failure}"),
    };
    Err(anyhow::anyhow!(diagnostic))
}

#[cfg(test)]
#[path = "cli_lifecycle_tests.rs"]
mod lifecycle_tests;
