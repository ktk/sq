//! Config loading and endpoint resolution.
//!
//! Resolution order for the endpoint:
//!   --endpoint/-e (name or URL) > SQ_ENDPOINT env > .sq.toml (cwd + ancestors)
//!   > ~/.config/sq/config.toml default > built-in http://localhost:7077/api

use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::prefixes;

pub const BUILTIN_ENDPOINT: &str = "http://localhost:7077/api";

#[derive(Deserialize, Default)]
pub struct FileConfig {
    pub default: Option<String>,
    #[serde(default)]
    pub endpoints: BTreeMap<String, EndpointCfg>,
    #[serde(default)]
    pub prefixes: BTreeMap<String, String>,
}

#[derive(Deserialize, Clone)]
pub struct EndpointCfg {
    pub url: String,
    #[serde(default)]
    pub token_env: Option<String>,
}

impl FileConfig {
    fn load(path: &Path) -> Option<FileConfig> {
        let s = std::fs::read_to_string(path).ok()?;
        match toml::from_str(&s) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("sq: ignoring {}: {e}", path.display());
                None
            }
        }
    }
}

/// Merged view of user + workspace config.
pub struct Merged {
    pub default: Option<String>,
    pub endpoints: BTreeMap<String, EndpointCfg>,
    pub prefixes: BTreeMap<String, String>,
}

pub fn merged() -> Merged {
    let user = dirs::config_dir()
        .map(|p| p.join("sq/config.toml"))
        .and_then(|p| FileConfig::load(&p))
        .unwrap_or_default();
    let ws = std::env::current_dir()
        .ok()
        .and_then(|d| find_up(&d, ".sq.toml"))
        .and_then(|p| FileConfig::load(&p))
        .unwrap_or_default();

    let mut endpoints = user.endpoints;
    endpoints.extend(ws.endpoints);

    let mut prefixes = prefixes::base_map();
    prefixes.extend(user.prefixes);
    prefixes.extend(ws.prefixes);

    Merged {
        default: ws.default.or(user.default),
        endpoints,
        prefixes,
    }
}

fn find_up(start: &Path, name: &str) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

pub struct Resolved {
    pub url: String,
    pub token_env: Option<String>,
    pub prefixes: BTreeMap<String, String>,
}

pub fn resolve(cli_endpoint: Option<&str>) -> Result<Resolved> {
    let m = merged();

    let (url, token_env) = if let Some(sel) = cli_endpoint {
        pick(sel, &m.endpoints)?
    } else if let Ok(env_url) = std::env::var("SQ_ENDPOINT") {
        (env_url, None)
    } else if let Some(name) = &m.default {
        pick(name, &m.endpoints)?
    } else {
        (BUILTIN_ENDPOINT.to_string(), None)
    };

    Ok(Resolved {
        url,
        token_env,
        prefixes: m.prefixes,
    })
}

/// A selector is either a raw URL (contains `://`) or a named endpoint.
fn pick(sel: &str, endpoints: &BTreeMap<String, EndpointCfg>) -> Result<(String, Option<String>)> {
    if sel.contains("://") {
        return Ok((sel.to_string(), None));
    }
    match endpoints.get(sel) {
        Some(e) => Ok((e.url.clone(), e.token_env.clone())),
        None => bail!(
            "unknown endpoint '{sel}' (not a URL and not in config). Known: {}",
            if endpoints.is_empty() {
                "<none>".to_string()
            } else {
                endpoints.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        ),
    }
}
