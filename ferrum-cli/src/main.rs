mod commands;

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
        /// Auto-approve without confirmation
        #[arg(short, long)]
        auto_approve: bool,
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
    /// Show Ferrum version and build info
    Version,
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    if !cli.no_telemetry {
        let providers = ferrum_provider_bridge::PluginManager::new().installed_provider_names();
        ferrum_telemetry::maybe_notify_install_with_providers(
            env!("CARGO_PKG_VERSION"),
            &providers,
        );
    }

    match cli.command {
        Commands::Init { path, passphrase } => commands::init(&path, passphrase.as_deref())?,
        Commands::Plan { state, passphrase } => commands::plan(&state, passphrase.as_deref()).await?,
        Commands::Apply {
            state,
            passphrase,
            auto_approve,
        } => commands::apply(&state, passphrase.as_deref(), auto_approve).await?,
        Commands::Import {
            tfstate,
            output,
            passphrase,
        } => commands::import_cmd(&tfstate, &output, passphrase.as_deref())?,
        Commands::Refresh { state, passphrase } => {
            commands::refresh(&state, passphrase.as_deref()).await?
        }
        Commands::Provider { command } => match command {
            ProviderCommands::Install { name } => commands::provider_install(&name).await?,
            ProviderCommands::List => commands::provider_list()?,
        },
        Commands::Version => {
            println!("Ferrum v{}", env!("CARGO_PKG_VERSION"));
            println!("Author: Roberto de Souza <rabbittrix@hotmail.com>");
            println!("https://github.com/rabbittrix/ferrum");
        }
    }

    Ok(())
}
