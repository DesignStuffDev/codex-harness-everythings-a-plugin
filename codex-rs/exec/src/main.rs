//! Entry-point for the `codex-exec` binary.
//!
//! When this CLI is invoked normally, it parses the standard `codex-exec` CLI
//! options and launches the non-interactive Codex agent. However, if it is
//! invoked with arg0 as `codex-linux-sandbox`, we instead treat the invocation
//! as a request to run the logic for the standalone `codex-linux-sandbox`
//! executable (i.e., parse any -s args and then run a *sandboxed* command under
//! Landlock + seccomp.
//!
//! This allows us to ship a completely separate set of functionality as part
//! of the `codex-exec` binary.
use clap::Parser;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_exec::Cli;
use codex_exec::run_main_with_process_final;
use codex_utils_cli::CliConfigOverrides;
use codex_utils_process::process_shutdown::ProcessFinalCapability;

mod process_final;

#[derive(Parser, Debug)]
struct TopCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    inner: Cli,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let top_cli = TopCli::parse();
        // Merge root-level overrides into inner CLI struct so downstream logic remains unchanged.
        let mut inner = top_cli.inner;
        inner
            .config_overrides
            .prepend_root_overrides(top_cli.config_overrides);

        let process_final = ProcessFinalCapability::for_executable();
        let operation = run_main_with_process_final(inner, arg0_paths, process_final.clone()).await;
        process_final::finish(operation, &process_final).await
    })
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
