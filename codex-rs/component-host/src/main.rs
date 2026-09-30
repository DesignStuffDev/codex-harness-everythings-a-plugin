use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use clap::Subcommand;
use codex_component_host::ComponentCatalog;
use serde_json::Value;
use serde_json::json;

mod launch;

#[derive(Parser)]
#[command(about = "Manage independently installed Codex engine components")]
struct Args {
    /// Uses the same home directory as the Codex runtime.
    #[arg(long)]
    codex_home: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Install {
        package: PathBuf,
    },
    Remove {
        plugin: String,
    },
    List,
    Select {
        kind: String,
        name: String,
        plugin: String,
    },
    Reset {
        kind: String,
        name: String,
    },
    /// Invoke an installed contract directly for diagnostics.
    Call {
        kind: String,
        name: String,
        method: String,
        #[arg(default_value = "{}")]
        params: String,
    },
    /// Run an installed graphical presentation plugin against the real harness.
    Launch {
        name: String,
        #[arg(long)]
        codex_bin: PathBuf,
        #[arg(long, default_value_t = 0)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let codex_home = args
        .codex_home
        .or_else(|| std::env::var_os("CODEX_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .context("set --codex-home or CODEX_HOME")?;
    match args.command {
        Command::Install { package } => {
            println!("{}", codex_component_host::install(&codex_home, &package)?)
        }
        Command::Remove { plugin } => codex_component_host::remove(&codex_home, &plugin)?,
        Command::Select { kind, name, plugin } => {
            codex_component_host::select(&codex_home, &kind, &name, Some(&plugin))?
        }
        Command::Reset { kind, name } => {
            codex_component_host::select(&codex_home, &kind, &name, None)?
        }
        Command::List => {
            let catalog = ComponentCatalog::load(&codex_home)?;
            let settings = catalog.settings();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "enabled": settings.enabled,
                    "installed": settings.installed,
                    "selections": settings.selections
                }))?
            );
        }
        Command::Call {
            kind,
            name,
            method,
            params,
        } => {
            if kind == "thread_store" {
                bail!(
                    "thread_store requires the persistent storage facade; use ComponentBinding::connect and the thread-store-component SDK initialization/lease lifecycle instead of one-shot call"
                );
            }
            if kind == "auth" {
                bail!(
                    "auth components can only be invoked by the credential provider; diagnostic output is disabled"
                );
            }
            let binding = find(&codex_home, &kind, &name)?;
            let result = binding
                .call(&method, serde_json::from_str::<Value>(&params)?)
                .await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Launch {
            name,
            codex_bin,
            port,
        } => {
            let binding = find(&codex_home, "presentation", &name)?;
            let params = json!({
                "codex_bin": codex_bin.canonicalize()?, "codex_home": codex_home.canonicalize()?,
                "cwd": std::env::current_dir()?.canonicalize()?,
                "host": "127.0.0.1", "port": port
            });
            launch::run(
                &binding,
                params,
                launch::SHUTDOWN_GRACE,
                tokio::signal::ctrl_c,
            )
            .await?;
        }
    }
    Ok(())
}

fn find(
    codex_home: &std::path::Path,
    kind: &str,
    name: &str,
) -> Result<codex_component_host::ComponentBinding> {
    let catalog = ComponentCatalog::load(codex_home)?;
    let mut bindings = catalog
        .components(kind)
        .into_iter()
        .filter(|binding| binding.spec.name == name);
    let binding = bindings
        .next()
        .context("requested component is not installed and enabled")?;
    if bindings.next().is_some() {
        bail!("select one component implementation before invoking it");
    }
    Ok(binding)
}
