use std::path::Path;

use anyhow::{bail, Context, Result};
use ferrum_core::{
    apply_with_providers, format_plan_colored, import_tfstate as core_import_tfstate, load_project,
    plan_cost_estimate, save_plan_graph, Engine,
};
use ferrum_provider_bridge::{PluginManager, ProviderPool};
use ferrum_state::{State, STATE_FILENAME};

pub fn init(path: &str, passphrase: Option<&str>) -> Result<()> {
    let dir = Path::new(path);
    if !dir.exists() {
        std::fs::create_dir_all(dir).context("create project directory")?;
    }

    let state_path = dir.join(STATE_FILENAME);
    if state_path.exists() {
        bail!("Ferrum state already exists at {}", state_path.display());
    }

    let mut state = match passphrase {
        Some(p) => State::new_with_passphrase(&state_path, p),
        None => State::new(&state_path),
    };
    state.save().context("write initial state")?;
    if passphrase.is_none() {
        state.save_key_file().context("write .ferrum_key")?;
    }

    let config_path = dir.join("ferrum.toml");
    if !config_path.exists() {
        let config = r#"# Ferrum project configuration
# Author: Roberto de Souza

[project]
name = "my-infrastructure"
version = "0.1.0"

[provider.aws]
region = "us-east-1"

[state]
encrypted = true
file = "ferrum.fstate"

[telemetry]
# Set to true to disable anonymous install notification
disabled = false
"#;
        std::fs::write(&config_path, config)?;
    }

    println!("✓ Ferrum initialized at {}", dir.display());
    println!("  State file: {} (AES-256-GCM encrypted)", state_path.display());
    if passphrase.is_none() {
        println!("  State key:  set FERRUM_STATE_KEY={} for CI/CD", state.export_key_hex());
    }
    Ok(())
}

fn project_dir_for_state(state_path: &str) -> std::path::PathBuf {
    ferrum_core::find_project_dir(Path::new(state_path))
}

pub async fn plan(state_path: &str, passphrase: Option<&str>) -> Result<()> {
    let project = load_project(&project_dir_for_state(state_path)).context("load .fe configuration")?;

    let state = State::load(state_path, passphrase).context("load state")?;
    let mut engine = Engine::new(state);
    let plan = engine
        .plan(&project.resources)
        .context("compute plan")?;

    print!("{}", format_plan_colored(&plan));

    let cost = plan_cost_estimate(&plan);
    if plan.has_changes() {
        println!("\n💰 Cost Estimate: {}", cost.summary);
        println!("   Monthly delta: ${:.2}/mo", cost.monthly_delta_usd);
    } else {
        println!("\nNo changes. Infrastructure matches configuration.");
    }

    let graph_path = save_plan_graph(Path::new(state_path), &project.resources)
        .context("write infrastructure graph")?;
    println!(
        "\n📊 Graph: {} ({} resources, {} execution steps)",
        graph_path.display(),
        project.resources.len(),
        project.execution_plan.order.len()
    );

    Ok(())
}

pub async fn apply(state_path: &str, passphrase: Option<&str>, auto_approve: bool) -> Result<()> {
    let project = load_project(&project_dir_for_state(state_path)).context("load .fe configuration")?;

    let state = State::load(state_path, passphrase).context("load state")?;
    let mut engine = Engine::new(state);
    let plan = engine
        .plan(&project.resources)
        .context("compute plan")?;

    if !plan.has_changes() {
        println!("No changes. Infrastructure is up-to-date.");
        return Ok(());
    }

    print!("{}", format_plan_colored(&plan));

    if !auto_approve {
        println!("\nApply these changes? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Apply cancelled.");
            return Ok(());
        }
    }

    let pool = ProviderPool::default();
    apply_with_providers(&mut engine.state, &plan, &project.resources, &pool)
        .await
        .context("apply via provider bridge")?;
    let graph_path = save_plan_graph(Path::new(state_path), &project.resources)
        .context("write infrastructure graph")?;
    println!("✓ Apply complete. State saved to {}", state_path);
    println!("  Graph: {}", graph_path.display());
    Ok(())
}

pub async fn provider_install(name: &str) -> Result<()> {
    let manager = PluginManager::new();
    println!("Installing Terraform provider '{name}'…");
    let installed = manager.ensure_provider(name).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "✓ Installed {} v{} at {}",
        installed.spec.display_name,
        installed.version,
        installed.binary_path.display()
    );
    println!("  SHA256 manifest: {}.sha256", installed.binary_path.display());
    Ok(())
}

pub fn provider_list() -> Result<()> {
    let manager = PluginManager::new();
    let installed = manager.discover_installed().map_err(|e| anyhow::anyhow!("{e}"))?;
    if installed.is_empty() {
        println!("No providers installed. Run: ferrum provider install aws");
        return Ok(());
    }
    println!("Installed providers:");
    for p in installed {
        println!(
            "  {} ({}) v{} — {}",
            p.spec.display_name,
            p.spec.name,
            p.version,
            p.binary_path.display()
        );
    }
    Ok(())
}

pub fn import_cmd(tfstate: &str, output: &str, passphrase: Option<&str>) -> Result<()> {
    let tf_path = Path::new(tfstate);
    if !tf_path.exists() {
        bail!("tfstate file not found: {}", tfstate);
    }

    let mut state = match passphrase {
        Some(p) => State::new_with_passphrase(output, p),
        None => State::new(output),
    };

    let report = core_import_tfstate(tf_path, &mut state).context("import tfstate")?;
    state.save().context("save encrypted state")?;

    println!("✓ Imported {} resources from Terraform state", report.imported);
    if report.skipped > 0 {
        println!("  Skipped {} resources (duplicates or unsupported)", report.skipped);
    }
    println!("  Terraform state version: {}", report.tf_version);
    println!("  Output: {} (AES-256-GCM encrypted)", output);
    println!("  Graph:  {} ({} dependency edges)", report.graph_path.display(), report.edge_count);
    println!("  Open Dashboard to visualize infrastructure graph");
    Ok(())
}

pub async fn refresh(state_path: &str, passphrase: Option<&str>) -> Result<()> {
    let state = State::load(state_path, passphrase).context("load state")?;
    let mut engine = Engine::new(state);
    let report = engine.refresh().await.context("refresh resources")?;

    println!(
        "✓ Refreshed {} resources ({} failed)",
        report.refreshed, report.failed
    );
    for err in &report.errors {
        eprintln!("  ✗ {}", err);
    }
    Ok(())
}
