use std::io::IsTerminal;
use std::path::Path;

use clap::Parser;
use codex_file_search::Cli;
use codex_file_search::FileMatch;
use codex_file_search::Reporter;
use codex_file_search_runtime::CliInterrupted;
use codex_file_search_runtime::SelectionContext;
use codex_file_search_runtime::run_cli_with_context;
use codex_file_search_runtime::standalone_policy;
use serde_json::json;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) if error.downcast_ref::<CliInterrupted>().is_some() => {
            std::process::ExitCode::from(130)
        }
        Err(error) => {
            eprintln!("Error: {error:?}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let reporter = StdioReporter {
        write_output_as_json: cli.json,
        show_indices: cli.compute_indices && std::io::stdout().is_terminal(),
    };
    // Preserve the legacy no-pattern listing before resolving CODEX_HOME,
    // validating search policy, or starting any native/component provider.
    if cli.pattern.is_none() {
        return codex_file_search::run_main(cli, reporter).await;
    }
    let policy = standalone_policy(cli.threads)?;
    let context = SelectionContext {
        codex_home: codex_utils_home_dir::find_codex_home()?
            .as_path()
            .to_owned(),
        base_dir: std::env::current_dir()?,
    };
    run_cli_with_context(cli, reporter, context, policy, tokio::signal::ctrl_c()).await?;
    Ok(())
}

struct StdioReporter {
    write_output_as_json: bool,
    show_indices: bool,
}

impl Reporter for StdioReporter {
    fn report_match(&self, file_match: &FileMatch) {
        if self.write_output_as_json {
            #[allow(clippy::unwrap_used)]
            let json = serde_json::to_string(file_match).unwrap();
            println!("{json}");
        } else if self.show_indices {
            #[allow(clippy::expect_used)]
            let indices = file_match
                .indices
                .as_ref()
                .expect("--compute-indices was specified");
            // `indices` is guaranteed to be sorted in ascending order. Instead
            // of calling `contains` for every character (which would be O(N^2)
            // in the worst-case), walk through the `indices` vector once while
            // iterating over the characters.
            let mut indices_iter = indices.iter().peekable();

            for (i, c) in file_match.path.to_string_lossy().chars().enumerate() {
                match indices_iter.peek() {
                    Some(next) if **next == i as u32 => {
                        // ANSI escape code for bold: \x1b[1m ... \x1b[0m
                        print!("\x1b[1m{c}\x1b[0m");
                        // advance the iterator since we've consumed this index
                        indices_iter.next();
                    }
                    _ => {
                        print!("{c}");
                    }
                }
            }
            println!();
        } else {
            println!("{}", file_match.path.to_string_lossy());
        }
    }

    fn warn_matches_truncated(&self, total_match_count: usize, shown_match_count: usize) {
        if self.write_output_as_json {
            let value = json!({"matches_truncated": true});
            #[allow(clippy::unwrap_used)]
            let json = serde_json::to_string(&value).unwrap();
            println!("{json}");
        } else {
            eprintln!(
                "Warning: showing {shown_match_count} out of {total_match_count} results. Provide a more specific pattern or increase the --limit.",
            );
        }
    }

    fn warn_no_search_pattern(&self, search_directory: &Path) {
        eprintln!(
            "No search pattern specified. Showing the contents of the current directory ({}):",
            search_directory.to_string_lossy()
        );
    }
}
