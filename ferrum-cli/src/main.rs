mod commands;
mod doctor;
mod templates;
mod test_drive;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "ferrum",
    version,
    author = "Roberto de Souza <rabbittrix@hotmail.com>",
    about = "Ferrum — next-generation Infrastructure as Code",
    long_about = "High-performance, memory-safe IaC tool written in Rust.\n\
                  Faster than Terraform. More secure than OpenTofu. More predictable than Pulumi."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Disable anonymous install telemetry
    #[arg(long, global = true, env = "FERRUM_TELEMETRY_DISABLED")]
    no_telemetry: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Ferrum project
    Init {
        /// Project directory (default: current)
        #[arg(default_value = ".")]
        path: String,
        /// Scaffold from template: docker-local, aws-web-app, azure-k8s-cluster
        #[arg(long)]
        template: Option<String>,
        /// Passphrase for state encryption
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    /// Show execution plan
    Plan {
        /// Path to ferrum.fstate
        #[arg(short, long, default_value = "ferrum.fstate")]
        state: String,
        /// Passphrase for state decryption
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    /// Apply planned changes
    Apply {
        #[arg(short, long, default_value = "ferrum.fstate")]
        state: String,
        #[arg(short, long)]
        passphrase: Option<String>,
        /// Auto-approve without confirmation (alias: --yes)
        #[arg(short = 'y', long = "auto-approve")]
        auto_approve: bool,
    },
    /// Destroy all managed infrastructure
    Destroy {
        #[arg(short, long, default_value = "ferrum.fstate")]
        state: String,
        #[arg(short, long)]
        passphrase: Option<String>,
        #[arg(short = 'y', long = "auto-approve")]
        auto_approve: bool,
    },
    /// System health checks (PATH, credentials, Docker, updates)
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Import Terraform tfstate into Ferrum encrypted format
    Import {
        /// Path to terraform.tfstate (JSON)
        tfstate: String,
        /// Output ferrum.fstate path
        #[arg(short, long, default_value = "ferrum.fstate")]
        output: String,
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    /// Refresh cloud state in parallel (stateless mode)
    Refresh {
        #[arg(short, long, default_value = "ferrum.fstate")]
        state: String,
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    /// Manage Terraform provider plugins
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// Show Ferrum version, build date, and platform
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run Docker smoke test (Hello Ferrum nginx container)
    #[command(hide = true)]
    TestDrive {
        /// Remove smoke test project and resources
        #[arg(long)]
        cleanup: bool,
    },
}

#[derive(Subcommand)]
enum ProviderCommands {
    /// Download and install a provider (aws, azurerm, google)
    Install {
        /// Provider name
        name: String,
    },
    /// List installed providers
    List,
}

fn maybe_telemetry(no_telemetry: bool, smoke_test: Option<bool>) {
    if no_telemetry || std::env::var("FERRUM_TELEMETRY_DISABLED").is_ok() {
        return;
    }
    let providers = ferrum_provider_bridge::PluginManager::new().installed_provider_names();
    ferrum_telemetry::maybe_notify_first_run(env!("CARGO_PKG_VERSION"), &providers, smoke_test);
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let version = env!("CARGO_PKG_VERSION");
    let build_date = option_env!("FERRUM_BUILD_DATE").unwrap_or("development");

    match cli.command {
        Commands::Init {
            path,
            template,
            passphrase,
        } => {
            commands::init(&path, template.as_deref(), passphrase.as_deref())?;
            if !cli.no_telemetry {
                maybe_telemetry(false, None);
            }
        }
        Commands::Doctor { json } => {
            doctor::doctor(version, json)?;
            if !cli.no_telemetry {
                maybe_telemetry(false, None);
            }
        }
        Commands::Plan { state, passphrase } => {
            run_async(commands::plan(&state, passphrase.as_deref()))?;
        }
        Commands::Apply {
            state,
            passphrase,
            auto_approve,
        } => run_async(commands::apply(&state, passphrase.as_deref(), auto_approve))?,
        Commands::Destroy {
            state,
            passphrase,
            auto_approve,
        } => run_async(commands::destroy(&state, passphrase.as_deref(), auto_approve))?,
        Commands::Import {
            tfstate,
            output,
            passphrase,
        } => commands::import_cmd(&tfstate, &output, passphrase.as_deref())?,
        Commands::Refresh { state, passphrase } => {
            run_async(commands::refresh(&state, passphrase.as_deref()))?;
        }
        Commands::Provider { command } => match command {
            ProviderCommands::Install { name } => {
                run_async(commands::provider_install(&name))?;
            }
            ProviderCommands::List => commands::provider_list()?,
        },
        Commands::Version { json } => commands::version(version, build_date, json)?,
        Commands::TestDrive { cleanup } => {
            let base = std::env::current_dir()?;
            let result = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?
                .block_on(test_drive::test_drive(cleanup, &base))?;
            test_drive::print_smoke_result(&result);
            if !result.success && !result.docker_available {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn run_async(fut: impl std::future::Future<Output = Result<()>>) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(fut)
}
