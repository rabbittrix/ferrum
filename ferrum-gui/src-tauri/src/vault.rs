use ferrum_state::State;
use serde::Serialize;
use std::path::{Path, PathBuf};

const DEFAULT_STATE: &str = "ferrum.fstate";

#[derive(Serialize)]
pub struct VaultEntryMeta {
    pub name: String,
}

#[derive(Serialize)]
pub struct VaultListResponse {
    pub secrets: Vec<VaultEntryMeta>,
    pub state_path: String,
}

#[derive(Serialize)]
pub struct VaultRevealResponse {
    pub name: String,
    pub value: String,
}

pub fn resolve_state_path(state_path: Option<String>) -> PathBuf {
    if let Some(p) = state_path {
        return PathBuf::from(p);
    }
    find_existing_state().unwrap_or_else(|| PathBuf::from(DEFAULT_STATE))
}

pub fn find_existing_state() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "ferrum.fstate",
        "../ferrum.fstate",
        "../../ferrum.fstate",
        "../../../ferrum.fstate",
    ];
    for c in CANDIDATES {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn load_state(path: &Path, passphrase: Option<&str>) -> Result<State, String> {
    if !path.exists() {
        return Err(format!(
            "state file not found: {}. Run `ferrum init` in your project root.",
            path.display()
        ));
    }
    State::load(path, passphrase).map_err(|e| e.to_string())
}

pub fn save_state(state: &mut State) -> Result<(), String> {
    state.save().map_err(|e| e.to_string())
}

pub fn vault_list_impl(
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<VaultListResponse, String> {
    let path = resolve_state_path(state_path);
    let mut state = load_state(&path, passphrase.as_deref())?;
    state.vault_ensure_defaults().map_err(|e| e.to_string())?;
    save_state(&mut state)?;

    let secrets = state
        .vault_names()
        .into_iter()
        .map(|name| VaultEntryMeta { name })
        .collect();

    Ok(VaultListResponse {
        secrets,
        state_path: path.to_string_lossy().into(),
    })
}

pub fn vault_reveal_impl(
    name: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<VaultRevealResponse, String> {
    let path = resolve_state_path(state_path);
    let state = load_state(&path, passphrase.as_deref())?;
    let value = state.vault_get(&name).map_err(|e| e.to_string())?;
    Ok(VaultRevealResponse { name, value })
}

pub fn vault_set_impl(
    name: String,
    value: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<(), String> {
    let path = resolve_state_path(state_path);
    let mut state = load_state(&path, passphrase.as_deref())?;
    state.vault_set(&name, &value).map_err(|e| e.to_string())?;
    save_state(&mut state)
}

pub fn vault_add_impl(
    name: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<(), String> {
    let path = resolve_state_path(state_path);
    let mut state = load_state(&path, passphrase.as_deref())?;
    if state.body.vault.contains_key(&name) {
        return Err(format!("secret '{name}' already exists"));
    }
    state.vault_set(&name, "").map_err(|e| e.to_string())?;
    save_state(&mut state)
}

pub fn vault_delete_impl(
    name: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<(), String> {
    let path = resolve_state_path(state_path);
    let mut state = load_state(&path, passphrase.as_deref())?;
    state.vault_remove(&name).map_err(|e| e.to_string())?;
    save_state(&mut state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn vault_roundtrip_encrypted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ferrum.fstate");
        let mut state = State::new(&path);
        state.vault_set("DB_PASSWORD", "s3cr3t!").unwrap();
        state.save().unwrap();
        state.save_key_file().unwrap();

        let loaded = State::load(&path, None).unwrap();
        assert_eq!(loaded.vault_get("DB_PASSWORD").unwrap(), "s3cr3t!");
    }
}
