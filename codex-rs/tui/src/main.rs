use clap::Parser;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_config::LoaderOverrides;
use codex_core_plugins::startup_sync::CuratedProcessShutdown;
use codex_tui::Cli;
use codex_tui::ExitReason;
use codex_tui::run_main_with_process_final;
use codex_utils_cli::CliConfigOverrides;
use codex_utils_process::process_shutdown::ProcessFinalCapability;
use std::io::Write;
use supports_color::Stream;

#[derive(Parser, Debug)]
struct TopCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    inner: Cli,
}

fn main() -> anyhow::Result<()> {
    codex_build_info::initialize!();
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let top_cli = TopCli::parse();
        let mut inner = top_cli.inner;
        inner
            .config_overrides
            .raw_overrides
            .splice(0..0, top_cli.config_overrides.raw_overrides);
        let capability = ProcessFinalCapability::for_executable();
        let result = run_main_with_process_final(
            inner,
            arg0_paths,
            LoaderOverrides::default(),
            /*explicit_remote_endpoint*/ None,
            capability.clone(),
        )
        .await;
        // Observe cleanup even when startup returned an error, before arg0's
        // runtime drops and its process-final watchdog is released.
        let deadline = capability.begin().graceful();
        let shutdown = CuratedProcessShutdown::begin(deadline);
        let observed = shutdown.wait_until(deadline).await;
        let cleanup = if observed.is_complete() && observed.clean_native_ownership() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "curated process cleanup incomplete; ownership is unconfirmed"
            ))
        };
        let exit_info = match (result, cleanup) {
            (Ok(exit_info), Ok(())) => exit_info,
            (Err(error), Ok(())) => return Err(error.into()),
            (Ok(exit_info), Err(cleanup)) => {
                return Err(match exit_info.exit_reason {
                    ExitReason::Fatal(message) => anyhow::anyhow!(message).context(cleanup),
                    _ => cleanup,
                });
            }
            (Err(error), Err(cleanup)) => {
                return Err(anyhow::Error::new(error).context(cleanup));
            }
        };
        let fatal_message = match &exit_info.exit_reason {
            ExitReason::Fatal(message) => Some(message.clone()),
            ExitReason::UserRequested
            | ExitReason::Archived(_)
            | ExitReason::TurnInterrupted
            | ExitReason::ThreadRemoved => None,
        };

        let color_enabled = supports_color::on(Stream::Stdout).is_some();
        for line in exit_info.format_exit_messages(color_enabled) {
            println!("{line}");
        }
        if let Some(message) = fatal_message {
            std::io::stdout().flush()?;
            return Err(anyhow::anyhow!(message));
        }
        Ok(())
    })
}
