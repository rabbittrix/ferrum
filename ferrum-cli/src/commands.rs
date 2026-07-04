use std::io::{self, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use ferrum_core::{
    apply_with_providers, compute_destroy_plan, detect_docker, detect_rancher_endpoint,
    format_plan_colored, import_tfstate as core_import_tfstate, load_project, plan_cost_estimate,
    save_plan_graph, version_info, Engine, FerrumConfig, NodeStatus, GRAPH_FILENAME,
};
use ferrum_provider_bridge::{PluginManager, ProviderPool};
use ferrum_state::{State, STATE_FILENAME};

use crate::templates::{apply_template, TEMPLATE_NAMES};

/// Cross-shell confirmation prompt (PowerShell, Bash, CMD).
pub fn confirm_prompt(message: &str) -> Result<bool> {
    eprint!("{message}");
    io::stderr().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
}

pub fn init(path: &str, template: Option<&str>, passphrase: Option<&str>) -> Result<()> {
    let dir = Path::new(path);
    if !dir.exists() {
        std::fs::create_dir_all(dir).context("create project directory")?;
    }

    let state_path = dir.join(STATE_FILENAME);
    if state_path.exists() {
        bail!("Ferrum state already exists at {}", state_path.display());
    }

    if let Some(t) = template {
        apply_template(dir, t)?;
        println!("✓ Template '{t}' scaffolded in {}", dir.display());
    }

    let mut state = match passphrase {
        Some(p) => State::new_with_passphrase(&state_path, p),
        None => State::new(&state_path),
    };
    state.save().context("write initial state")?;
    if passphrase.is_none() {
        state.save_key_file().context("write .ferrum_key")?;
    }

    let json_path = dir.join("ferrum.json");
    if !json_path.exists() {
        let docker = detect_docker();
        let rancher = detect_rancher_endpoint();
        let config = serde_json::json!({
            "project": {
                "name": dir.file_name().and_then(|s| s.to_str()).unwrap_or("my-infrastructure"),
                "template": template,
            },
            "state": { "file": "ferrum.fstate", "encrypted": true },
            "orchestration": {
                "docker": docker,
                "rancher_url": rancher,
            },
            "lock": { "backend": "file" },
            "telemetry": { "disabled": false },
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&config)?)?;
    }

    let toml_path = dir.join("ferrum.toml");
    if !toml_path.exists() {
        std::fs::write(
            &toml_path,
            r#"# Ferrum project configuration (legacy — prefer ferrum.json)
[project]
name = "my-infrastructure"

[state]
encrypted = true
file = "ferrum.fstate"
"#,
        )?;
    }

    if detect_docker() {
        println!("  Docker detected — orchestration.docker enabled in ferrum.json");
    }
    if let Some(url) = detect_rancher_endpoint() {
        println!("  Rancher detected at {url} — set orchestration.rancher_url in ferrum.json");
    }

    if template.is_none() {
        let fe_path = dir.join("main.fe");
        if !fe_path.exists() {
            std::fs::write(
                &fe_path,
                "// Ferrum infrastructure\n// Run: ferrum init --template docker-local\n",
            )?;
        }
    }

    println!("✓ Ferrum initialized at {}", dir.display());
    println!("  State file: {} (AES-256-GCM encrypted)", state_path.display());
    if passphrase.is_none() {
        println!(
            "  State key:  set FERRUM_STATE_KEY={} for CI/CD",
            state.export_key_hex()
        );
    }
    if template.is_none() {
        println!(
            "  Tip: use --template for a starter project: {}",
            TEMPLATE_NAMES.join(", ")
        );
    }
    Ok(())
}

fn project_dir_for_state(state_path: &str) -> std::path::PathBuf {
    ferrum_core::find_project_dir(Path::new(state_path))
}

fn acquire_lock(state_path: &Path) -> Result<ferrum_core::StateLock> {
    let dir = project_dir_for_state(state_path.to_str().unwrap_or("ferrum.fstate"));
    let config = FerrumConfig::load(&dir);
    let holder = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "ferrum".into());
    config
        .lock_manager(state_path)
        .acquire(&holder)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

fn release_lock(state_path: &Path, lock: &ferrum_core::StateLock) {
    let dir = project_dir_for_state(state_path.to_str().unwrap_or("ferrum.fstate"));
    let config = FerrumConfig::load(&dir);
    let _ = config.lock_manager(state_path).release(lock);
}

pub async fn plan(state_path: &str, passphrase: Option<&str>) -> Result<()> {
    let path = Path::new(state_path);
    let lock = acquire_lock(path)?;
    let result = plan_inner(state_path, passphrase).await;
    release_lock(path, &lock);
    result
}

async fn plan_inner(state_path: &str, passphrase: Option<&str>) -> Result<()> {
    let project = load_project(&project_dir_for_state(state_path)).context("load .fe configuration")?;

    let state = State::load(state_path, passphrase).context("load state")?;
    let mut engine = Engine::new(state);
    let plan = engine.plan(&project.resources).context("compute plan")?;

    print!("{}", format_plan_colored(&plan));

    let cost = plan_cost_estimate(&plan);
    if plan.has_changes() {
        println!("\nCost Estimate: {}", cost.summary);
        println!("   Monthly delta: ${:.2}/mo", cost.monthly_delta_usd);
    } else {
        println!("\nNo changes. Infrastructure matches configuration.");
    }

    let graph_path = save_plan_graph(Path::new(state_path), &project.resources)
        .context("write infrastructure graph")?;
    println!(
        "\nGraph: {} ({} resources, {} execution steps)",
        graph_path.display(),
        project.resources.len(),
        project.execution_plan.order.len()
    );

    Ok(())
}

pub async fn apply(state_path: &str, passphrase: Option<&str>, auto_approve: bool) -> Result<()> {
    let path = Path::new(state_path);
    let lock = acquire_lock(path)?;
    let result = apply_inner(state_path, passphrase, auto_approve).await;
    release_lock(path, &lock);
    result
}

async fn apply_inner(state_path: &str, passphrase: Option<&str>, auto_approve: bool) -> Result<()> {
    let project = load_project(&project_dir_for_state(state_path)).context("load .fe configuration")?;

    let state = State::load(state_path, passphrase).context("load state")?;
    let mut engine = Engine::new(state);
    let plan = engine.plan(&project.resources).context("compute plan")?;

    if !plan.has_changes() {
        println!("No changes. Infrastructure is up-to-date.");
        return Ok(());
    }

    print!("{}", format_plan_colored(&plan));

    if !auto_approve && !confirm_prompt("\nApply these changes? [y/N] ")? {
        println!("Apply cancelled.");
        return Ok(());
    }

    let graph_path = project_dir_for_state(state_path).join(GRAPH_FILENAME);
    update_graph_status(&graph_path, &plan.execution_order, NodeStatus::Creating);

    let pool = ProviderPool::default();
    apply_with_providers(&mut engine.state, &plan, &project.resources, &pool)
        .await
        .context("apply via provider bridge")?;

    update_graph_status(&graph_path, &plan.execution_order, NodeStatus::Active);

    let graph_path = save_plan_graph(Path::new(state_path), &project.resources)
        .context("write infrastructure graph")?;
    println!("Apply complete. State saved to {}", state_path);
    println!("  Graph: {}", graph_path.display());
    Ok(())
}

pub async fn destroy(state_path: &str, passphrase: Option<&str>, auto_approve: bool) -> Result<()> {
    let path = Path::new(state_path);
    let lock = acquire_lock(path)?;
    let result = destroy_inner(state_path, passphrase, auto_approve).await;
    release_lock(path, &lock);
    result
}

async fn destroy_inner(state_path: &str, passphrase: Option<&str>, auto_approve: bool) -> Result<()> {
    let state = State::load(state_path, passphrase).context("load state")?;
    if state.resources().is_empty() {
        println!("Nothing to destroy — state is empty.");
        return Ok(());
    }

    let plan = compute_destroy_plan(&state).context("compute destroy plan")?;
    print!("{}", format_plan_colored(&plan));

    if !auto_approve && !confirm_prompt("\nDestroy ALL resources? [y/N] ")? {
        println!("Destroy cancelled.");
        return Ok(());
    }

    let mut engine = Engine::new(state);
    let empty: Vec<ferrum_state::ResourceInstance> = vec![];
    engine.apply(&plan, &empty).context("destroy apply")?;

    let graph_path = project_dir_for_state(state_path).join(GRAPH_FILENAME);
    if graph_path.exists() {
        let _ = std::fs::remove_file(&graph_path);
    }

    println!("Destroy complete. All resources removed from state.");
    Ok(())
}

fn update_graph_status(graph_path: &Path, addresses: &[String], status: NodeStatus) {
    if !graph_path.exists() {
        return;
    }
    if let Ok(mut graph) = ferrum_core::InfrastructureGraph::load(graph_path) {
        for addr in addresses {
            graph.set_node_status(addr, status.clone());
        }
        let _ = graph.save(graph_path);
    }
}

pub fn version(version: &str, build_date: &str, json: bool) -> Result<()> {
    if json {
        println!("{}", version_info(version, build_date));
    } else {
        println!("Ferrum v{version}");
        println!("Build date: {build_date}");
        println!("Platform:   {} / {}", std::env::consts::OS, std::env::consts::ARCH);
        println!("Author:     Roberto de Souza <rabbittrix@hotmail.com>");
        println!("Repository: https://github.com/rabbittrix/ferrum");
    }
    Ok(())
}

pub async fn provider_install(name: &str) -> Result<()> {
    let manager = PluginManager::new();
    println!("Installing Terraform provider '{name}'…");
    let installed = manager
        .ensure_provider(name)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
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
    let installed = manager
        .discover_installed()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
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
    println!(
        "  Graph:  {} ({} dependency edges)",
        report.graph_path.display(),
        report.edge_count
    );
    Ok(())
}

pub async fn refresh(state_path: &str, passphrase: Option<&str>) -> Result<()> {
    let path = Path::new(state_path);
    let lock = acquire_lock(path)?;
    let result = refresh_inner(state_path, passphrase).await;
    release_lock(path, &lock);
    result
}

async fn refresh_inner(state_path: &str, passphrase: Option<&str>) -> Result<()> {
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
