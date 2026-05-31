use std::path::{Path, PathBuf};

use pi_bridge_types::AuthSource;

pub fn logo_path() -> String {
    workspace_root()
        .join("assets/pi-logo-auto.svg")
        .display()
        .to_string()
}

pub fn auth_source_label(source: Option<AuthSource>) -> &'static str {
    match source {
        Some(AuthSource::Stored) => "stored in auth.json",
        Some(AuthSource::Runtime) => "active for this run",
        Some(AuthSource::Environment) => "environment variable",
        Some(AuthSource::Fallback) => "fallback resolver",
        Some(AuthSource::ModelsJsonKey) => "models.json key",
        Some(AuthSource::ModelsJsonCommand) => "models.json command",
        None => "not configured",
    }
}

pub fn provider_env_hint(provider: &str) -> String {
    format!(
        "{} API key or existing Pi auth",
        provider.to_uppercase().replace('-', "_")
    )
}

pub fn provider_logo_path(provider: &str) -> Option<String> {
    let file = match provider {
        "anthropic" => "anthropic.svg",
        "openai" | "azure-openai-responses" => "openai.svg",
        "amazon-bedrock" => "aws.svg",
        "cerebras" => "cerebras.svg",
        "cloudflare-ai-gateway" | "cloudflare-workers-ai" => "cloudflare.svg",
        "deepseek" => "deepseek.svg",
        "fireworks" => "fireworks.svg",
        "github-copilot" => "github.svg",
        _ => return None,
    };
    let path = workspace_root().join("assets/provider-logos").join(file);
    path.exists().then(|| path.display().to_string())
}

pub fn provider_initials(display_name: &str) -> String {
    display_name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}
